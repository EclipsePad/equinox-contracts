use astroport::{asset::AssetInfo, router::SwapOperation};
use cosmwasm_std::{Addr, Decimal, Deps, StdError, StdResult, Storage, Uint128};

use eclipse_base::converters::{str_to_dec, u128_to_dec};
use equinox_msg::voter::{
    msg::UserType,
    state::{
        ADDRESS_CONFIG, DAO_ESSENCE_ACC, DAO_WEIGHTS_ACC, DELEGATOR_ADDRESSES,
        ELECTOR_ADDITIONAL_ESSENCE_FRACTION, ELECTOR_ESSENCE_ACC, ELECTOR_WEIGHTS,
        ELECTOR_WEIGHTS_ACC, ELECTOR_WEIGHTS_REF, EPOCH_COUNTER, IS_LOCKED, ROUTE_CONFIG,
        SLACKER_ESSENCE_ACC, USER_ESSENCE,
    },
    types::{EssenceAllocationItem, RouteItem, WeightAllocationItem},
};

use crate::{
    error::ContractError,
    math::{calc_scaled_essence_allocation, calc_updated_essence_allocation},
};

pub fn verify_weight_allocation(
    deps: Deps,
    weight_allocation: &Vec<WeightAllocationItem>,
) -> Result<(), ContractError> {
    // check weights:
    // 1) empty
    if weight_allocation.is_empty() {
        Err(ContractError::EmptyVotingList)?;
    }

    // 2) diplications
    let mut pool_list: Vec<String> = weight_allocation
        .iter()
        .map(|x| x.lp_token.to_string())
        .collect();
    pool_list.sort_unstable();
    pool_list.dedup();

    if pool_list.len() != weight_allocation.len() {
        Err(ContractError::VotingListDuplication)?;
    }

    // 3) out of range
    if weight_allocation
        .iter()
        .any(|x| x.weight.is_zero() || x.weight > Decimal::one())
    {
        Err(ContractError::WeightIsOutOfRange)?;
    }

    // 4) wrong sum
    if (weight_allocation
        .iter()
        .fold(Decimal::zero(), |acc, cur| acc + cur.weight))
        != Decimal::one()
    {
        Err(ContractError::WeightsAreUnbalanced)?;
    }

    // 5) whitelist
    let whitelisted_pools: Vec<String> = deps.querier.query_wasm_smart(
        ADDRESS_CONFIG
            .load(deps.storage)?
            .astroport_emission_controller,
        &astroport_governance::emissions_controller::hub::QueryMsg::QueryWhitelist {},
    )?;

    if weight_allocation
        .iter()
        .any(|x| !whitelisted_pools.contains(&x.lp_token))
    {
        Err(ContractError::PoolIsNotWhitelisted)?;
    }

    Ok(())
}

// reset is_locked on user actions on epoch start
pub fn try_unlock_and_check(
    storage: &mut dyn Storage,
    block_time: u64,
) -> Result<(), ContractError> {
    let is_locked = try_unlock(storage, block_time)?;

    if is_locked {
        Err(ContractError::EpochEnd)?;
    }

    Ok(())
}

// reset is_locked on eclipsepad-staking actions on epoch start
pub fn try_unlock(storage: &mut dyn Storage, block_time: u64) -> Result<bool, ContractError> {
    let mut is_locked = IS_LOCKED.load(storage)?;

    if is_locked && block_time >= EPOCH_COUNTER.load(storage)?.start_date {
        is_locked = false;
        IS_LOCKED.save(storage, &is_locked)?;
    }

    Ok(is_locked)
}

pub fn get_route(storage: &dyn Storage, denom: &str) -> StdResult<Vec<SwapOperation>> {
    Ok(ROUTE_CONFIG
        .load(storage, denom)?
        .iter()
        .map(
            |RouteItem {
                 denom_in,
                 denom_out,
             }| SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: denom_in.to_string(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: denom_out.to_string(),
                },
            },
        )
        .collect())
}

