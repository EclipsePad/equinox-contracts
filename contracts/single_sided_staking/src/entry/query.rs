use cosmwasm_std::{Addr, Decimal256, Deps, Env, Order, StdResult, Uint128, Uint256};
use cw_storage_plus::Bound;
use std::cmp::{max, min};

use crate::{
    config::{BPS_DENOMINATOR, ECLIPASTRO_REWARD_DISTRIBUTION_PERIOD, ONE_DAY},
    state::{
        RewardWeights, TotalStakingByDuration, BLACK_LIST, BLACK_LIST_REWARDS, CONFIG,
        LAST_CLAIM_TIME, OWNER, PENDING_ECLIPASTRO_REWARDS, REWARD, STAKING_DURATION_BY_END_TIME,
        TOTAL_STAKING, USER_STAKED,
    },
};
use equinox_msg::{
    single_sided_staking::{
        Config, Reward, StakingWithDuration, UserReward, UserRewardByDuration,
        UserRewardByLockedAt, UserStaking, UserStakingByDuration,
    },
    voter::msg::{AstroStakingRewardResponse, QueryMsg as VoterQueryMsg},
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
    env: Env,
    timestamp: Option<u64>,
) -> StdResult<Vec<StakingWithDuration>> {
    let config = CONFIG.load(deps.storage)?;
    let block_time = env.block.time.seconds();
    let staking = config
        .timelock_config
        .into_iter()
        .map(|c| {
            Ok(total_staking_by_duration(
                deps,
                block_time,
                timestamp.unwrap_or(block_time),
                c.duration,
            )
            .unwrap_or_default())
        })
        .collect::<StdResult<Vec<StakingWithDuration>>>()
        .unwrap_or(vec![]);
    Ok(staking)
}

pub fn total_staking_by_duration(
    deps: Deps,
    block_time: u64,
    mut timestamp: u64,
    duration: u64,
) -> StdResult<StakingWithDuration> {
    if timestamp > block_time {
        timestamp = block_time;
    }
    let last_claim_time = LAST_CLAIM_TIME.load(deps.storage)?;
    let mut next_check_time = last_claim_time / ONE_DAY * ONE_DAY + ONE_DAY;
    let mut staking_by_duration = TotalStakingByDuration::load_at_ts(
        deps.storage,
        duration,
        block_time,
        Some(min(last_claim_time, timestamp)),
    )
    .unwrap_or_default();
    if timestamp < next_check_time {
        Ok(StakingWithDuration {
            staked: staking_by_duration.staked,
            valid_staked: staking_by_duration.valid_staked,
            duration,
        })
    } else {
        loop {
            let staking_by_endtime =
                STAKING_DURATION_BY_END_TIME.load(deps.storage, (duration, next_check_time))?;
            if !staking_by_endtime.is_zero() {
                staking_by_duration.valid_staked -= staking_by_endtime;
            }
            next_check_time += ONE_DAY;
            if timestamp < next_check_time {
                break;
            }
        }
        Ok(StakingWithDuration {
            staked: staking_by_duration.staked,
            valid_staked: staking_by_duration.valid_staked,
            duration,
        })
    }
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
pub fn query_reward(
    deps: Deps,
    env: Env,
    user: String,
    duration: u64,
    locked_at: u64,
) -> StdResult<UserReward> {
    let blacklist = BLACK_LIST.load(deps.storage).unwrap_or_default();
    if blacklist.contains(&user) {
        return Ok(UserReward::default());
    }
    _user_reward(deps, env, user, duration, locked_at)
}

pub fn query_reward_schedule(
    deps: Deps,
    env: Env,
    from: Option<u64>,
) -> StdResult<Vec<((u64, u64), Reward)>> {
    REWARD
        .range(
            deps.storage,
            Some(Bound::exclusive((
                from.unwrap_or(env.block.time.seconds()),
                0u64,
            ))),
            None,
            Order::Ascending,
        )
        .collect::<StdResult<Vec<_>>>()
}

pub fn query_reward_list(
    deps: Deps,
    env: Env,
    user: String,
) -> StdResult<Vec<(u64, u64, UserReward)>> {
    USER_STAKED
        .sub_prefix(&user)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|s| {
            let ((duration, locked_at), _) = s.unwrap();
            let user_reward =
                _user_reward(deps, env.clone(), user.clone(), duration, locked_at).unwrap();
            Ok((duration, locked_at, user_reward))
        })
        .collect()
}

