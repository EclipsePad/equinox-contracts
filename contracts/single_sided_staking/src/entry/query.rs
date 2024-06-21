use cosmwasm_std::{Addr, Decimal256, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;
use std::cmp::{max, min};

use crate::{
    config::REWARD_DISTRIBUTION_PERIOD,
    state::{
        CONFIG, LAST_CLAIM_TIME, OWNER, PENDING_ECLIPASTRO_REWARDS, REWARD_WEIGHTS, TOTAL_STAKING,
        TOTAL_STAKING_BY_DURATION, USER_STAKED,
    },
};
use equinox_msg::{
    single_sided_staking::{
        Config, RewardWeights, StakingWithDuration, UserReward, UserRewardByDuration,
        UserRewardByLockedAt, UserStaked, UserStaking, UserStakingByDuration, VaultRewards,
    },
    token_converter::{QueryMsg as ConverterQueryMsg, RewardResponse},
};

/// query owner
pub fn query_owner(deps: Deps, _env: Env) -> StdResult<Addr> {
    let owner = OWNER.get(deps)?;
    Ok(owner.unwrap())
}

/// query config
pub fn query_config(deps: Deps, _env: Env) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

// query total staking
pub fn query_total_staking(deps: Deps, _env: Env) -> StdResult<Uint128> {
    let total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    Ok(total_staking)
}

// query total staking by duration
pub fn query_total_staking_by_duration(
    deps: Deps,
    _env: Env,
) -> StdResult<Vec<StakingWithDuration>> {
    let config = CONFIG.load(deps.storage)?;

    let total_staking = config
        .timelock_config
        .into_iter()
        .map(|c| {
            Ok(StakingWithDuration {
                amount: TOTAL_STAKING_BY_DURATION
                    .load(deps.storage, c.duration)
                    .unwrap_or_default(),
                duration: c.duration,
            })
        })
        .collect::<StdResult<Vec<StakingWithDuration>>>()
        .unwrap_or(vec![]);
    Ok(total_staking)
}

/// query user staking
pub fn query_staking(deps: Deps, _env: Env, user: String) -> StdResult<Vec<UserStaking>> {
    let config = CONFIG.load(deps.storage)?;
    let durations = config
        .timelock_config
        .into_iter()
        .map(|c| c.duration)
        .collect::<Vec<u64>>();
    let mut staking_lists = vec![];
    for duration in durations {
        let staking = USER_STAKED
            .prefix((&user, duration))
            .range(deps.storage, None, None, Order::Ascending)
            .map(|s| {
                let (locked_at, staking) = s?;
                let locked_at = if locked_at == 0u64 {
                    None
                } else {
                    Some(locked_at)
                };
                Ok(UserStakingByDuration {
                    locked_at,
                    amount: staking.staked,
                })
            })
            .collect::<StdResult<Vec<UserStakingByDuration>>>()
            .unwrap_or(vec![]);
        if !staking.is_empty() {
            staking_lists.push(UserStaking { duration, staking });
        }
    }
    Ok(staking_lists)
}

/// query user reward
pub fn query_reward(deps: Deps, env: Env, user: String) -> StdResult<Vec<UserRewardByDuration>> {
    let current_time = env.block.time.seconds();
    let user_reward = calculate_total_user_reward(deps, user, current_time)?;
    Ok(user_reward)
}

/// calculate penalty amount
// penalty bps will be only affected to staking amount, not reward amount
pub fn calculate_penalty(
    deps: Deps,
    env: Env,
    amount: Uint128,
    duration: u64,
    locked_at: u64,
) -> StdResult<Uint128> {
    let config = CONFIG.load(deps.storage)?;
    let current_time = env.block.time.seconds();
    if locked_at + duration <= current_time {
        return Ok(Uint128::zero());
    };
    // config is removed by owner so users can unlock immediately
    if let Some(timelock_config) = config
        .timelock_config
        .into_iter()
        .find(|c| c.duration == duration)
    {
        let penalty_amount = amount
            // .multiply_ratio(locked_at + duration - current_time, duration)
            .multiply_ratio(timelock_config.early_unlock_penalty_bps, 10000u128);
        Ok(penalty_amount)
    } else {
        Ok(Uint128::zero())
    }
}

pub fn calculate_updated_reward_weights(deps: Deps, current_time: u64) -> StdResult<RewardWeights> {
    let mut reward_weights = REWARD_WEIGHTS.load(deps.storage).unwrap_or_default();
    let last_claim_time = LAST_CLAIM_TIME.load(deps.storage).unwrap_or(current_time);
    let total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    if total_staking.is_zero() {
        return Ok(reward_weights);
    }

    let total_staking_with_multiplier = calculate_total_staking_with_multiplier(deps)?;
    let pending_eclipastro_reward =
        calculate_eclipastro_reward(deps, last_claim_time, current_time)?;
    let pending_vault_rewards = calculate_vault_rewards(deps, last_claim_time, current_time)?;
    reward_weights.eclipastro += Decimal256::from_ratio(pending_eclipastro_reward, total_staking);
    reward_weights.eclip +=
        Decimal256::from_ratio(pending_vault_rewards.eclip, total_staking_with_multiplier);
    reward_weights.beclip +=
        Decimal256::from_ratio(pending_vault_rewards.beclip, total_staking_with_multiplier);
    Ok(reward_weights)
}

pub fn calculate_eclipastro_reward(
    deps: Deps,
    last_claim_time: u64,
    current_time: u64,
) -> StdResult<Uint128> {
    if last_claim_time == current_time {
        return Ok(Uint128::zero());
    }
    let start_bound = Some(Bound::exclusive(
        last_claim_time - REWARD_DISTRIBUTION_PERIOD,
    ));
    let mut pending_rewards = Uint128::zero();
    let keys = PENDING_ECLIPASTRO_REWARDS
        .keys(deps.storage, start_bound, None, Order::Ascending)
        .collect::<StdResult<Vec<u64>>>()
        .unwrap_or(vec![]);
    for k in keys.into_iter() {
        let pending_reward = PENDING_ECLIPASTRO_REWARDS
            .load(deps.storage, k)
            .unwrap_or_default();
        let start_time = max(k, last_claim_time);
        let end_time = min(k + REWARD_DISTRIBUTION_PERIOD, current_time);
        pending_rewards +=
            pending_reward.multiply_ratio(end_time - start_time, REWARD_DISTRIBUTION_PERIOD);
    }
    Ok(pending_rewards)
}

pub fn calculate_vault_rewards(
    deps: Deps,
    last_claim_time: u64,
    current_time: u64,
) -> StdResult<VaultRewards> {
    let config = CONFIG.load(deps.storage)?;
    Ok(VaultRewards {
        eclip: config
            .rewards
            .eclip
            .daily_reward
            .multiply_ratio(current_time - last_claim_time, 86400u64),
        beclip: config
            .rewards
            .beclip
            .daily_reward
            .multiply_ratio(current_time - last_claim_time, 86400u64),
    })
}

pub fn calculate_total_staking_with_multiplier(deps: Deps) -> StdResult<Uint128> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config
        .timelock_config
        .into_iter()
        .fold(Uint128::zero(), |acc, cur| {
            let duration = cur.duration;
            let multiplier = cur.reward_multiplier;
            let total_staking_by_duration = TOTAL_STAKING_BY_DURATION
                .load(deps.storage, duration)
                .unwrap_or_default();
            acc + total_staking_by_duration
                .checked_mul(Uint128::from(multiplier))
                .unwrap()
        }))
}