pub fn get_user_type(storage: &dyn Storage, address: &Addr) -> StdResult<UserType> {
    // check if user exists
    if !USER_ESSENCE.has(storage, address) {
        Err(StdError::generic_err(
            ContractError::UserIsNotFound.to_string(),
        ))?;
    }

    // elector is a user who placed a vote in current epoch
    if ELECTOR_WEIGHTS.has(storage, address) {
        return Ok(UserType::Elector);
    }

    // delegator is a user who delegated
    if DELEGATOR_ADDRESSES.has(storage, address) {
        return Ok(UserType::Delegator);
    }

    // slacker is a user who met one of following requirements:
    // a) was an elector earlier (ELECTOR_WEIGHTS_REF) but didn't place a vote in current epoch
    // b) has essence but didn't place a vote at all
    Ok(UserType::Slacker)
}

pub fn get_user_weights(
    storage: &dyn Storage,
    address: &Addr,
) -> StdResult<Vec<WeightAllocationItem>> {
    Ok(match get_user_type(storage, address)? {
        UserType::Elector => ELECTOR_WEIGHTS.load(storage, address)?,
        UserType::Delegator => DAO_WEIGHTS_ACC.load(storage).unwrap_or_default(),
        UserType::Slacker => ELECTOR_WEIGHTS_REF
            .load(storage, address)
            .unwrap_or_default(),
    })
}

/// returns (total_essence_allocation, total_weights_allocation)
pub fn get_total_votes(
    storage: &dyn Storage,
    block_time: u64,
) -> StdResult<(Vec<EssenceAllocationItem>, Vec<(String, Decimal)>)> {
    // get slackers essence
    let slacker_essence = SLACKER_ESSENCE_ACC.load(storage)?;
    let elector_additional_essence_fraction = str_to_dec(ELECTOR_ADDITIONAL_ESSENCE_FRACTION);
    // 80 % of slackers essence goes to electors
    let elector_essence_acc_before = ELECTOR_ESSENCE_ACC.load(storage)?;
    let elector_weights_acc_before = ELECTOR_WEIGHTS_ACC.load(storage)?;
    let elector_essence_allocation_acc_after = calc_scaled_essence_allocation(
        &elector_essence_acc_before,
        &elector_weights_acc_before,
        &slacker_essence,
        elector_additional_essence_fraction,
    );
    // 20 % of slackers essence goes to dao
    let dao_essence_acc_before = DAO_ESSENCE_ACC.load(storage)?;
    let dao_weights_acc_before = DAO_WEIGHTS_ACC.load(storage)?;
    let dao_essence_allocation_acc_after = calc_scaled_essence_allocation(
        &dao_essence_acc_before,
        &dao_weights_acc_before,
        &slacker_essence,
        Decimal::one() - elector_additional_essence_fraction,
    );
    // final votes
    let full_elector_essence = elector_essence_allocation_acc_after
        .iter()
        .fold(Uint128::zero(), |acc, cur| {
            acc + cur.essence_info.capture(block_time)
        });
    let full_dao_essence = dao_essence_allocation_acc_after
        .iter()
        .fold(Uint128::zero(), |acc, cur| {
            acc + cur.essence_info.capture(block_time)
        });
    let total_essence_decimal = u128_to_dec(full_elector_essence + full_dao_essence);

    let total_essence_allocation = calc_updated_essence_allocation(
        &elector_essence_allocation_acc_after,
        &dao_essence_allocation_acc_after,
        &vec![],
    );
    let total_weights_allocation: Vec<(String, Decimal)> = total_essence_allocation
        .iter()
        .map(|x| {
            (
                x.lp_token.to_string(),
                u128_to_dec(x.essence_info.capture(block_time)) / total_essence_decimal,
            )
        })
        .collect();

    Ok((total_essence_allocation, total_weights_allocation))
}