pub fn _user_reward(
    deps: Deps,
    env: Env,
    user: String,
    duration: u64,
    locked_at: u64,
) -> StdResult<UserReward> {
    let config = CONFIG.load(deps.storage)?;
    let user_staking = USER_STAKED.load(deps.storage, (&user, duration, locked_at))?;
    let block_time = env.block.time.seconds();
    let end_time = calculate_lock_end_time(duration, locked_at);
    let multiplier = config
        .timelock_config
        .iter()
        .find(|c| c.duration == duration)
        .unwrap()
        .reward_multiplier;
    let flexible_multiplier = config
        .timelock_config
        .iter()
        .find(|c| c.duration == 0u64)
        .unwrap()
        .reward_multiplier;
    if end_time >= block_time || duration == 0u64 {
        calculate_reward_with_multiplier(
            multiplier,
            user_staking.reward_weights,
            calculate_reward_weights(deps, env, block_time, block_time)?,
            user_staking.staked,
        )
    } else {
        let reward_during_lock = calculate_reward_with_multiplier(
            multiplier,
            user_staking.reward_weights.clone(),
            calculate_reward_weights(deps, env.clone(), block_time, end_time)?,
            user_staking.staked,
        )?;
        let reward_after_lock = calculate_reward_with_multiplier(
            flexible_multiplier,
            calculate_reward_weights(deps, env.clone(), block_time, end_time)?,
            calculate_reward_weights(deps, env.clone(), block_time, block_time)?,
            user_staking.staked,
        )?;
        Ok(UserReward {
            eclipastro: reward_during_lock.eclipastro + reward_after_lock.eclipastro,
            beclip: reward_during_lock.beclip + reward_after_lock.beclip,
            eclip: reward_during_lock.eclip + reward_after_lock.eclip,
        })
    }
}

pub fn query_calculate_reward(
    deps: Deps,
    env: Env,
    amount: Uint128,
    duration: u64,
    locked_at: Option<u64>,
    from: u64,
    to: Option<u64>,
) -> StdResult<UserReward> {
    let config = CONFIG.load(deps.storage)?;
    let block_time = env.block.time.seconds();
    let end_time = calculate_lock_end_time(duration, locked_at.unwrap_or_default());
    let to = to.unwrap_or(block_time);
    let multiplier = config
        .timelock_config
        .iter()
        .find(|c| c.duration == duration)
        .unwrap()
        .reward_multiplier;
    let flexible_multiplier = config
        .timelock_config
        .iter()
        .find(|c| c.duration == 0u64)
        .unwrap()
        .reward_multiplier;
    if end_time >= to {
        calculate_reward_with_multiplier(
            multiplier,
            RewardWeights::load_at_ts(deps.storage, block_time, Some(from))?,
            calculate_reward_weights(deps, env, block_time, block_time)?,
            amount,
        )
    } else {
        let reward_during_lock = calculate_reward_with_multiplier(
            multiplier,
            RewardWeights::load_at_ts(deps.storage, block_time, Some(from))?,
            calculate_reward_weights(deps, env.clone(), block_time, end_time)?,
            amount,
        )?;
        let reward_after_lock = calculate_reward_with_multiplier(
            flexible_multiplier,
            calculate_reward_weights(deps, env.clone(), block_time, end_time)?,
            calculate_reward_weights(deps, env.clone(), block_time, to)?,
            amount,
        )?;
        Ok(UserReward {
            eclipastro: reward_during_lock.eclipastro + reward_after_lock.eclipastro,
            beclip: reward_during_lock.beclip + reward_after_lock.beclip,
            eclip: reward_during_lock.eclip + reward_after_lock.eclip,
        })
    }
}

