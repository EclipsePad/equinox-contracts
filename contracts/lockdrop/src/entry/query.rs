use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::{Addr, Decimal256, Deps, Env, Order, StdResult, Uint128, Uint256};
use equinox_msg::{
    lockdrop::{
        Config, DetailedLpLockupInfo, DetailedSingleLockupInfo, IncentiveAmounts,
        LockdropIncentive, LockdropIncentives, LpLockupInfoResponse, LpLockupStateResponse,
        LpStakingRewardWeights, LpStakingRewards, LpUserLockupInfo, RewardDistributionConfig,
        SingleLockupInfoResponse, SingleLockupStateResponse, SingleStakingRewardWeights,
        SingleStakingRewards, SingleStakingRewardsByDuration, SingleUserLockupInfo, StakeType,
        UserLpLockupInfoResponse, UserSingleLockupInfoResponse,
    },
    lp_staking::{QueryMsg as LpStakingQueryMsg, RewardAmount},
    single_sided_staking::{QueryMsg as SingleSidedQueryMsg, UserRewardByDuration},
};

use crate::{
    error::ContractError,
    state::{
        CONFIG, LP_LOCKDROP_INCENTIVES, LP_LOCKUP_INFO, LP_LOCKUP_STATE, LP_STAKING_REWARD_WEIGHTS,
        LP_USER_LOCKUP_INFO, OWNER, REWARD_DISTRIBUTION_CONFIG, SINGLE_LOCKDROP_INCENTIVES,
        SINGLE_LOCKUP_INFO, SINGLE_LOCKUP_STATE, SINGLE_STAKING_REWARD_WEIGHTS,
        SINGLE_USER_LOCKUP_INFO,
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

/// query config
pub fn query_reward_config(deps: Deps, _env: Env) -> StdResult<RewardDistributionConfig> {
    let config = REWARD_DISTRIBUTION_CONFIG.load(deps.storage)?;
    Ok(config)
}

/// query eclipASTRO Lockdrop info
pub fn query_single_lockup_info(deps: Deps, env: Env) -> StdResult<SingleLockupInfoResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let single_staking_rewards =
        calculate_single_sided_total_rewards(deps, env.contract.address.to_string())?;
    let single_lockups = SINGLE_LOCKUP_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| {
            let (duration, lockup_info) = r.unwrap();
            let reward_weights = SINGLE_STAKING_REWARD_WEIGHTS
                .load(deps.storage, duration)
                .unwrap_or_default();
            let lock_config = cfg
                .lock_configs
                .iter()
                .find(|c| c.duration == duration)
                .unwrap();
            DetailedSingleLockupInfo {
                duration,
                xastro_amount_in_lockups: lockup_info.xastro_amount_in_lockups,
                total_eclipastro_staked: lockup_info.total_staked,
                total_eclipastro_withdrawed: lockup_info.total_withdrawed,
                reward_multiplier: lock_config.multiplier,
                reward_weights,
            }
        })
        .collect::<Vec<DetailedSingleLockupInfo>>();
    Ok(SingleLockupInfoResponse {
        single_lockups,
        pending_rewards: single_staking_rewards,
    })
}

/// query eclipASTRO/xASTRO Lp token Lockdrop info
pub fn query_lp_lockup_info(deps: Deps, env: Env) -> StdResult<LpLockupInfoResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let lp_staking_rewards = calculate_lp_total_rewards(deps, env.contract.address.to_string())?;
    let reward_weights = LP_STAKING_REWARD_WEIGHTS
        .load(deps.storage)
        .unwrap_or_default();
    let lp_lockups = LP_LOCKUP_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| {
            let (duration, lockup_info) = r.unwrap();
            let lock_config = cfg
                .lock_configs
                .iter()
                .find(|c| c.duration == duration)
                .unwrap();
            DetailedLpLockupInfo {
                duration,
                xastro_amount_in_lockups: lockup_info.xastro_amount_in_lockups,
                total_lp_staked: lockup_info.total_staked,
                total_lp_withdrawed: lockup_info.total_withdrawed,
                reward_multiplier: lock_config.multiplier,
            }
        })
        .collect::<Vec<DetailedLpLockupInfo>>();
    Ok(LpLockupInfoResponse {
        lp_lockups,
        pending_rewards: lp_staking_rewards,
        reward_weights,
    })
}

