use cosmwasm_std::{ensure_eq, Addr, Decimal, Deps, Env, Order, StdResult, Uint128};
use equinox_msg::{
    reward_distributor::{Config, FlexibleReward, TimelockReward, UserRewardResponse},
    token_converter::{QueryMsg as ConverterQueryMsg, RewardResponse},
};

use crate::{
    error::ContractError,
    state::{
        StakingData, TotalStakingData, UserStakingData, CONFIG, FLEXIBLE_USER_STAKING,
        LAST_UPDATE_TIME, OWNER, TIMELOCK_USER_STAKING, TOTAL_STAKING,
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
    let config = CONFIG.load(deps.storage)?;
    let pending_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    let total_staking_data = total_staking_reward_update(deps, env, &pending_reward)?;
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

pub fn total_staking_reward_update(
    deps: Deps,
    env: Env,
    pending_reward: &RewardResponse,
) -> StdResult<TotalStakingData> {
    let config = CONFIG.load(deps.storage)?;
    let mut total_staking_data = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    let last_update_time = LAST_UPDATE_TIME.load(deps.storage).unwrap_or_default();
    if pending_reward.users_reward.amount.gt(&Uint128::zero()) {
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
            total_staking_data.reward_weight_eclipastro = total_staking_data
                .reward_weight_eclipastro
                .checked_add(Decimal::from_ratio(
                    pending_reward.users_reward.amount,
                    total_staking,
                ))
                .unwrap();
        }
    }
    // calculate total staking amount with multiplier
    // as eclip reward is changed by the lock duration
    let total_staking_with_multiplier =
        total_staking_data
            .staking_data
            .iter()
            .fold(Uint128::zero(), |acc, cur| {
                let multiplier = config
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
            .multiply_ratio(env.block.time.seconds() - last_update_time, 86400u64);
        total_staking_data.reward_weight_eclip = total_staking_data
            .reward_weight_eclip
            .checked_add(Decimal::from_ratio(
                pending_reward,
                total_staking_with_multiplier,
            ))
            .unwrap();
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
                    .checked_mul(Decimal::from_ratio(user.amount, Uint128::one()))
                    .unwrap()
                    .to_uint_floor(),
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
                    .checked_mul(Decimal::from_ratio(
                        user.amount
                            .checked_mul(eclip_reward_multiplier.into())
                            .unwrap(),
                        Uint128::one(),
                    ))
                    .unwrap()
                    .to_uint_floor(),
            )
            .unwrap();
    }
    user.rewards.eclipastro.reward_weight = total_staking_data.reward_weight_eclipastro;
    user.rewards.eclip.reward_weight = total_staking_data.reward_weight_eclip;
    Ok(user)
}