pub fn calculate_reward_weights(
    deps: Deps,
    env: Env,
    block_time: u64,
    timestamp: u64,
) -> StdResult<RewardWeights> {
    let cfg = CONFIG.load(deps.storage)?;
    let last_claim_time = LAST_CLAIM_TIME.load(deps.storage).unwrap_or(block_time);
    if last_claim_time.ge(&timestamp) {
        return Ok(
            RewardWeights::load_at_ts(deps.storage, block_time, Some(timestamp))
                .unwrap_or_default(),
        );
    }
    let total_staking = TOTAL_STAKING.load(deps.storage)?;
    let mut total_staking_by_durations = cfg
        .timelock_config
        .into_iter()
        .map(|tc| {
            let duration = tc.duration;
            (
                duration,
                tc.reward_multiplier,
                TotalStakingByDuration::load_at_ts(
                    deps.storage,
                    duration,
                    block_time,
                    Some(last_claim_time),
                )
                .unwrap_or_default(),
            )
        })
        .collect::<Vec<(u64, u64, TotalStakingByDuration)>>();
    let mut reward_weights =
        RewardWeights::load_at_ts(deps.storage, block_time, Some(last_claim_time))?;
    let mut start_time = last_claim_time;
    let mut end_time = last_claim_time / ONE_DAY * ONE_DAY + ONE_DAY;
    if total_staking.is_zero() {
        return Ok(reward_weights);
    }
    loop {
        if timestamp.le(&end_time) {
            end_time = timestamp;
        }
        let boost_sum = total_staking_by_durations
            .iter()
            .fold(Uint256::zero(), |acc, cur| {
                acc + (Uint256::from_uint128(cur.2.valid_staked) * Uint256::from_u128(cur.1.into())
                    + Uint256::from_uint128(cur.2.staked - cur.2.valid_staked)
                        * Uint256::from_u128(cur.1.into()))
                    / Uint256::from_u128(BPS_DENOMINATOR.into())
            });
        let (eclip_reward, beclip_reward) =
            calculate_eclip_beclip_reward(deps, start_time, end_time)?;
        reward_weights.eclip += Decimal256::from_ratio(eclip_reward, boost_sum);
        reward_weights.beclip += Decimal256::from_ratio(beclip_reward, boost_sum);
        let pending_eclipastro_reward =
            calculate_eclipastro_reward(deps, env.clone(), start_time, end_time)?;
        reward_weights.eclipastro +=
            Decimal256::from_ratio(pending_eclipastro_reward, total_staking);
        if end_time.le(&timestamp) {
            return Ok(reward_weights);
        }
        total_staking_by_durations = total_staking_by_durations
            .into_iter()
            .map(|(duration, multiplier, mut staking)| {
                let lock_end_staking = STAKING_DURATION_BY_END_TIME
                    .load(deps.storage, (duration, end_time))
                    .unwrap_or_default();
                staking.valid_staked -= lock_end_staking;
                (duration, multiplier, staking)
            })
            .collect::<Vec<(u64, u64, TotalStakingByDuration)>>();
        start_time = end_time;
        end_time += ONE_DAY;
    }
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
        let penalty_amount =
            amount.multiply_ratio(timelock_config.early_unlock_penalty_bps, BPS_DENOMINATOR);
        Ok(penalty_amount)
    } else {
        Ok(Uint128::zero())
    }
}
/// calculate total eclipastro reward during period
pub fn calculate_eclipastro_reward(
    deps: Deps,
    env: Env,
    last_claim_time: u64,
    current_time: u64,
) -> StdResult<Uint128> {
    if last_claim_time == current_time {
        return Ok(Uint128::zero());
    }
    let mut pending_rewards = Uint128::zero();
    let eclipastro_rewards = query_eclipastro_rewards(deps, env)?;
    for (start_time, amount) in eclipastro_rewards.into_iter() {
        let start_time = max(start_time, last_claim_time);
        let end_time = min(
            start_time + ECLIPASTRO_REWARD_DISTRIBUTION_PERIOD,
            current_time,
        );
        pending_rewards +=
            amount.multiply_ratio(end_time - start_time, ECLIPASTRO_REWARD_DISTRIBUTION_PERIOD);
    }
    Ok(pending_rewards)
}

pub fn calculate_user_reward(
    deps: Deps,
    user: String,
    duration: u64,
    locked_at: u64,
    block_time: u64,
) -> StdResult<UserReward> {
    let config = CONFIG.load(deps.storage)?;
    let user_staking = USER_STAKED
        .load(deps.storage, (&user, duration, locked_at))
        .unwrap_or_default();
    let lock_end_time = calculate_lock_end_time(duration, locked_at);
    let multiplier = config
        .timelock_config
        .iter()
        .find(|c| c.duration == duration)
        .unwrap()
        .reward_multiplier;

    let flexible_multiplier = config
        .timelock_config
        .into_iter()
        .find(|c| c.duration == 0)
        .unwrap()
        .reward_multiplier;
    if lock_end_time < block_time {
        let reward_weights = RewardWeights::load_at_ts(deps.storage, block_time, Some(block_time))
            .unwrap_or_default();
        calculate_reward_with_multiplier(
            multiplier,
            user_staking.reward_weights,
            reward_weights,
            user_staking.staked,
        )
    } else {
        let reward_weights =
            RewardWeights::load_at_ts(deps.storage, block_time, Some(lock_end_time))
                .unwrap_or_default();
        let reward_during_lock = calculate_reward_with_multiplier(
            multiplier,
            user_staking.reward_weights.clone(),
            reward_weights.clone(),
            user_staking.staked,
        )?;
        let next_reward_weights =
            RewardWeights::load_at_ts(deps.storage, block_time, Some(block_time))
                .unwrap_or_default();
        let reward_after_lock = calculate_reward_with_multiplier(
            flexible_multiplier,
            reward_weights,
            next_reward_weights,
            user_staking.staked,
        )?;
        Ok(UserReward {
            eclipastro: reward_during_lock.eclipastro + reward_after_lock.eclipastro,
            beclip: reward_during_lock.beclip + reward_after_lock.beclip,
            eclip: reward_during_lock.eclip + reward_after_lock.eclip,
        })
    }
}