/// query eclipASTRO lockup state
pub fn query_single_lockup_state(deps: Deps, _env: Env) -> StdResult<SingleLockupStateResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    Ok(SingleLockupStateResponse {
        are_claims_allowed: cfg.claims_allowed,
        countdown_start_at: cfg.countdown_start_at,
        total_eclipastro_lockup: state.total_eclipastro_lockup,
    })
}

/// query eclipASTRO/xASTRO lp token lockup state
pub fn query_lp_lockup_state(deps: Deps, _env: Env) -> StdResult<LpLockupStateResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let state = LP_LOCKUP_STATE.load(deps.storage)?;
    Ok(LpLockupStateResponse {
        are_claims_allowed: cfg.claims_allowed,
        countdown_start_at: cfg.countdown_start_at,
        total_lp_lockdrop: state.total_lp_lockdrop,
    })
}

/// query eclipASTRO user lockup info
pub fn query_user_single_lockup_info(
    deps: Deps,
    env: Env,
    user_address: String,
) -> StdResult<Vec<UserSingleLockupInfoResponse>> {
    let cfg = CONFIG.load(deps.storage)?;
    let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;

    if cfg.claims_allowed {
        let single_staking_rewards =
            calculate_single_sided_total_rewards(deps, env.contract.address.to_string())?;
        Ok(SINGLE_USER_LOCKUP_INFO
            .prefix(&user_address)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|r| {
                let (duration, mut user_lockup_info) = r.unwrap();
                if user_lockup_info.total_eclipastro_staked.is_zero() {
                    user_lockup_info.total_eclipastro_staked = user_lockup_info
                        .xastro_amount_in_lockups
                        .multiply_ratio(state.total_eclipastro_lockup, state.total_xastro);
                }
                user_lockup_info.lockdrop_incentives = get_user_single_lockdrop_incentives(
                    deps,
                    user_lockup_info.lockdrop_incentives,
                    user_lockup_info.xastro_amount_in_lockups,
                    duration,
                )
                .unwrap();
                let pending_lockdrop_incentives = calculate_pending_lockdrop_incentives(
                    deps,
                    env.block.time.seconds(),
                    user_lockup_info.lockdrop_incentives.clone(),
                )
                .unwrap();
                let updated_reward_weights = calculate_updated_single_staking_reward_weights(
                    deps,
                    single_staking_rewards
                        .iter()
                        .find(|r| r.duration == duration)
                        .unwrap(),
                )
                .unwrap();
                let user_rewards = calculate_single_staking_user_rewards(
                    deps,
                    updated_reward_weights,
                    pending_lockdrop_incentives,
                    user_lockup_info.clone(),
                )
                .unwrap();
                UserSingleLockupInfoResponse {
                    duration,
                    xastro_amount_in_lockups: user_lockup_info.xastro_amount_in_lockups,
                    eclipastro_staked: user_lockup_info.total_eclipastro_staked,
                    eclipastro_withdrawed: user_lockup_info.total_eclipastro_withdrawed,
                    lockdrop_incentives: user_lockup_info.lockdrop_incentives,
                    withdrawal_flag: user_lockup_info.withdrawal_flag,
                    staking_rewards: vec![
                        Asset {
                            info: AssetInfo::Token {
                                contract_addr: cfg.eclipastro_token.clone(),
                            },
                            amount: user_rewards.eclipastro,
                        },
                        Asset {
                            info: cfg.beclip.clone(),
                            amount: user_rewards.beclip,
                        },
                        Asset {
                            info: cfg.eclip.clone(),
                            amount: user_rewards.eclip,
                        },
                    ],
                    countdown_start_at: cfg.countdown_start_at,
                    reward_weights: user_lockup_info.reward_weights,
                }
            })
            .collect::<Vec<UserSingleLockupInfoResponse>>())
    } else {
        Ok(SINGLE_USER_LOCKUP_INFO
            .prefix(&user_address)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|r| {
                let (duration, mut user_lockup_info) = r.unwrap();
                user_lockup_info.lockdrop_incentives = get_user_single_lockdrop_incentives(
                    deps,
                    user_lockup_info.lockdrop_incentives,
                    user_lockup_info.xastro_amount_in_lockups,
                    duration,
                )
                .unwrap();
                UserSingleLockupInfoResponse {
                    duration,
                    xastro_amount_in_lockups: user_lockup_info.xastro_amount_in_lockups,
                    eclipastro_staked: Uint128::zero(),
                    eclipastro_withdrawed: Uint128::zero(),
                    withdrawal_flag: user_lockup_info.withdrawal_flag,
                    lockdrop_incentives: user_lockup_info.lockdrop_incentives,
                    staking_rewards: vec![],
                    countdown_start_at: cfg.countdown_start_at,
                    reward_weights: user_lockup_info.reward_weights,
                }
            })
            .collect::<Vec<UserSingleLockupInfoResponse>>())
    }
}