pub fn calculate_user_reward(
    deps: Deps,
    user: String,
    duration: u64,
    locked_at: u64,
    current_time: u64,
) -> StdResult<UserReward> {
    let config = CONFIG.load(deps.storage)?;
    let multiplier = config
        .timelock_config
        .into_iter()
        .find(|c| c.duration == duration)
        .unwrap()
        .reward_multiplier;
    let updated_reward_weights = calculate_updated_reward_weights(deps, current_time)?;
    let user_staking = USER_STAKED
        .load(deps.storage, (&user, duration, locked_at))
        .unwrap_or(UserStaked {
            staked: Uint128::zero(),
            reward_weights: updated_reward_weights.clone(),
        });
    Ok(UserReward {
        eclipastro: updated_reward_weights
            .eclipastro
            .checked_sub(user_staking.reward_weights.eclipastro)
            .unwrap_or_default()
            .checked_mul(Decimal256::from_ratio(user_staking.staked, 1u128))
            .unwrap()
            .to_uint_floor()
            .try_into()?,
        beclip: updated_reward_weights
            .beclip
            .checked_sub(user_staking.reward_weights.beclip)
            .unwrap_or_default()
            .checked_mul(Decimal256::from_ratio(user_staking.staked, 1u128))
            .unwrap()
            .checked_mul(Decimal256::from_ratio(multiplier, 1u128))
            .unwrap()
            .to_uint_floor()
            .try_into()?,
        eclip: updated_reward_weights
            .eclip
            .checked_sub(user_staking.reward_weights.eclip)
            .unwrap_or_default()
            .checked_mul(Decimal256::from_ratio(user_staking.staked, 1u128))
            .unwrap()
            .checked_mul(Decimal256::from_ratio(multiplier, 1u128))
            .unwrap()
            .to_uint_floor()
            .try_into()?,
    })
}