pub fn calculate_reward_with_multiplier(
    multiplier: u64,
    start_reward_weights: RewardWeights,
    end_reward_weights: RewardWeights,
    staked: Uint128,
) -> StdResult<UserReward> {
    Ok(UserReward {
        eclipastro: end_reward_weights
            .eclipastro
            .checked_sub(start_reward_weights.eclipastro)
            .unwrap_or_default()
            .checked_mul(Decimal256::from_ratio(staked, 1u128))
            .unwrap()
            .to_uint_floor()
            .try_into()?,
        beclip: end_reward_weights
            .beclip
            .checked_sub(start_reward_weights.beclip)
            .unwrap_or_default()
            .checked_mul(Decimal256::from_ratio(staked, 1u128))
            .unwrap()
            .checked_mul(Decimal256::from_ratio(multiplier, BPS_DENOMINATOR))
            .unwrap()
            .to_uint_floor()
            .try_into()?,
        eclip: end_reward_weights
            .eclip
            .checked_sub(start_reward_weights.eclip)
            .unwrap_or_default()
            .checked_mul(Decimal256::from_ratio(staked, 1u128))
            .unwrap()
            .checked_mul(Decimal256::from_ratio(multiplier, BPS_DENOMINATOR))
            .unwrap()
            .to_uint_floor()
            .try_into()?,
    })
}

pub fn calculate_total_user_reward(
    deps: Deps,
    user: String,
    block_time: u64,
) -> StdResult<Vec<UserRewardByDuration>> {
    let config = CONFIG.load(deps.storage)?;
    let mut total_user_reward = vec![];
    for timelock_config in config.timelock_config {
        let duration = timelock_config.duration;
        let user_reward_by_duration = USER_STAKED
            .prefix((&user, duration))
            .range(deps.storage, None, None, Order::Ascending)
            .map(|s| {
                let (locked_at, _) = s.unwrap();
                Ok(UserRewardByLockedAt {
                    locked_at,
                    rewards: calculate_user_reward(
                        deps,
                        user.clone(),
                        duration,
                        locked_at,
                        block_time,
                    )?,
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

pub fn query_eclipastro_pending_rewards(deps: Deps, voter: String) -> StdResult<Uint128> {
    let rewards: AstroStakingRewardResponse = deps
        .querier
        .query_wasm_smart(voter, &VoterQueryMsg::AstroStakingRewards {})
        .unwrap();
    Ok(rewards.users)
}

pub fn query_eclipastro_rewards(deps: Deps, env: Env) -> StdResult<Vec<(u64, Uint128)>> {
    let start_bound = Some(Bound::exclusive(
        env.block.time.seconds() - ECLIPASTRO_REWARD_DISTRIBUTION_PERIOD,
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

pub fn query_blacklist(deps: Deps) -> StdResult<Vec<String>> {
    Ok(BLACK_LIST.load(deps.storage).unwrap_or_default())
}

pub fn query_blacklist_rewards(deps: Deps, env: Env) -> StdResult<UserReward> {
    let config = CONFIG.load(deps.storage)?;
    let blacklist = BLACK_LIST.load(deps.storage).unwrap_or_default();
    let mut blacklist_rewards = BLACK_LIST_REWARDS.load(deps.storage).unwrap_or_default();
    for user in blacklist.iter() {
        for timelock_config in &config.timelock_config {
            let duration = timelock_config.duration;
            for s in USER_STAKED.prefix((&user, duration)).range(
                deps.storage,
                None,
                None,
                Order::Ascending,
            ) {
                let (locked_at, _) = s.unwrap();
                let rewards = _user_reward(deps, env.clone(), user.clone(), duration, locked_at)?;
                blacklist_rewards.eclip += rewards.eclip;
                blacklist_rewards.beclip += rewards.beclip;
                blacklist_rewards.eclipastro += rewards.eclipastro;
            }
        }
    }
    Ok(blacklist_rewards)
}

pub fn calculate_lock_end_time(duration: u64, locked_at: u64) -> u64 {
    (duration + locked_at) / ONE_DAY * ONE_DAY + ONE_DAY
}

pub fn calculate_eclip_beclip_reward(
    deps: Deps,
    start_time: u64,
    end_time: u64,
) -> StdResult<(Uint128, Uint128)> {
    let rewards = REWARD
        .range(
            deps.storage,
            Some(Bound::exclusive((start_time, 0u64))),
            None,
            Order::Ascending,
        )
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let mut eclip_reward = Uint128::zero();
    let mut beclip_reward = Uint128::zero();
    for ((end, start), reward) in rewards {
        let duration = end - start;
        if start >= end_time {
            continue;
        }
        eclip_reward += reward
            .eclip
            .multiply_ratio(min(end_time, end) - max(start_time, start), duration);
        beclip_reward += reward
            .beclip
            .multiply_ratio(min(end_time, end) - max(start_time, start), duration);
    }
    Ok((eclip_reward, beclip_reward))
}