/// query lp token user lockup info
pub fn query_user_lp_lockup_info(
    deps: Deps,
    env: Env,
    user_address: String,
) -> StdResult<Vec<UserLpLockupInfoResponse>> {
    let cfg = CONFIG.load(deps.storage)?;
    let state = LP_LOCKUP_STATE.load(deps.storage)?;

    if cfg.claims_allowed {
        let lp_total_rewards = calculate_lp_total_rewards(deps, env.contract.address.to_string())?;
        Ok(LP_USER_LOCKUP_INFO
            .prefix(&user_address)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|r| {
                let (duration, mut user_lockup_info) = r.unwrap();
                if user_lockup_info.total_lp_staked.is_zero() {
                    user_lockup_info.total_lp_staked = user_lockup_info
                        .xastro_amount_in_lockups
                        .multiply_ratio(state.total_lp_lockdrop, state.total_xastro);
                }
                user_lockup_info.lockdrop_incentives = get_user_lp_lockdrop_incentives(
                    deps,
                    user_lockup_info.lockdrop_incentives,
                    user_lockup_info.xastro_amount_in_lockups,
                    duration,
                )
                .unwrap();
                let pending_lockdrop_incentives = calculate_pending_lockdrop_incentives(
                    deps,
                    env.block.time.seconds(),
                    user_lockup_info.lockdrop_incentives.clone(),
                )
                .unwrap();
                let updated_reward_weights =
                    calculate_updated_lp_reward_weights(deps, &lp_total_rewards).unwrap();
                let user_rewards = calculate_lp_staking_user_rewards(
                    deps,
                    updated_reward_weights,
                    pending_lockdrop_incentives,
                    user_lockup_info.clone(),
                )
                .unwrap();
                UserLpLockupInfoResponse {
                    duration,
                    xastro_amount_in_lockups: user_lockup_info.xastro_amount_in_lockups,
                    lp_token_staked: user_lockup_info.total_lp_staked,
                    lp_token_withdrawed: user_lockup_info.total_lp_withdrawed,
                    lockdrop_incentives: user_lockup_info.lockdrop_incentives,
                    withdrawal_flag: user_lockup_info.withdrawal_flag,
                    staking_rewards: vec![
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: cfg.astro_token.clone(),
                            },
                            amount: user_rewards.astro,
                        },
                        Asset {
                            info: cfg.beclip.clone(),
                            amount: user_rewards.beclip,
                        },
                        Asset {
                            info: cfg.eclip.clone(),
                            amount: user_rewards.eclip,
                        },
                    ],
                    countdown_start_at: cfg.countdown_start_at,
                    reward_weights: user_lockup_info.reward_weights,
                }
            })
            .collect::<Vec<UserLpLockupInfoResponse>>())
    } else {
        Ok(LP_USER_LOCKUP_INFO
            .prefix(&user_address.to_string())
            .range(deps.storage, None, None, Order::Ascending)
            .map(|r| {
                let (duration, user_lockup_info) = r.unwrap();
                UserLpLockupInfoResponse {
                    duration,
                    xastro_amount_in_lockups: user_lockup_info.xastro_amount_in_lockups,
                    lp_token_staked: Uint128::zero(),
                    lp_token_withdrawed: Uint128::zero(),
                    withdrawal_flag: user_lockup_info.withdrawal_flag,
                    lockdrop_incentives: get_user_lp_lockdrop_incentives(
                        deps,
                        user_lockup_info.lockdrop_incentives,
                        user_lockup_info.xastro_amount_in_lockups,
                        duration,
                    )
                    .unwrap(),
                    staking_rewards: vec![],
                    countdown_start_at: cfg.countdown_start_at,
                    reward_weights: user_lockup_info.reward_weights,
                }
            })
            .collect::<Vec<UserLpLockupInfoResponse>>())
    }
}

