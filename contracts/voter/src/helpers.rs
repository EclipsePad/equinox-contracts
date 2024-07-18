use astroport::{asset::AssetInfo, router::SwapOperation};
use cosmwasm_std::{Decimal, Deps, StdResult, Storage};

use equinox_msg::voter::{
    state::{ADDRESS_CONFIG, EPOCH_COUNTER, IS_LOCKED, ROUTE_CONFIG},
    types::{RouteItem, WeightAllocationItem},
};

use crate::error::ContractError;

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