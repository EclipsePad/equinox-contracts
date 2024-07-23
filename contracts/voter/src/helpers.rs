use astroport::{asset::AssetInfo, router::SwapOperation};
use cosmwasm_std::{Addr, Decimal, Deps, StdError, StdResult, Storage, Uint128};

use eclipse_base::converters::{str_to_dec, u128_to_dec};
use equinox_msg::voter::{
    msg::UserType,
    state::{
        ADDRESS_CONFIG, DAO_ESSENCE_ACC, DAO_WEIGHTS_ACC, DELEGATOR_ADDRESSES,
        ELECTOR_ADDITIONAL_ESSENCE_FRACTION, ELECTOR_ESSENCE_ACC, ELECTOR_WEIGHTS,
        ELECTOR_WEIGHTS_ACC, ELECTOR_WEIGHTS_REF, EPOCH_COUNTER, IS_LOCKED, REWARDS_CLAIM_STAGE,
        ROUTE_CONFIG, SLACKER_ESSENCE_ACC, TOKEN_CONFIG, USER_ESSENCE, USER_REWARDS, VOTE_RESULTS,
    },
    types::{
        EssenceAllocationItem, RewardsClaimStage, RewardsInfo, RouteItem, TokenConfig,
        WeightAllocationItem,
    },
};

use crate::{
    error::ContractError,
    math::{
        calc_delegator_rewards, calc_personal_elector_rewards, calc_scaled_essence_allocation,
        calc_updated_essence_allocation,
    },
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

/// returns (is_updated, user_rewards)
pub fn get_accumulated_rewards(
    storage: &dyn Storage,
    user: &Addr,
    block_time: u64,
) -> StdResult<(bool, RewardsInfo)> {
    let epoch = EPOCH_COUNTER.load(storage)?;
    let rewards_claim_stage = REWARDS_CLAIM_STAGE.load(storage)?;
    let mut user_rewards = USER_REWARDS.load(storage, user).unwrap_or_default();

    // it's only possible to claim user rewards when voter rewards are claimed and swapped
    // skip if rewards are claimed in previous epoch
    if !matches!(rewards_claim_stage, RewardsClaimStage::Swapped)
        || (user_rewards.last_update_epoch + 1 == epoch.id)
    {
        return Ok((false, user_rewards));
    }

    let user_essence = USER_ESSENCE.load(storage, user)?;
    let TokenConfig { eclip, .. } = TOKEN_CONFIG.load(storage)?;
    let vote_results = VOTE_RESULTS.load(storage)?;

    // collect rewards
    match get_user_type(storage, user)? {
        // For delegators previous epoch rewards will be accumulated on UpdateEssenceAllocation, ClaimRewards, Undelegate.
        // Alternatively essence will be the same in each epoch then it's possible
        // to iterate over range last_update_epoch..current_epoch_id and accumulate rewards
        // for multiple epochs
        UserType::Delegator => {
            let delegator_unclaimed_rewards =
                vote_results.iter().fold(Uint128::zero(), |acc, cur| {
                    if cur.epoch_id <= user_rewards.last_update_epoch {
                        acc
                    } else {
                        let delegator_rewards = calc_delegator_rewards(
                            cur.dao_delegators_eclip_rewards,
                            cur.slacker_essence,
                            cur.dao_essence,
                            user_essence.capture(cur.end_date),
                        );

                        acc + delegator_rewards
                    }
                });

            if user_rewards
                .value
                .iter()
                .all(|(_rewards_amount, rewards_denom)| rewards_denom != &eclip)
            {
                user_rewards.value.push((Uint128::zero(), eclip.clone()));
            }

            user_rewards.last_update_epoch = epoch.id - 1;
            user_rewards.value = user_rewards
                .value
                .into_iter()
                .map(|(amount, denom)| {
                    if denom != eclip {
                        (amount, denom)
                    } else {
                        (amount + delegator_unclaimed_rewards, denom)
                    }
                })
                .collect();

            return Ok((true, user_rewards));
        }
        // For electors previous epoch rewards will be accumulated on UpdateEssenceAllocation,
        // ClaimRewards, PlaceVote, Delegate. If last_update_epoch + 1 != current_epoch_id
        // it means the user is a slacker now and doesn't have other epoch rewards
        _ => {
            if let Some(target_result) = vote_results
                .iter()
                .find(|x| x.epoch_id == user_rewards.last_update_epoch + 1)
            {
                let personal_elector_weight_list =
                    ELECTOR_WEIGHTS_REF.load(storage, user).unwrap_or_default();
                let elector_rewards = calc_personal_elector_rewards(
                    &target_result.pool_info_list,
                    &target_result.elector_weights,
                    &personal_elector_weight_list,
                    target_result.slacker_essence,
                    target_result.elector_essence,
                    user_essence.capture(block_time),
                );

                for (_amount, denom) in &elector_rewards {
                    if user_rewards
                        .value
                        .iter()
                        .all(|(_rewards_amount, rewards_denom)| rewards_denom != denom)
                    {
                        user_rewards.value.push((Uint128::zero(), denom.to_owned()));
                    }
                }

                user_rewards.last_update_epoch = epoch.id - 1;
                user_rewards.value = user_rewards
                    .value
                    .into_iter()
                    .map(|(rewards_amount, rewards_denom)| {
                        let (additional_amount, _) = elector_rewards
                            .iter()
                            .cloned()
                            .find(|(_amount, denom)| denom == &rewards_denom)
                            .unwrap_or((Uint128::zero(), String::default()));

                        (rewards_amount + additional_amount, rewards_denom)
                    })
                    .collect();

                return Ok((true, user_rewards));
            }
        }
    };

    Ok((false, user_rewards))
}