pub fn query_incentives(deps: Deps, stake_type: StakeType) -> StdResult<IncentiveAmounts> {
    if stake_type == StakeType::SingleStaking {
        return Ok(SINGLE_LOCKDROP_INCENTIVES
            .load(deps.storage)
            .unwrap_or_default());
    }
    Ok(LP_LOCKDROP_INCENTIVES
        .load(deps.storage)
        .unwrap_or_default())
}

pub fn calculate_single_sided_total_rewards(
    deps: Deps,
    user: String,
) -> StdResult<Vec<SingleStakingRewardsByDuration>> {
    let cfg = CONFIG.load(deps.storage)?;
    let rewards: Vec<UserRewardByDuration> = deps.querier.query_wasm_smart(
        cfg.single_sided_staking.to_string(),
        &SingleSidedQueryMsg::Reward { user },
    )?;
    let mut single_staking_rewards = vec![];
    for reward_by_duration in rewards {
        let duration = reward_by_duration.duration;
        let mut eclipastro_reward = Uint128::zero();
        let mut beclip_reward = Uint128::zero();
        let mut eclip_reward = Uint128::zero();
        for reward_by_locked_at in reward_by_duration.rewards {
            eclipastro_reward += reward_by_locked_at.rewards.eclipastro;
            beclip_reward += reward_by_locked_at.rewards.beclip;
            eclip_reward += reward_by_locked_at.rewards.eclip;
        }
        single_staking_rewards.push(SingleStakingRewardsByDuration {
            duration,
            rewards: SingleStakingRewards {
                eclipastro: eclipastro_reward,
                beclip: beclip_reward,
                eclip: eclip_reward,
            },
        })
    }
    Ok(single_staking_rewards)
}

pub fn calculate_updated_single_staking_reward_weights(
    deps: Deps,
    rewards_by_duration: &SingleStakingRewardsByDuration,
) -> Result<SingleStakingRewardWeights, ContractError> {
    let duration = rewards_by_duration.duration;
    let mut reward_weights = SINGLE_STAKING_REWARD_WEIGHTS
        .load(deps.storage, duration)
        .unwrap_or_default();
    let lockup_info = SINGLE_LOCKUP_INFO
        .load(deps.storage, duration)
        .unwrap_or_default();
    if lockup_info.total_staked - lockup_info.total_withdrawed == Uint128::zero() {
        return Ok(reward_weights);
    }
    reward_weights.eclipastro += Decimal256::from_ratio(
        rewards_by_duration.rewards.eclipastro,
        lockup_info.total_staked - lockup_info.total_withdrawed,
    );
    reward_weights.beclip += Decimal256::from_ratio(
        rewards_by_duration.rewards.beclip,
        lockup_info.total_staked - lockup_info.total_withdrawed,
    );
    reward_weights.eclip += Decimal256::from_ratio(
        rewards_by_duration.rewards.eclip,
        lockup_info.total_staked - lockup_info.total_withdrawed,
    );
    Ok(reward_weights)
}

pub fn calculate_pending_lockdrop_incentives(
    deps: Deps,
    current_time: u64,
    incentives: LockdropIncentives,
) -> StdResult<IncentiveAmounts> {
    Ok(IncentiveAmounts {
        eclip: calculate_pending_lockdrop_incentive(deps, current_time, incentives.eclip)?,
        beclip: calculate_pending_lockdrop_incentive(deps, current_time, incentives.beclip)?,
    })
}

pub fn calculate_pending_lockdrop_incentive(
    deps: Deps,
    current_time: u64,
    incentive: LockdropIncentive,
) -> StdResult<Uint128> {
    let cfg = CONFIG.load(deps.storage)?;
    let reward_cfg = REWARD_DISTRIBUTION_CONFIG.load(deps.storage)?;
    if !cfg.claims_allowed {
        return Ok(Uint128::zero());
    }

    let instant_amount = incentive
        .allocated
        .multiply_ratio(reward_cfg.instant, 10000u64);
    let vesting_amount = incentive.allocated - instant_amount;
    let max_allowed_to_claim = if current_time >= cfg.countdown_start_at + reward_cfg.vesting_period
    {
        incentive.allocated
    } else {
        instant_amount
            .checked_add(vesting_amount.multiply_ratio(
                current_time - cfg.countdown_start_at,
                reward_cfg.vesting_period,
            ))
            .unwrap()
    };
    let claimable_amount = max_allowed_to_claim
        .checked_sub(incentive.claimed)
        .unwrap_or_default();
    Ok(claimable_amount)
}

