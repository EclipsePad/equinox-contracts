use cosmwasm_std::{Addr, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

use eclipse_base::{
    error::ContractError,
    staking::{
        msg::{
            EssenceAndRewardsInfo, QueryAprInfoResponse, QueryBalancesResponse,
            QueryEssenceListResponseItem, QueryEssenceResponse, QueryRewardsReductionInfoResponse,
            StakerInfoResponse, StateResponse, UsersAmountResponse,
        },
        state::{
            BECLIP_SUPPLY, BONDED_VAULT_CREATION_DATE, CONFIG, DECREASING_REWARDS_DATE, IS_PAUSED,
            LOCKER_INFO, LOCKING_ESSENCE, LOCK_STATES, PAGINATION_CONFIG, STAKER_INFO, STAKE_STATE,
            STAKING_ESSENCE_COMPONENTS, TOTAL_LOCKING_ESSENCE, TOTAL_STAKING_ESSENCE_COMPONENTS,
            YEAR_IN_SECONDS,
        },
        types::{
            AprInfoItem, Config, LockerInfo, LockingAprItem, PaginationConfig, StakerInfo, Vault,
        },
    },
    voter::types::EssenceInfo,
};

use crate::{helpers, math};

pub fn query_config(deps: Deps, _env: Env) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

pub fn query_pause_state(deps: Deps, _env: Env) -> StdResult<bool> {
    IS_PAUSED.load(deps.storage)
}

pub fn query_bonded_vault_creation_date(deps: Deps, _env: Env, user: String) -> StdResult<u64> {
    Ok(BONDED_VAULT_CREATION_DATE
        .load(deps.storage, &deps.api.addr_validate(&user)?)
        .unwrap_or_default())
}

pub fn query_beclip_supply(deps: Deps, _env: Env) -> StdResult<Uint128> {
    Ok(BECLIP_SUPPLY.load(deps.storage).unwrap_or_default())
}

pub fn query_pagination_config(deps: Deps, _env: Env) -> StdResult<PaginationConfig> {
    PAGINATION_CONFIG.load(deps.storage)
}

pub fn query_state(deps: Deps, _env: Env) -> StdResult<StateResponse> {
    Ok(StateResponse {
        stake_state: STAKE_STATE.load(deps.storage)?,
        lock_states: LOCK_STATES.load(deps.storage)?,
    })
}

pub fn query_aggregated_vault(
    deps: Deps,
    env: Env,
    user: String,
    tier: Option<u64>,
    vault_creation_date_list: Vec<u64>,
) -> StdResult<Vault> {
    let block_time = env.block.time.seconds();
    let user = deps.api.addr_validate(&user)?;
    let Config {
        lock_schedule,
        seconds_per_essence,
        eclip_per_second,
        eclip_per_second_multiplier,
        ..
    } = CONFIG.load(deps.storage)?;

    // get eclip_per_second values
    let (decreasing_rewards_date, eclip_per_second_before, eclip_per_second_after) =
        helpers::split_eclip_per_second(
            deps.storage,
            eclip_per_second_multiplier,
            eclip_per_second,
            block_time,
        )?;

    let total_essence = helpers::get_total_essence(deps.storage, block_time, seconds_per_essence)?;

    let aggregated_vault = match tier {
        Some(lock_tier) => {
            let max_lock_tier = CONFIG.load(deps.storage)?.lock_schedule.len() as u64;

            if lock_tier >= max_lock_tier {
                Err(ContractError::LockTierIsOutOfRange)?;
            }

            let (locking_period, _global_rewards_per_tier) = lock_schedule[lock_tier as usize];
            let locker_infos = LOCKER_INFO.load(deps.storage, &user)?;

            let vaults_target: Vec<Vault> = locker_infos
                .into_iter()
                .find(|x| x.lock_tier == lock_tier)
                .ok_or(ContractError::Unauthorized)?
                .vaults
                .into_iter()
                .filter(|vault| vault_creation_date_list.contains(&vault.creation_date))
                .collect();

            math::calc_aggregated_vault(
                &vaults_target,
                locking_period,
                decreasing_rewards_date,
                block_time,
                seconds_per_essence,
                eclip_per_second_before,
                eclip_per_second_after,
                total_essence,
            )
        }
        None => {
            let staker_info = STAKER_INFO.load(deps.storage, &user)?;

            let vaults_target: Vec<Vault> = staker_info
                .vaults
                .into_iter()
                .filter(|vault| vault_creation_date_list.contains(&vault.creation_date))
                .collect();

            math::calc_aggregated_vault(
                &vaults_target,
                0,
                decreasing_rewards_date,
                block_time,
                seconds_per_essence,
                eclip_per_second_before,
                eclip_per_second_after,
                total_essence,
            )
        }
    };

    Ok(aggregated_vault)
}

pub fn query_balances(deps: Deps, _env: Env) -> StdResult<QueryBalancesResponse> {
    let rewards_pool = Uint128::zero(); // disabled

    let unclaimed_staking = STAKER_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .fold(Uint128::zero(), |staker_acc, cur| {
            let (_address, staker_info) = cur.unwrap();

            staker_acc
                + staker_info
                    .vaults
                    .iter()
                    .fold(Uint128::zero(), |acc, vault| {
                        acc + vault.accumulated_rewards
                    })
        });

    let unclaimed_locking = LOCKER_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .fold(Uint128::zero(), |lockers_acc, cur| {
            let (_address, locker_infos) = cur.unwrap();

            lockers_acc
                + locker_infos.iter().fold(
                    Uint128::zero(),
                    |vaults_acc, LockerInfo { vaults, .. }| {
                        vaults_acc
                            + vaults.iter().fold(Uint128::zero(), |acc, vault| {
                                acc + vault.accumulated_rewards
                            })
                    },
                )
        });

    Ok(QueryBalancesResponse {
        rewards_pool,
        unclaimed: unclaimed_staking + unclaimed_locking,
    })
}

pub fn query_staker_info(deps: Deps, env: Env, staker: String) -> StdResult<StakerInfoResponse> {
    let block_time = env.block.time.seconds();
    let staker = deps.api.addr_validate(&staker)?;
    let Config {
        lock_schedule,
        seconds_per_essence,
        penalty_multiplier,
        eclip_per_second,
        eclip_per_second_multiplier,
        ..
    } = CONFIG.load(deps.storage)?;

    // get eclip_per_second values
    let (decreasing_rewards_date, eclip_per_second_before, eclip_per_second_after) =
        helpers::split_eclip_per_second(
            deps.storage,
            eclip_per_second_multiplier,
            eclip_per_second,
            block_time,
        )?;

    let staker_info = STAKER_INFO.load(deps.storage, &staker).unwrap_or_default();
    let locker_infos = LOCKER_INFO.load(deps.storage, &staker).unwrap_or_default();

    let staking_essence_components = STAKING_ESSENCE_COMPONENTS
        .load(deps.storage, &staker)
        .unwrap_or_default();
    let (a, b) = staking_essence_components;
    let staking_essence =
        math::calc_staking_essence_from_components(a, b, block_time, seconds_per_essence);

    let locking_essence = LOCKING_ESSENCE
        .load(deps.storage, &staker)
        .unwrap_or_default();

    let total_essence = helpers::get_total_essence(deps.storage, block_time, seconds_per_essence)?;

    let mut staking_rewards = Uint128::zero();
    let mut funds_to_unstake = Uint128::zero();

    let staking_vaults_info = staker_info
        .vaults
        .iter()
        .map(|vault| {
            funds_to_unstake += vault.amount;

            let staking_essence_per_vault = math::calc_staking_essence_per_vault(
                vault.amount,
                vault.creation_date,
                block_time,
                seconds_per_essence,
            );

            let staking_rewards_per_vault = math::calc_staking_rewards_per_vault(
                vault.accumulated_rewards,
                staking_essence_per_vault,
                vault.claim_date,
                decreasing_rewards_date,
                block_time,
                eclip_per_second_before,
                eclip_per_second_after,
                total_essence,
            );

            staking_rewards += staking_rewards_per_vault;

            EssenceAndRewardsInfo {
                staking_essence: staking_essence_per_vault,
                locking_essence: Uint128::zero(),
                essence: staking_essence_per_vault,
                staking_rewards: staking_rewards_per_vault,
                locking_rewards: Uint128::zero(),
                rewards: staking_rewards_per_vault,
                penalty: Uint128::zero(),
            }
        })
        .collect();

    let mut locking_rewards = Uint128::zero();
    let mut unlock_penalty = Uint128::zero();
    let mut funds_to_unlock = Uint128::zero();

    let locking_vaults_info = locker_infos
        .iter()
        .cloned()
        .map(|locker_info| {
            let LockerInfo { lock_tier, vaults } = locker_info;
            let (locking_period, _) = lock_schedule[lock_tier as usize];

            let vaults = vaults
                .iter()
                .map(|vault| {
                    funds_to_unlock += vault.amount;

                    let locking_essence_per_vault = math::calc_locking_essence_per_vault(
                        vault.amount,
                        locking_period,
                        seconds_per_essence,
                    );

                    let penalty_per_vault = math::calc_penalty_per_tier(
                        &[vault.clone()],
                        locking_period,
                        block_time,
                        penalty_multiplier,
                    );

                    unlock_penalty += penalty_per_vault;

                    let locking_rewards_per_vault = math::calc_locking_rewards_per_vault(
                        vault.accumulated_rewards,
                        locking_essence_per_vault,
                        vault.claim_date,
                        decreasing_rewards_date,
                        block_time,
                        eclip_per_second_before,
                        eclip_per_second_after,
                        total_essence,
                    );

                    locking_rewards += locking_rewards_per_vault;

                    EssenceAndRewardsInfo {
                        staking_essence: Uint128::zero(),
                        locking_essence: locking_essence_per_vault,
                        essence: locking_essence_per_vault,
                        staking_rewards: Uint128::zero(),
                        locking_rewards: locking_rewards_per_vault,
                        rewards: locking_rewards_per_vault,
                        penalty: penalty_per_vault,
                    }
                })
                .collect();

            (lock_tier, vaults)
        })
        .collect();

    let essence_and_rewards_info = EssenceAndRewardsInfo {
        staking_essence,
        locking_essence,
        essence: staking_essence + locking_essence,
        staking_rewards,
        locking_rewards,
        rewards: staking_rewards + locking_rewards,
        penalty: unlock_penalty,
    };

    Ok(StakerInfoResponse {
        staker,
        staker_info,
        locker_infos,
        staking_vaults_info,
        locking_vaults_info,
        essence_and_rewards_info,
        funds_to_unstake,
        funds_to_unlock,
        block_time,
    })
}

pub fn query_gov_essence_reduced(
    deps: Deps,
    _env: Env,
    address_list: Vec<String>,
) -> StdResult<Vec<(Addr, EssenceInfo)>> {
    Ok(address_list
        .iter()
        .map(|address| {
            let user = Addr::unchecked(address);
            let (a, b) = STAKING_ESSENCE_COMPONENTS
                .load(deps.storage, &user)
                .unwrap_or_default();
            let le = LOCKING_ESSENCE
                .load(deps.storage, &user)
                .unwrap_or_default();

            (user, EssenceInfo::new(a, b, le))
        })
        .collect())
}

pub fn query_essence(deps: Deps, env: Env, user: String) -> StdResult<QueryEssenceResponse> {
    let block_time = env.block.time.seconds();
    let user = &deps.api.addr_validate(&user)?;
    let Config {
        seconds_per_essence,
        ..
    } = CONFIG.load(deps.storage)?;
    let staking_essence_components = STAKING_ESSENCE_COMPONENTS
        .load(deps.storage, user)
        .unwrap_or_default();
    let (a, b) = staking_essence_components;
    let staking_essence =
        math::calc_staking_essence_from_components(a, b, block_time, seconds_per_essence);
    let locking_essence = LOCKING_ESSENCE.load(deps.storage, user).unwrap_or_default();
    let essence = staking_essence + locking_essence;

    Ok(QueryEssenceResponse {
        staking_essence_components,
        staking_essence,
        locking_essence,
        essence,
    })
}

pub fn query_total_essence(deps: Deps, env: Env) -> StdResult<QueryEssenceResponse> {
    let block_time = env.block.time.seconds();
    let Config {
        seconds_per_essence,
        ..
    } = CONFIG.load(deps.storage)?;
    let staking_essence_components = TOTAL_STAKING_ESSENCE_COMPONENTS
        .load(deps.storage)
        .unwrap_or_default();
    let (a, b) = staking_essence_components;
    let staking_essence =
        math::calc_staking_essence_from_components(a, b, block_time, seconds_per_essence);
    let locking_essence = TOTAL_LOCKING_ESSENCE.load(deps.storage).unwrap_or_default();
    let essence = staking_essence + locking_essence;

    Ok(QueryEssenceResponse {
        staking_essence_components,
        staking_essence,
        locking_essence,
        essence,
    })
}

pub fn query_time_until_decreasing_rewards(deps: Deps, env: Env) -> StdResult<u64> {
    let block_time = env.block.time.seconds();
    let decreasing_rewards_date = DECREASING_REWARDS_DATE.load(deps.storage)?;

    Ok(decreasing_rewards_date - block_time)
}

// marketing info

pub fn query_users_amount(deps: Deps, _env: Env) -> StdResult<UsersAmountResponse> {
    let stakers: Vec<Addr> = STAKER_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .flatten()
        .filter(|(_address, staker)| !staker.vaults.is_empty())
        .map(|(address, _)| address)
        .collect();

    let lockers: Vec<Addr> = LOCKER_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .flatten()
        .filter(|(_address, lockers)| lockers.iter().any(|x| !x.vaults.is_empty()))
        .map(|(address, _)| address)
        .collect();

    let stakers_and_lockers: Vec<Addr> = stakers
        .iter()
        .cloned()
        .filter(|address| lockers.contains(address))
        .collect();

    let stakers = stakers.len() as u128;
    let lockers = lockers.len() as u128;
    let stakers_and_lockers = stakers_and_lockers.len() as u128;
    let stakers_only = stakers - stakers_and_lockers;
    let lockers_only = lockers - stakers_and_lockers;
    let total = stakers + lockers - stakers_and_lockers;

    Ok(UsersAmountResponse {
        stakers_only: Uint128::from(stakers_only),
        lockers_only: Uint128::from(lockers_only),
        stakers_and_lockers: Uint128::from(stakers_and_lockers),
        total: Uint128::from(total),
    })
}

pub fn query_wallets_per_tier(deps: Deps, _env: Env) -> StdResult<Vec<Uint128>> {
    let Config { lock_schedule, .. } = CONFIG.load(deps.storage)?;

    let lockers: Vec<(Addr, Vec<LockerInfo>)> = LOCKER_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .flatten()
        .filter(|(_address, lockers)| lockers.iter().any(|x| !x.vaults.is_empty()))
        .collect();

    let tiers: Vec<Uint128> = lockers.iter().fold(
        vec![Uint128::zero(); lock_schedule.len()],
        |mut acc, (_address, locker_infos)| {
            for (i, LockerInfo { vaults, .. }) in locker_infos.iter().enumerate() {
                acc[i] += if !vaults.is_empty() {
                    Uint128::one()
                } else {
                    Uint128::zero()
                };
            }

            acc
        },
    );

    Ok(tiers)
}

pub fn query_staking_essence_list(
    deps: Deps,
    _env: Env,
    amount: u32,
    start_from: Option<String>,
    block_time: u64,
) -> StdResult<Vec<QueryEssenceListResponseItem>> {
    let Config {
        seconds_per_essence,
        ..
    } = CONFIG.load(deps.storage)?;

    let address;
    let start_bound = match start_from {
        None => None,
        Some(x) => {
            address = deps.api.addr_validate(&x)?;
            Some(Bound::exclusive(&address))
        }
    };

    Ok(STAKER_INFO
        .range(deps.storage, start_bound, None, Order::Ascending)
        .take(amount as usize)
        .map(|x| {
            let (user, staker_info) = x.unwrap();

            let essence =
                math::calc_staking_essence(&staker_info.vaults, block_time, seconds_per_essence);

            QueryEssenceListResponseItem { user, essence }
        })
        .collect())
}

pub fn query_locking_essence_list(
    deps: Deps,
    _env: Env,
    amount: u32,
    start_from: Option<String>,
) -> StdResult<Vec<QueryEssenceListResponseItem>> {
    let Config {
        lock_schedule,
        seconds_per_essence,
        ..
    } = CONFIG.load(deps.storage)?;

    let address;
    let start_bound = match start_from {
        None => None,
        Some(x) => {
            address = deps.api.addr_validate(&x)?;
            Some(Bound::exclusive(&address))
        }
    };

    Ok(LOCKER_INFO
        .range(deps.storage, start_bound, None, Order::Ascending)
        .take(amount as usize)
        .map(|x| {
            let (user, locker_infos) = x.unwrap();

            let essence =
                math::calc_locking_essence(&locker_infos, &lock_schedule, seconds_per_essence);

            QueryEssenceListResponseItem { user, essence }
        })
        .collect())
}

pub fn query_apr_info(
    deps: Deps,
    env: Env,
    amount_to_add: Option<Uint128>,
    staker_address: Option<String>,
) -> StdResult<QueryAprInfoResponse> {
    let block_time = env.block.time.seconds();
    let amount_to_add = amount_to_add.unwrap_or_default();
    let Config {
        seconds_per_essence,
        eclip_per_second,
        eclip_per_second_multiplier,
        lock_schedule,
        ..
    } = CONFIG.load(deps.storage)?;
    let staking_essence_components = TOTAL_STAKING_ESSENCE_COMPONENTS
        .load(deps.storage)
        .unwrap_or_default();
    let (a, b) = staking_essence_components;
    let current_staking_essence =
        math::calc_staking_essence_from_components(a, b, block_time, seconds_per_essence);
    let expected_staking_essence = math::calc_staking_essence_from_components(
        a,
        b,
        block_time + YEAR_IN_SECONDS,
        seconds_per_essence,
    );
    let locking_essence = TOTAL_LOCKING_ESSENCE.load(deps.storage).unwrap_or_default();
    let current_total_essence = current_staking_essence + locking_essence;
    let expected_total_essence = expected_staking_essence + locking_essence;

    let staking_vaults = staker_address
        .map(|x| -> StdResult<Vec<Vault>> {
            let staker_address = deps.api.addr_validate(&x)?;
            let staker_info = STAKER_INFO.load(deps.storage, &staker_address)?;

            Ok(staker_info.vaults)
        })
        .transpose()?
        .unwrap_or_default();

    let current_staking_apr = math::calc_current_staking_apr(
        &staking_vaults,
        block_time,
        eclip_per_second,
        current_total_essence,
        seconds_per_essence,
    );

    let expected_staking_apr = math::calc_expected_staking_apr(
        amount_to_add,
        eclip_per_second,
        eclip_per_second_multiplier,
        expected_total_essence,
        seconds_per_essence,
    );

    let mut current_locking_apr_list: Vec<LockingAprItem> = vec![];
    let mut expected_locking_apr_list: Vec<LockingAprItem> = vec![];

    for (tier, (locking_period, _global_rewards_per_tier)) in lock_schedule.iter().enumerate() {
        let current_locking_apr = math::calc_current_locking_apr_per_tier(
            amount_to_add,
            eclip_per_second,
            current_total_essence,
            locking_period.to_owned(),
            seconds_per_essence,
        );

        let expected_locking_apr = math::calc_expected_locking_apr_per_tier(
            amount_to_add,
            eclip_per_second,
            eclip_per_second_multiplier,
            expected_total_essence,
            locking_period.to_owned(),
            seconds_per_essence,
        );

        current_locking_apr_list.push(LockingAprItem {
            tier: tier as u64,
            apr: current_locking_apr,
        });
        expected_locking_apr_list.push(LockingAprItem {
            tier: tier as u64,
            apr: expected_locking_apr,
        });
    }

    Ok(QueryAprInfoResponse {
        current: AprInfoItem {
            staking_apr: current_staking_apr,
            locking_apr_list: current_locking_apr_list,
        },
        expected: AprInfoItem {
            staking_apr: expected_staking_apr,
            locking_apr_list: expected_locking_apr_list,
        },
    })
}

pub fn query_staker_info_list(
    deps: Deps,
    _env: Env,
    amount: u32,
    start_from: Option<String>,
) -> StdResult<Vec<(Addr, StakerInfo)>> {
    let address;
    let start_bound = match start_from {
        None => None,
        Some(x) => {
            address = deps.api.addr_validate(&x)?;
            Some(Bound::exclusive(&address))
        }
    };

    STAKER_INFO
        .range(deps.storage, start_bound, None, Order::Ascending)
        .take(amount as usize)
        .collect()
}

pub fn query_locker_info_list(
    deps: Deps,
    _env: Env,
    amount: u32,
    start_from: Option<String>,
) -> StdResult<Vec<(Addr, Vec<LockerInfo>)>> {
    let address;
    let start_bound = match start_from {
        None => None,
        Some(x) => {
            address = deps.api.addr_validate(&x)?;
            Some(Bound::exclusive(&address))
        }
    };

    LOCKER_INFO
        .range(deps.storage, start_bound, None, Order::Ascending)
        .take(amount as usize)
        .collect()
}

pub fn query_rewards_reduction_info(
    deps: Deps,
    _env: Env,
) -> StdResult<QueryRewardsReductionInfoResponse> {
    Ok(QueryRewardsReductionInfoResponse {
        eclip_per_second: CONFIG.load(deps.storage)?.eclip_per_second,
        decreasing_rewards_date: DECREASING_REWARDS_DATE.load(deps.storage)?,
    })
}
