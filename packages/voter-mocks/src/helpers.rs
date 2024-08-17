use astroport::{asset::AssetInfo, router::SwapOperation};
use cosmwasm_std::{Addr, Decimal, Deps, StdError, StdResult, Storage, Uint128};

use eclipse_base::{
    converters::{str_to_dec, u128_to_dec},
    utils::unwrap_field,
};
use equinox_msg::voter::types::{
    AddressConfig, BribesAllocationItem, EssenceInfo, RewardsClaimStage, RewardsInfo, RouteItem,
    TokenConfig, TotalEssenceAndWeightAllocation, UserType, WeightAllocationItem,
};

use crate::{
    error::ContractError,
    math::{
        calc_delegator_rewards, calc_merged_rewards, calc_personal_elector_rewards,
        calc_scaled_essence_allocation, calc_splitted_user_essence_info,
        calc_updated_essence_allocation,
    },
    state::{
        ADDRESS_CONFIG, DAO_ESSENCE_ACC, DAO_WEIGHTS_ACC, DELEGATOR_ESSENCE_FRACTIONS,
        ELECTOR_ADDITIONAL_ESSENCE_FRACTION, ELECTOR_ESSENCE_ACC, ELECTOR_WEIGHTS,
        ELECTOR_WEIGHTS_ACC, ELECTOR_WEIGHTS_REF, EPOCH_COUNTER, IS_PAUSED, REWARDS_CLAIM_STAGE,
        ROUTE_CONFIG, SLACKER_ESSENCE_ACC, TOKEN_CONFIG, USER_ESSENCE, USER_REWARDS, VOTE_RESULTS,
    },
};

pub fn verify_weight_allocation(
    _deps: Deps,
    weight_allocation: &[WeightAllocationItem],
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

    // // 5) whitelist
    // let whitelisted_pools: Vec<String> = deps.querier.query_wasm_smart(
    //     ADDRESS_CONFIG
    //         .load(deps.storage)?
    //         .astroport_emission_controller,
    //     &astroport_governance::emissions_controller::hub::QueryMsg::QueryWhitelist {},
    // )?;

    // if weight_allocation
    //     .iter()
    //     .any(|x| !whitelisted_pools.contains(&x.lp_token))
    // {
    //     Err(ContractError::PoolIsNotWhitelisted)?;
    // }

    Ok(())
}

/// user actions are disabled when the contract is paused
pub fn check_pause_state(storage: &dyn Storage) -> Result<(), ContractError> {
    if IS_PAUSED.load(storage)? {
        Err(ContractError::ContractIsPaused)?;
    }

    Ok(())
}

/// user essence allocation updates are disallowed until completing bribes collection
pub fn check_rewards_claim_stage(storage: &dyn Storage) -> Result<(), ContractError> {
    if !matches!(
        REWARDS_CLAIM_STAGE.load(storage)?,
        RewardsClaimStage::Swapped
    ) {
        Err(ContractError::AwaitSwappedStage)?;
    }

    Ok(())
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

/// possible options:
/// 1) elector
/// 2) delegator
/// 3) slacker
/// 4) elector, delegator
/// 5) slacker, delegator
pub fn get_user_types(storage: &dyn Storage, address: &Addr) -> StdResult<Vec<UserType>> {
    // check if user exists
    if !USER_ESSENCE.has(storage, address) {
        Err(StdError::generic_err(
            ContractError::UserIsNotFound.to_string(),
        ))?;
    }

    let mut user_types: Vec<UserType> = if ELECTOR_WEIGHTS.has(storage, address) {
        // elector is a user who placed a vote in current epoch
        vec![UserType::Elector]
    } else {
        // slacker is a user who met one of following requirements:
        // a) was an elector earlier (ELECTOR_WEIGHTS_REF) but didn't place a vote in current epoch
        // b) has essence but didn't place a vote at all
        vec![UserType::Slacker]
    };

    // delegator is a user who delegated
    let delegator_essence_fraction = DELEGATOR_ESSENCE_FRACTIONS
        .load(storage, address)
        .unwrap_or_default();

    if delegator_essence_fraction == Decimal::one() {
        return Ok(vec![UserType::Delegator]);
    }

    if !delegator_essence_fraction.is_zero() {
        user_types.push(UserType::Delegator);
    }

    Ok(user_types)
}

/// returns (delegator_essence_info, elector_or_slacker_essence_info)
pub fn split_user_essence_info(
    storage: &dyn Storage,
    address: &Addr,
) -> (EssenceInfo, EssenceInfo) {
    let essence_info = USER_ESSENCE.load(storage, address).unwrap_or_default();
    let delegator_essence_fraction = DELEGATOR_ESSENCE_FRACTIONS
        .load(storage, address)
        .unwrap_or_default();

    calc_splitted_user_essence_info(&essence_info, delegator_essence_fraction)
}

pub fn get_user_weights(
    storage: &dyn Storage,
    address: &Addr,
    user_type: &UserType,
) -> Vec<WeightAllocationItem> {
    match user_type {
        UserType::Elector => ELECTOR_WEIGHTS.load(storage, address).unwrap_or_default(),
        UserType::Delegator => DAO_WEIGHTS_ACC.load(storage).unwrap_or_default(),
        UserType::Slacker => ELECTOR_WEIGHTS_REF
            .load(storage, address)
            .unwrap_or_default(),
    }
}

/// returns (total_essence_allocation, total_weights_allocation)
pub fn get_total_votes(
    storage: &dyn Storage,
    block_time: u64,
) -> StdResult<TotalEssenceAndWeightAllocation> {
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
        &[],
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

    Ok(TotalEssenceAndWeightAllocation {
        essence: total_essence_allocation,
        weight: total_weights_allocation,
    })
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
    let mut is_updated = false;

    // it's only possible to claim user rewards when voter rewards are claimed and swapped
    // skip if rewards are claimed in previous epoch
    if !matches!(rewards_claim_stage, RewardsClaimStage::Swapped)
        || (user_rewards.last_update_epoch + 1 == epoch.id)
    {
        return Ok((is_updated, user_rewards));
    }

    let (delegator_essence_info, elector_or_slacker_essence_info) =
        split_user_essence_info(storage, user);
    let TokenConfig { eclip, .. } = TOKEN_CONFIG.load(storage)?;
    let vote_results = VOTE_RESULTS.load(storage)?;

    for user_type in get_user_types(storage, user)? {
        // collect rewards
        match user_type {
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
                                delegator_essence_info.capture(cur.end_date),
                            );

                            acc + delegator_rewards
                        }
                    });

                is_updated = true;
                user_rewards.value = calc_merged_rewards(
                    &user_rewards.value,
                    &[(delegator_unclaimed_rewards, eclip.clone())],
                );
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
                        elector_or_slacker_essence_info.capture(block_time),
                    );

                    is_updated = true;
                    user_rewards.value = calc_merged_rewards(&user_rewards.value, &elector_rewards);
                }
            }
        };
    }

    if is_updated {
        user_rewards.last_update_epoch = epoch.id - 1;
    }

    Ok((is_updated, user_rewards))
}