pub fn get_user_single_lockdrop_incentives(
    deps: Deps,
    lockdrop_incentives: LockdropIncentives,
    xastro_amount_in_lockups: Uint128,
    duration: u64,
) -> Result<LockdropIncentives, ContractError> {
    if !lockdrop_incentives.eclip.allocated.is_zero()
        || !lockdrop_incentives.beclip.allocated.is_zero()
    {
        return Ok(lockdrop_incentives);
    }
    let cfg = CONFIG.load(deps.storage)?;
    let lock_configs = cfg.lock_configs.clone();
    let single_lockdrop_incentives = SINGLE_LOCKDROP_INCENTIVES
        .load(deps.storage)
        .unwrap_or_default();
    let single_sided_state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let duration_multiplier = cfg
        .lock_configs
        .into_iter()
        .find(|c| c.duration == duration)
        .unwrap_or_default()
        .multiplier;
    let lockdrop_incentives = if cfg.claims_allowed {
        LockdropIncentives {
            eclip: LockdropIncentive {
                allocated: Uint256::from(xastro_amount_in_lockups)
                    .multiply_ratio(duration_multiplier, 10000u128)
                    .multiply_ratio(
                        single_lockdrop_incentives.eclip,
                        single_sided_state.weighted_total_xastro,
                    )
                    .try_into()
                    .unwrap_or_default(),
                claimed: Uint128::zero(),
            },
            beclip: LockdropIncentive {
                allocated: Uint256::from(xastro_amount_in_lockups)
                    .multiply_ratio(duration_multiplier, 10000u128)
                    .multiply_ratio(
                        single_lockdrop_incentives.beclip,
                        single_sided_state.weighted_total_xastro,
                    )
                    .try_into()
                    .unwrap_or_default(),
                claimed: Uint128::zero(),
            },
        }
    } else {
        let total_weighted_xastro_amount_for_single_staking = SINGLE_LOCKUP_INFO
            .range(deps.storage, None, None, Order::Ascending)
            .fold(Uint128::zero(), |acc, cur| {
                let (duration, info) = cur.unwrap();
                let duration_multiplier = lock_configs
                    .clone()
                    .into_iter()
                    .find(|c| c.duration == duration)
                    .unwrap_or_default()
                    .multiplier;
                acc.checked_add(
                    info.xastro_amount_in_lockups
                        .multiply_ratio(duration_multiplier, 10000u128),
                )
                .unwrap()
            });
        LockdropIncentives {
            eclip: LockdropIncentive {
                allocated: Uint256::from(xastro_amount_in_lockups)
                    .multiply_ratio(duration_multiplier, 10000u128)
                    .multiply_ratio(
                        single_lockdrop_incentives.eclip,
                        total_weighted_xastro_amount_for_single_staking,
                    )
                    .try_into()
                    .unwrap_or_default(),
                claimed: Uint128::zero(),
            },
            beclip: LockdropIncentive {
                allocated: Uint256::from(xastro_amount_in_lockups)
                    .multiply_ratio(duration_multiplier, 10000u128)
                    .multiply_ratio(
                        single_lockdrop_incentives.beclip,
                        total_weighted_xastro_amount_for_single_staking,
                    )
                    .try_into()
                    .unwrap_or_default(),
                claimed: Uint128::zero(),
            },
        }
    };
    Ok(lockdrop_incentives)
}
pub fn get_user_lp_lockdrop_incentives(
    deps: Deps,
    lockdrop_incentives: LockdropIncentives,
    xastro_amount_in_lockups: Uint128,
    duration: u64,
) -> Result<LockdropIncentives, ContractError> {
    if !lockdrop_incentives.eclip.allocated.is_zero()
        || !lockdrop_incentives.beclip.allocated.is_zero()
    {
        return Ok(lockdrop_incentives);
    }
    let cfg = CONFIG.load(deps.storage)?;
    let lock_configs = cfg.lock_configs.clone();
    let lp_lockdrop_incentives = LP_LOCKDROP_INCENTIVES
        .load(deps.storage)
        .unwrap_or_default();
    let lp_state = LP_LOCKUP_STATE.load(deps.storage)?;
    let duration_multiplier = cfg
        .lock_configs
        .into_iter()
        .find(|c| c.duration == duration)
        .unwrap_or_default()
        .multiplier;
    let lockdrop_incentives = if cfg.claims_allowed {
        LockdropIncentives {
            eclip: LockdropIncentive {
                allocated: Uint256::from(xastro_amount_in_lockups)
                    .multiply_ratio(duration_multiplier, 10000u128)
                    .multiply_ratio(lp_lockdrop_incentives.eclip, lp_state.weighted_total_xastro)
                    .try_into()
                    .unwrap_or_default(),
                claimed: Uint128::zero(),
            },
            beclip: LockdropIncentive {
                allocated: Uint256::from(xastro_amount_in_lockups)
                    .multiply_ratio(duration_multiplier, 10000u128)
                    .multiply_ratio(
                        lp_lockdrop_incentives.beclip,
                        lp_state.weighted_total_xastro,
                    )
                    .try_into()
                    .unwrap_or_default(),
                claimed: Uint128::zero(),
            },
        }
    } else {
        let total_weighted_xastro_amount_for_lp_staking = LP_LOCKUP_INFO
            .range(deps.storage, None, None, Order::Ascending)
            .fold(Uint128::zero(), |acc, cur| {
                let (duration, info) = cur.unwrap();
                let duration_multiplier = lock_configs
                    .clone()
                    .into_iter()
                    .find(|c| c.duration == duration)
                    .unwrap_or_default()
                    .multiplier;
                acc.checked_add(
                    info.xastro_amount_in_lockups
                        .multiply_ratio(duration_multiplier, 10000u128),
                )
                .unwrap()
            });
        LockdropIncentives {
            eclip: LockdropIncentive {
                allocated: Uint256::from(xastro_amount_in_lockups)
                    .multiply_ratio(duration_multiplier, 10000u128)
                    .multiply_ratio(
                        lp_lockdrop_incentives.eclip,
                        total_weighted_xastro_amount_for_lp_staking,
                    )
                    .try_into()
                    .unwrap_or_default(),
                claimed: Uint128::zero(),
            },
            beclip: LockdropIncentive {
                allocated: Uint256::from(xastro_amount_in_lockups)
                    .multiply_ratio(duration_multiplier, 10000u128)
                    .multiply_ratio(
                        lp_lockdrop_incentives.beclip,
                        total_weighted_xastro_amount_for_lp_staking,
                    )
                    .try_into()
                    .unwrap_or_default(),
                claimed: Uint128::zero(),
            },
        }
    };
    Ok(lockdrop_incentives)
}