pub fn calculate_total_user_reward(
    deps: Deps,
    user: String,
    current_time: u64,
) -> StdResult<Vec<UserRewardByDuration>> {
    let config = CONFIG.load(deps.storage)?;
    let updated_reward_weights = calculate_updated_reward_weights(deps, current_time)?;
    let mut total_user_reward = vec![];
    for timelock_config in config.timelock_config {
        let duration = timelock_config.duration;
        let multiplier = timelock_config.reward_multiplier;
        let user_reward_by_duration = USER_STAKED
            .prefix((&user, duration))
            .range(deps.storage, None, None, Order::Ascending)
            .map(|s| {
                let (locked_at, staking_data) = s.unwrap();
                Ok(UserRewardByLockedAt {
                    locked_at,
                    rewards: UserReward {
                        eclipastro: updated_reward_weights
                            .eclipastro
                            .checked_sub(staking_data.reward_weights.eclipastro)
                            .unwrap_or_default()
                            .checked_mul(Decimal256::from_ratio(staking_data.staked, 1u128))
                            .unwrap()
                            .to_uint_floor()
                            .try_into()?,
                        beclip: updated_reward_weights
                            .beclip
                            .checked_sub(staking_data.reward_weights.beclip)
                            .unwrap_or_default()
                            .checked_mul(Decimal256::from_ratio(staking_data.staked, 1u128))
                            .unwrap()
                            .checked_mul(Decimal256::from_ratio(multiplier, 1u128))
                            .unwrap()
                            .to_uint_floor()
                            .try_into()?,
                        eclip: updated_reward_weights
                            .eclip
                            .checked_sub(staking_data.reward_weights.eclip)
                            .unwrap_or_default()
                            .checked_mul(Decimal256::from_ratio(staking_data.staked, 1u128))
                            .unwrap()
                            .checked_mul(Decimal256::from_ratio(multiplier, 1u128))
                            .unwrap()
                            .to_uint_floor()
                            .try_into()?,
                    },
                })
            })
            .collect::<StdResult<Vec<UserRewardByLockedAt>>>()
            .unwrap_or_default();
        total_user_reward.push(UserRewardByDuration {
            duration,
            rewards: user_reward_by_duration,
        });
    }
    Ok(total_user_reward)
}
pub fn query_eclipastro_pending_rewards(
    deps: Deps,
    converter_contract: String,
) -> StdResult<Uint128> {
    let rewards: RewardResponse = deps
        .querier
        .query_wasm_smart(converter_contract.clone(), &ConverterQueryMsg::Rewards {})
        .unwrap();
    Ok(rewards.users_reward.amount)
}

pub fn query_eclipastro_rewards(deps: Deps, env: Env) -> StdResult<Vec<(u64, Uint128)>> {
    let start_bound = Some(Bound::exclusive(
        env.block.time.seconds() - REWARD_DISTRIBUTION_PERIOD,
    ));
    let keys = PENDING_ECLIPASTRO_REWARDS
        .keys(deps.storage, start_bound, None, Order::Ascending)
        .collect::<StdResult<Vec<u64>>>()
        .unwrap_or(vec![]);
    let mut pending_rewards = vec![];
    for k in keys.into_iter() {
        let pending_reward = PENDING_ECLIPASTRO_REWARDS
            .load(deps.storage, k)
            .unwrap_or_default();
        pending_rewards.push((k, pending_reward));
    }
    Ok(pending_rewards)
}
