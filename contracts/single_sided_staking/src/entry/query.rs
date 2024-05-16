use cosmwasm_std::{Addr, Deps, Env, Order, StdResult, Uint128};

use crate::state::{CONFIG, OWNER, STAKING, TOTAL_STAKING, TOTAL_STAKING_BY_DURATION};
use equinox_msg::{
    reward_distributor::{
        QueryMsg as RewardDistributorQueryMsg, TimelockReward, UserRewardResponse,
    },
    timelock_staking::{Config, StakingWithDuration, UserStaking, UserStakingByDuration},
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
        let staking = STAKING
            .prefix((&user, duration))
            .range(deps.storage, None, None, Order::Ascending)
            .map(|s| {
                let (locked_at, amount) = s?;
                Ok(UserStakingByDuration { locked_at, amount })
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
pub fn query_reward(deps: Deps, _env: Env, user: String) -> StdResult<Vec<TimelockReward>> {
    let config = CONFIG.load(deps.storage)?;
    let user_reward: UserRewardResponse = deps.querier.query_wasm_smart(
        config.reward_contract.to_string(),
        &RewardDistributorQueryMsg::Reward { user },
    )?;
    Ok(user_reward.timelock)
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