pub fn calculate_single_staking_user_rewards(
    _deps: Deps,
    reward_weights: SingleStakingRewardWeights,
    pending_lockdrop_incentives: IncentiveAmounts,
    user_lockup_info: SingleUserLockupInfo,
) -> StdResult<SingleStakingRewards> {
    let eclipastro_reward: Uint128 = reward_weights
        .eclipastro
        .checked_sub(user_lockup_info.reward_weights.eclipastro)
        .unwrap_or_default()
        .checked_mul(Decimal256::from_ratio(
            user_lockup_info.total_eclipastro_staked - user_lockup_info.total_eclipastro_withdrawed,
            1u128,
        ))
        .unwrap()
        .to_uint_floor()
        .try_into()?;
    let mut beclip_reward: Uint128 = reward_weights
        .beclip
        .checked_sub(user_lockup_info.reward_weights.beclip)
        .unwrap_or_default()
        .checked_mul(Decimal256::from_ratio(
            user_lockup_info.total_eclipastro_staked - user_lockup_info.total_eclipastro_withdrawed,
            1u128,
        ))
        .unwrap()
        .to_uint_floor()
        .try_into()?;
    beclip_reward += pending_lockdrop_incentives.beclip;
    let mut eclip_reward: Uint128 = reward_weights
        .eclip
        .checked_sub(user_lockup_info.reward_weights.eclip)
        .unwrap_or_default()
        .checked_mul(Decimal256::from_ratio(
            user_lockup_info.total_eclipastro_staked - user_lockup_info.total_eclipastro_withdrawed,
            1u128,
        ))
        .unwrap()
        .to_uint_floor()
        .try_into()?;
    eclip_reward += pending_lockdrop_incentives.eclip;
    Ok(SingleStakingRewards {
        eclipastro: eclipastro_reward,
        eclip: beclip_reward,
        beclip: beclip_reward,
    })
}