/// returns (astro_supply, xastro_supply)
pub fn get_astro_and_xastro_supply(deps: Deps) -> StdResult<(Uint128, Uint128)> {
    let AddressConfig {
        astroport_staking, ..
    } = ADDRESS_CONFIG.load(deps.storage)?;

    let astro_supply = deps
        .querier
        .query_wasm_smart::<Uint128>(
            astroport_staking.to_string(),
            &astroport::staking::QueryMsg::TotalDeposit {},
        )
        .unwrap_or_default();

    let xastro_supply = deps
        .querier
        .query_wasm_smart::<Uint128>(
            astroport_staking.to_string(),
            &astroport::staking::QueryMsg::TotalShares {},
        )
        .unwrap_or_default();

    Ok((astro_supply, xastro_supply))
}

pub fn query_astroport_rewards(deps: Deps, sender: &Addr) -> StdResult<Vec<(Uint128, String)>> {
    let astroport_tribute_market = &unwrap_field(
        ADDRESS_CONFIG.load(deps.storage)?.astroport_tribute_market,
        "astroport_tribute_market",
    )?;

    Ok(deps
        .querier
        .query_wasm_smart::<Vec<(Uint128, String)>>(
            astroport_tribute_market,
            &tribute_market_mocks::msg::QueryMsg::Rewards {
                user: sender.to_string(),
            },
        )
        .unwrap_or_default())
}

pub fn query_eclipsepad_rewards(deps: Deps, sender: &Addr) -> StdResult<Vec<(Uint128, String)>> {
    if let Some(eclipsepad_tribute_market) =
        &ADDRESS_CONFIG.load(deps.storage)?.eclipsepad_tribute_market
    {
        return Ok(deps
            .querier
            .query_wasm_smart::<Vec<(Uint128, String)>>(
                eclipsepad_tribute_market,
                &tribute_market_mocks::msg::QueryMsg::Rewards {
                    user: sender.to_string(),
                },
            )
            .unwrap_or_default());
    }

    Ok(vec![])
}

pub fn query_astroport_bribe_allocation(deps: Deps) -> StdResult<Vec<BribesAllocationItem>> {
    let astroport_tribute_market = &unwrap_field(
        ADDRESS_CONFIG.load(deps.storage)?.astroport_tribute_market,
        "astroport_tribute_market",
    )?;

    Ok(deps
        .querier
        .query_wasm_smart::<Vec<BribesAllocationItem>>(
            astroport_tribute_market,
            &tribute_market_mocks::msg::QueryMsg::BribesAllocation {},
        )
        .unwrap_or_default())
}

pub fn query_eclipsepad_bribe_allocation(deps: Deps) -> StdResult<Vec<BribesAllocationItem>> {
    if let Some(eclipsepad_tribute_market) =
        &ADDRESS_CONFIG.load(deps.storage)?.eclipsepad_tribute_market
    {
        return Ok(deps
            .querier
            .query_wasm_smart::<Vec<BribesAllocationItem>>(
                eclipsepad_tribute_market,
                &tribute_market_mocks::msg::QueryMsg::BribesAllocation {},
            )
            .unwrap_or_default());
    }

    Ok(vec![])
}
