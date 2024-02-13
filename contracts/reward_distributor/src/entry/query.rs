use cosmwasm_std::{ensure_eq, Addr, Decimal, Deps, Env, StdResult, Uint128};
use equinox_msg::{reward_distributor::{Config, UserRewardResponse}, token_converter::{QueryMsg as ConverterQueryMsg, RewardResponse}};

use crate::{error::ContractError, state::{StakingData, TotalStakingData, UserRewards, CONFIG, LAST_UPDATE_TIME, OWNER, TOTAL_STAKING, USER_REWARDS, USER_STAKING}};


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
    let user_rewards = user_reward_update(deps, &total_staking_data, &user)?;
    Ok(UserRewardResponse {
        eclip: user_rewards.eclip.pending_reward,
        eclipastro: user_rewards.eclipastro.pending_reward,
    })
}

pub fn total_staking_reward_update(deps: Deps, env: Env) -> StdResult<TotalStakingData> {
    let config = CONFIG.load(deps.storage)?;
    let mut total_staking_data = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    let last_update_time = LAST_UPDATE_TIME.load(deps.storage).unwrap_or_default();
    let pending_eclipastro_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    // increase total_staking amount, calculate reward weight of eclipASTRO, update total_staking_data
    let total_staking = total_staking_data
        .staking_data
        .iter()
        .fold(Uint128::zero(), |acc, cur| {
            acc.checked_add(cur.amount).unwrap()
        });
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
    if total_staking.gt(&Uint128::zero()) {
        total_staking_data.reward_weight_eclipastro = total_staking_data
            .reward_weight_eclipastro
            .checked_add(Decimal::from_ratio(
                pending_eclipastro_reward.users_reward.amount,
                total_staking,
            ))
            .unwrap();
    }
    if total_staking_with_multiplier.gt(&Uint128::zero()) {
        let pending_reward = config
            .eclip_daily_reward
            .multiply_ratio(env.block.time.seconds() - last_update_time, 86400u64);
        total_staking_data.reward_weight_astro = total_staking_data
            .reward_weight_astro
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
    user: &String,
) -> StdResult<UserRewards> {
    let config = CONFIG.load(deps.storage)?;
    let user_staking = USER_STAKING.load(deps.storage, &user).unwrap_or(vec![]);
    let mut user_rewards = USER_REWARDS.load(deps.storage, &user).unwrap_or_default();
    // calculate user pending reward and update reward weight to current weight
    let total_user_staking = user_staking.iter().fold(Uint128::zero(), |acc, cur| {
        acc.checked_add(cur.amount).unwrap()
    });
    let total_user_staking_with_multiplier =
        user_staking.iter().fold(Uint128::zero(), |acc, cur| {
            let multiplier = config
                .locking_reward_config
                .iter()
                .find(|c| c.duration == cur.duration)
                .unwrap_or_default()
                .multiplier;
            acc.checked_add(cur.amount.checked_mul(Uint128::from(multiplier)).unwrap())
                .unwrap()
        });
    if total_user_staking.gt(&Uint128::zero()) {
        user_rewards.eclipastro.pending_reward = user_rewards
            .eclipastro
            .pending_reward
            .checked_add(
                total_staking_data
                    .reward_weight_eclipastro
                    .checked_sub(user_rewards.eclipastro.reward_weight)
                    .unwrap()
                    .checked_mul(Decimal::from_ratio(total_user_staking, Uint128::one()))
                    .unwrap()
                    .to_uint_floor(),
            )
            .unwrap();
    }
    if total_user_staking_with_multiplier.gt(&Uint128::zero()) {
        user_rewards.eclip.pending_reward = user_rewards
            .eclip
            .pending_reward
            .checked_add(
                total_staking_data
                    .reward_weight_astro
                    .checked_sub(user_rewards.eclip.reward_weight)
                    .unwrap()
                    .checked_mul(Decimal::from_ratio(
                        total_user_staking_with_multiplier,
                        Uint128::one(),
                    ))
                    .unwrap()
                    .to_uint_floor(),
            )
            .unwrap();
    }
    user_rewards.eclipastro.reward_weight = total_staking_data.reward_weight_eclipastro;
    user_rewards.eclip.reward_weight = total_staking_data.reward_weight_astro;
    Ok(user_rewards)
}

pub fn user_staking_amount_update(
    mut user_staking: Vec<StakingData>,
    duration: u64,
    amount: Uint128,
    is_add: bool,
) -> Result<Vec<StakingData>, ContractError> {
    // update user_staking
    let mut is_duration_contained = false;
    if is_add {
        user_staking = user_staking
            .into_iter()
            .map(|s| {
                if s.duration == duration {
                    is_duration_contained = true;
                    Ok(StakingData {
                        duration: duration,
                        amount: s.amount.checked_add(amount).unwrap(),
                    })
                } else {
                    Ok(s)
                }
            })
            .collect::<StdResult<Vec<StakingData>>>()?;
        if is_duration_contained == false {
            user_staking.push(StakingData {
                duration: duration,
                amount,
            });
        }
    } else {
        user_staking = user_staking
            .into_iter()
            .map(|s| {
                if s.duration == duration {
                    is_duration_contained = true;
                    Ok(StakingData {
                        duration: duration,
                        amount: s.amount.checked_sub(amount).unwrap(),
                    })
                } else {
                    Ok(s)
                }
            })
            .collect::<StdResult<Vec<StakingData>>>()?;
        ensure_eq!(
            is_duration_contained,
            true,
            ContractError::NotEnoughBalance {}
        );
    }
    Ok(user_staking)
}