pub fn calculate_lp_total_rewards(deps: Deps, user: String) -> StdResult<LpStakingRewards> {
    let cfg = CONFIG.load(deps.storage)?;
    let rewards: Vec<RewardAmount> = deps.querier.query_wasm_smart(
        cfg.lp_staking.to_string(),
        &LpStakingQueryMsg::Reward { user },
    )?;
    let mut astro_reward = Uint128::zero();
    let mut beclip_reward = Uint128::zero();
    let mut eclip_reward = Uint128::zero();
    for user_reward in rewards {
        if user_reward.info.to_string() == cfg.astro_token {
            astro_reward = user_reward.amount;
        }
        if user_reward.info == cfg.beclip {
            beclip_reward = user_reward.amount;
        }
        if user_reward.info == cfg.eclip {
            eclip_reward = user_reward.amount;
        }
    }
    Ok(LpStakingRewards {
        astro: astro_reward,
        beclip: beclip_reward,
        eclip: eclip_reward,
    })
}

pub fn calculate_updated_lp_reward_weights(
    deps: Deps,
    lp_rewards: &LpStakingRewards,
) -> Result<LpStakingRewardWeights, ContractError> {
    let mut reward_weights = LP_STAKING_REWARD_WEIGHTS
        .load(deps.storage)
        .unwrap_or_default();
    let total_staking = LP_LOCKUP_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .fold(Uint128::zero(), |acc, r| {
            let (_, lockup_info) = r.unwrap();
            acc + lockup_info.total_staked - lockup_info.total_withdrawed
        });
    if total_staking.is_zero() {
        return Ok(reward_weights);
    }
    reward_weights.astro += Decimal256::from_ratio(lp_rewards.astro, total_staking);
    reward_weights.beclip += Decimal256::from_ratio(lp_rewards.beclip, total_staking);
    reward_weights.eclip += Decimal256::from_ratio(lp_rewards.eclip, total_staking);
    Ok(reward_weights)
}

pub fn calculate_lp_staking_user_rewards(
    _deps: Deps,
    reward_weights: LpStakingRewardWeights,
    pending_lockdrop_incentives: IncentiveAmounts,
    user_lockup_info: LpUserLockupInfo,
) -> StdResult<LpStakingRewards> {
    let astro_reward: Uint128 = reward_weights
        .astro
        .checked_sub(user_lockup_info.reward_weights.astro)
        .unwrap_or_default()
        .checked_mul(Decimal256::from_ratio(
            user_lockup_info.total_lp_staked - user_lockup_info.total_lp_withdrawed,
            1u128,
        ))
        .unwrap_or_default()
        .to_uint_floor()
        .try_into()?;
    let mut beclip_reward: Uint128 = reward_weights
        .beclip
        .checked_sub(user_lockup_info.reward_weights.beclip)
        .unwrap_or_default()
        .checked_mul(Decimal256::from_ratio(
            user_lockup_info.total_lp_staked - user_lockup_info.total_lp_withdrawed,
            1u128,
        ))
        .unwrap_or_default()
        .to_uint_floor()
        .try_into()?;
    beclip_reward += pending_lockdrop_incentives.beclip;
    let mut eclip_reward: Uint128 = reward_weights
        .eclip
        .checked_sub(user_lockup_info.reward_weights.eclip)
        .unwrap_or_default()
        .checked_mul(Decimal256::from_ratio(
            user_lockup_info.total_lp_staked - user_lockup_info.total_lp_withdrawed,
            1u128,
        ))
        .unwrap_or_default()
        .to_uint_floor()
        .try_into()?;
    eclip_reward += pending_lockdrop_incentives.eclip;
    Ok(LpStakingRewards {
        astro: astro_reward,
        beclip: beclip_reward,
        eclip: eclip_reward,
    })
}
