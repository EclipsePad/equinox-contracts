use std::cmp::min;

use cosmwasm_std::{ensure_eq, Addr, Decimal256, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;
use equinox_msg::reward_distributor::{
    Config, FlexibleReward, StakingData, TimelockReward, TotalStakingData, UserRewardResponse,
};

use crate::{
    error::ContractError,
    state::{
        UserStakingData, CONFIG, FLEXIBLE_USER_STAKING, LAST_UPDATE_TIME, OWNER, PENDING_REWARDS,
        REWARD_DISTRIBUTION_PERIOD, REWARD_WEIGHT_MULTIPLIER, TIMELOCK_USER_STAKING, TOTAL_STAKING,
    },
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

/// query user reward
pub fn query_reward(deps: Deps, env: Env, user: String) -> StdResult<UserRewardResponse> {
    let total_staking_data = total_staking_reward_update(deps, env)?;
    let mut flexible_user_staking = FLEXIBLE_USER_STAKING
        .load(deps.storage, &user)
        .unwrap_or_default();
    flexible_user_staking =
        user_reward_update(deps, &total_staking_data, 0u64, &flexible_user_staking)?;
    let timelock_rewards = TIMELOCK_USER_STAKING
        .sub_prefix(&user)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|s| {
            let ((duration, locked_at), mut staking_data) = s?;
            staking_data = user_reward_update(deps, &total_staking_data, duration, &staking_data)?;
            Ok(TimelockReward {
                duration,
                locked_at,
                eclip: staking_data.rewards.eclip.pending_reward,
                eclipastro: staking_data.rewards.eclipastro.pending_reward,
            })
        })
        .collect::<StdResult<Vec<TimelockReward>>>()
        .unwrap_or(vec![]);
    Ok(UserRewardResponse {
        flexible: FlexibleReward {
            eclip: flexible_user_staking.rewards.eclip.pending_reward.clone(),
            eclipastro: flexible_user_staking
                .rewards
                .eclipastro
                .pending_reward
                .clone(),
        },
        timelock: timelock_rewards,
    })
}

pub fn query_total_staking(deps: Deps, env: Env) -> StdResult<TotalStakingData> {
    let total_staking_data = total_staking_reward_update(deps, env)?;
    Ok(total_staking_data)
}

pub fn query_pending_rewards(deps: Deps, _env: Env) -> StdResult<Vec<(u64, Uint128)>> {
    let pending_rewards = PENDING_REWARDS
        .range(deps.storage, None, None, Order::Descending)
        .take(30)
        .collect::<StdResult<Vec<(u64, Uint128)>>>()
        .unwrap_or(vec![]);
    Ok(pending_rewards)
}

pub fn total_staking_reward_update(deps: Deps, env: Env) -> StdResult<TotalStakingData> {
    let current_time = env.block.time.seconds();
    let config = CONFIG.load(deps.storage)?;
    let mut total_staking_data = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    let last_update_time = LAST_UPDATE_TIME
        .load(deps.storage)
        .unwrap_or(current_time);
    let pending_eclipastro_reward = calculate_eclipastro_reward(deps, current_time);
    if pending_eclipastro_reward.gt(&Uint128::zero()) {
        // calculate total staking amount
        let total_staking = total_staking_data
            .staking_data
            .iter()
            .fold(Uint128::zero(), |acc, cur| {
                acc.checked_add(cur.amount).unwrap()
            });
        // update eclipASTRO reward weight
        // new_weight = old_weight + (converted eclipASTRO reward from xASTRO reward) / total_staking
        if total_staking.gt(&Uint128::zero()) {
            total_staking_data = update_reward_weight_eclipastro(
                total_staking_data,
                pending_eclipastro_reward,
                total_staking,
            );
        }
    }
    // calculate total staking amount with multiplier
    // as eclip reward is changed by the lock duration
    let total_staking_with_multiplier =
        total_staking_data
            .staking_data
            .iter()
            .fold(Uint128::zero(), |acc, cur| {
                let multiplier: u64 = config
                    .locking_reward_config
                    .iter()
                    .find(|c| c.duration == cur.duration)
                    .unwrap_or_default()
                    .multiplier;
                acc.checked_add(cur.amount.checked_mul(Uint128::from(multiplier)).unwrap())
                    .unwrap()
            });
    // update ECLIP reward weight
    // new_weight = old_weight + (current_time - last_update_time) * reward_per_day / total_staking_with_multiplier
    if total_staking_with_multiplier.gt(&Uint128::zero()) {
        let pending_reward = config
            .eclip_daily_reward
            .multiply_ratio(current_time - last_update_time, 86400u64);
        if pending_reward.gt(&Uint128::zero()) {
            total_staking_data = update_reward_weight_eclip(
                total_staking_data,
                pending_reward,
                total_staking_with_multiplier,
            );
        }
    }
    Ok(total_staking_data)
}

pub fn total_staking_amount_update(
    mut total_staking_data: TotalStakingData,
    duration: u64,
    amount: Uint128,
    is_add: bool,
) -> Result<TotalStakingData, ContractError> {
    let mut is_duration_contained = false;
    if is_add {
        total_staking_data.staking_data = total_staking_data
            .staking_data
            .into_iter()
            .map(|d| {
                if d.duration == duration {
                    is_duration_contained = true;
                    Ok(StakingData {
                        duration,
                        amount: d.amount.checked_add(amount).unwrap(),
                    })
                } else {
                    Ok(d)
                }
            })
            .collect::<StdResult<Vec<StakingData>>>()?;
        if is_duration_contained == false {
            total_staking_data.staking_data.push(StakingData {
                duration,
                amount: amount,
            });
        }
    } else {
        total_staking_data.staking_data = total_staking_data
            .staking_data
            .into_iter()
            .map(|d| {
                if d.duration == duration {
                    is_duration_contained = true;
                    Ok(StakingData {
                        duration,
                        amount: d.amount.checked_sub(amount).unwrap(),
                    })
                } else {
                    Ok(d)
                }
            })
            .collect::<StdResult<Vec<StakingData>>>()?;
        ensure_eq!(
            is_duration_contained,
            true,
            ContractError::NotEnoughBalance {}
        );
    }
    Ok(total_staking_data)
}

pub fn user_reward_update(
    deps: Deps,
    total_staking_data: &TotalStakingData,
    duration: u64,
    user: &UserStakingData,
) -> StdResult<UserStakingData> {
    let config = CONFIG.load(deps.storage)?;
    let mut user = user.clone();
    let eclip_reward_multiplier = config
        .locking_reward_config
        .iter()
        .find(|c| c.duration == duration)
        .unwrap_or_default()
        .multiplier;

    // update user eclipASTRO reward info
    // new_pending_eclipASTRO_reward = old_pending_eclipastro_reward + (current_total_staking_reward_weight - last_user_claimed_reward_weight) * user_staking
    // last_user_claimed_reward_weight = current_total_staking_reward_weight
    if user.amount.gt(&Uint128::zero()) {
        user.rewards.eclipastro.pending_reward = user
            .rewards
            .eclipastro
            .pending_reward
            .checked_add(
                total_staking_data
                    .reward_weight_eclipastro
                    .checked_sub(user.rewards.eclipastro.reward_weight)
                    .unwrap()
                    .checked_mul(Decimal256::from_ratio(
                        user.amount,
                        REWARD_WEIGHT_MULTIPLIER,
                    ))
                    .unwrap()
                    .to_uint_floor()
                    .try_into()?,
            )
            .unwrap();
    }
    // update user ECLIP reward info
    // same as eclipASTRO reward calculation method
    if user
        .amount
        .checked_mul(eclip_reward_multiplier.into())
        .unwrap()
        .gt(&Uint128::zero())
    {
        user.rewards.eclip.pending_reward = user
            .rewards
            .eclip
            .pending_reward
            .checked_add(
                total_staking_data
                    .reward_weight_eclip
                    .checked_sub(user.rewards.eclip.reward_weight)
                    .unwrap()
                    .checked_mul(Decimal256::from_ratio(
                        user.amount
                            .checked_mul(eclip_reward_multiplier.into())
                            .unwrap(),
                        REWARD_WEIGHT_MULTIPLIER,
                    ))
                    .unwrap()
                    .to_uint_floor()
                    .try_into()?,
            )
            .unwrap();
    }
    user.rewards.eclipastro.reward_weight = total_staking_data.reward_weight_eclipastro;
    user.rewards.eclip.reward_weight = total_staking_data.reward_weight_eclip;
    Ok(user)
}

pub fn calculate_eclipastro_reward(deps: Deps, time: u64) -> Uint128 {
    let last_update_time = LAST_UPDATE_TIME.load(deps.storage).unwrap_or(time);
    if last_update_time == time {
        return Uint128::zero();
    }
    let start_bound = Some(Bound::exclusive(
        last_update_time - REWARD_DISTRIBUTION_PERIOD,
    ));
    let mut pending_rewards = Uint128::zero();
    let keys = PENDING_REWARDS
        .keys(deps.storage, start_bound, None, Order::Ascending)
        .collect::<StdResult<Vec<u64>>>()
        .unwrap_or(vec![]);
    for k in keys.into_iter() {
        let pending_reward = PENDING_REWARDS
            .load(deps.storage, k)
            .unwrap_or(Uint128::zero());
        let end_time = min(k + REWARD_DISTRIBUTION_PERIOD, time);
        pending_rewards = pending_rewards
            .checked_add(
                pending_reward.multiply_ratio(end_time - last_update_time, REWARD_DISTRIBUTION_PERIOD),
            )
            .unwrap();
    }
    pending_rewards
}

pub fn update_reward_weight_eclipastro(
    mut total_staking_data: TotalStakingData,
    pending_eclipastro_reward: Uint128,
    total_staking: Uint128,
) -> TotalStakingData {
    total_staking_data.reward_weight_eclipastro = total_staking_data
        .reward_weight_eclipastro
        .checked_add(Decimal256::from_ratio(
            pending_eclipastro_reward
                .checked_mul(Uint128::from(REWARD_WEIGHT_MULTIPLIER))
                .unwrap(),
            total_staking,
        ))
        .unwrap();
    total_staking_data
}

pub fn update_reward_weight_eclip(
    mut total_staking_data: TotalStakingData,
    pending_eclip_reward: Uint128,
    total_staking: Uint128,
) -> TotalStakingData {
    total_staking_data.reward_weight_eclip = total_staking_data
        .reward_weight_eclip
        .checked_add(Decimal256::from_ratio(
            pending_eclip_reward
                .checked_mul(Uint128::from(REWARD_WEIGHT_MULTIPLIER))
                .unwrap(),
            total_staking,
        ))
        .unwrap();
    total_staking_data
}
