use std::{cmp::min, str::FromStr};

use astroport::{
    asset::{Asset, AssetInfo},
    pair::{PoolResponse, QueryMsg as PoolQueryMsg},
    staking::QueryMsg as AstroportStakingQueryMsg,
};
use cosmwasm_std::{
    Addr, BankQuery, Coin, Decimal, Decimal256, Deps, Env, Order, QuerierWrapper, QueryRequest,
    StdResult, SupplyResponse, Uint128, Uint256,
};
use equinox_msg::{
    lockdrop::{
        BlacklistRewards, Config, DetailedLpLockupInfo, DetailedSingleLockupInfo, IncentiveAmounts,
        LockdropIncentive, LockdropIncentives, LpLockupInfoResponse, LpLockupStateResponse,
        LpStakingRewardWeights, LpStakingRewards, LpUserLockupInfo, RewardDistributionConfig,
        SingleLockupInfoResponse, SingleLockupStateResponse, SingleStakingRewardsByDuration,
        StakeType, UserLpLockupInfoResponse, UserSingleLockupInfoResponse,
    },
    lp_staking::{QueryMsg as LpStakingQueryMsg, RewardAmount},
    single_sided_staking::{QueryMsg as SingleSidedQueryMsg, UserReward},
};

use crate::{
    config::{BPS_DENOMINATOR, DEFAULT_LAST_EARLY_UNLOCK_PENALTY},
    error::ContractError,
    state::{
        ADJUST_REWARDS, BLACK_LIST, BLACK_LIST_REWARDS, CONFIG, LP_LOCKDROP_INCENTIVES,
        LP_LOCKUP_INFO, LP_LOCKUP_STATE, LP_STAKING_REWARD_WEIGHTS, LP_USER_LOCKUP_INFO, OWNER,
        REWARD_DISTRIBUTION_CONFIG, SINGLE_LOCKDROP_INCENTIVES, SINGLE_LOCKUP_INFO,
        SINGLE_LOCKUP_STATE, SINGLE_STAKING_REWARD_WEIGHTS, SINGLE_USER_LOCKUP_INFO,
    },
};

/// query owner
pub fn query_owner(deps: Deps, _env: Env) -> StdResult<Addr> {
    OWNER.get(deps).transpose().unwrap()
}

/// query config
pub fn query_config(deps: Deps, _env: Env) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

/// query config
pub fn query_reward_config(deps: Deps, _env: Env) -> StdResult<RewardDistributionConfig> {
    REWARD_DISTRIBUTION_CONFIG.load(deps.storage)
}

/// query eclipASTRO Lockdrop info
pub fn query_single_lockup_info(deps: Deps, env: Env) -> StdResult<SingleLockupInfoResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut single_staking_rewards: Vec<SingleStakingRewardsByDuration> = vec![];
    let single_lockups = SINGLE_LOCKUP_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| {
            let (duration, lockup_info) = r.unwrap();
            let locked_at = if duration == 0u64 {
                0u64
            } else {
                cfg.countdown_start_at
            };
            let single_staking_reward: UserReward = if cfg.single_sided_staking.is_some() {
                deps.querier
                    .query_wasm_smart(
                        cfg.single_sided_staking.clone().unwrap(),
                        &SingleSidedQueryMsg::Reward {
                            user: env.contract.address.to_string(),
                            duration,
                            locked_at,
                        },
                    )
                    .unwrap()
            } else {
                UserReward::default()
            };
            single_staking_rewards.push(SingleStakingRewardsByDuration {
                duration,
                rewards: single_staking_reward,
            });
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
    _env: Env,
    user_address: String,
) -> StdResult<Vec<UserSingleLockupInfoResponse>> {
    let cfg = CONFIG.load(deps.storage)?;
    let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let blacklist = BLACK_LIST.load(deps.storage).unwrap_or_default();

    if cfg.claims_allowed {
        Ok(SINGLE_USER_LOCKUP_INFO
            .prefix(&user_address)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|r| {
                let (duration, mut user_lockup_info) = r.unwrap();
                let locked_at = if duration == 0u64 {
                    0u64
                } else {
                    cfg.countdown_start_at
                };
                let adjust_reward = ADJUST_REWARDS
                    .load(deps.storage, &(user_address.clone(), duration))
                    .unwrap_or_default();
                if user_lockup_info.total_eclipastro_staked.is_zero() {
                    user_lockup_info.total_eclipastro_staked = user_lockup_info
                        .xastro_amount_in_lockups
                        .multiply_ratio(state.total_eclipastro_lockup, state.total_xastro);
                }
                let mut single_staking_reward: UserReward = if cfg.single_sided_staking.is_some() {
                    deps.querier
                        .query_wasm_smart(
                            cfg.single_sided_staking.clone().unwrap(),
                            &SingleSidedQueryMsg::CalculateReward {
                                amount: user_lockup_info.total_eclipastro_staked
                                    - user_lockup_info.total_eclipastro_withdrawed,
                                duration,
                                locked_at: Some(locked_at),
                                from: user_lockup_info
                                    .last_claimed
                                    .unwrap_or(cfg.countdown_start_at),
                                to: None,
                            },
                        )
                        .unwrap()
                } else {
                    UserReward::default()
                };
                if !adjust_reward.is_zero() {
                    if adjust_reward.gt(&single_staking_reward.eclipastro) {
                        single_staking_reward.eclipastro = Uint128::zero();
                    } else {
                        single_staking_reward.eclipastro -= adjust_reward;
                    }
                }
                let lockdrop_incentives = if blacklist.contains(&user_address) {
                    LockdropIncentives::default()
                } else {
                    get_user_single_lockdrop_incentives(
                        deps,
                        user_lockup_info.lockdrop_incentives,
                        user_lockup_info.xastro_amount_in_lockups,
                        duration,
                    )
                    .unwrap()
                };
                let staking_rewards = if blacklist.contains(&user_address) {
                    vec![]
                } else {
                    vec![
                        Asset {
                            info: cfg.eclipastro_token.clone().unwrap(),
                            amount: single_staking_reward.eclipastro
                                + user_lockup_info.unclaimed_rewards.eclipastro,
                        },
                        Asset {
                            info: cfg.beclip.clone(),
                            amount: single_staking_reward.beclip
                                + user_lockup_info.unclaimed_rewards.beclip,
                        },
                        Asset {
                            info: cfg.eclip.clone(),
                            amount: single_staking_reward.eclip
                                + user_lockup_info.unclaimed_rewards.eclip,
                        },
                    ]
                };
                UserSingleLockupInfoResponse {
                    duration,
                    xastro_amount_in_lockups: user_lockup_info.xastro_amount_in_lockups,
                    eclipastro_staked: user_lockup_info.total_eclipastro_staked,
                    eclipastro_withdrawed: user_lockup_info.total_eclipastro_withdrawed,
                    lockdrop_incentives,
                    withdrawal_flag: user_lockup_info.withdrawal_flag,
                    staking_rewards,
                    countdown_start_at: cfg.countdown_start_at,
                }
            })
            .collect::<Vec<UserSingleLockupInfoResponse>>())
    } else {
        Ok(SINGLE_USER_LOCKUP_INFO
            .prefix(&user_address)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|r| {
                let (duration, user_lockup_info) = r.unwrap();
                let lockdrop_incentives = if blacklist.contains(&user_address) {
                    LockdropIncentives::default()
                } else {
                    get_user_single_lockdrop_incentives(
                        deps,
                        user_lockup_info.lockdrop_incentives,
                        user_lockup_info.xastro_amount_in_lockups,
                        duration,
                    )
                    .unwrap()
                };
                UserSingleLockupInfoResponse {
                    duration,
                    xastro_amount_in_lockups: user_lockup_info.xastro_amount_in_lockups,
                    eclipastro_staked: Uint128::zero(),
                    eclipastro_withdrawed: Uint128::zero(),
                    withdrawal_flag: user_lockup_info.withdrawal_flag,
                    lockdrop_incentives,
                    staking_rewards: vec![],
                    countdown_start_at: cfg.countdown_start_at,
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
    let blacklist = BLACK_LIST.load(deps.storage).unwrap_or_default();

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
                    pending_lockdrop_incentives.clone(),
                    user_lockup_info.clone(),
                )
                .unwrap();
                let lockdrop_incentives = if blacklist.contains(&user_address) {
                    LockdropIncentives::default()
                } else {
                    user_lockup_info.lockdrop_incentives.clone()
                };
                let staking_rewards = if blacklist.contains(&user_address) {
                    vec![]
                } else {
                    vec![
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: cfg.astro_token.clone(),
                            },
                            amount: user_rewards.astro,
                        },
                        Asset {
                            info: cfg.beclip.clone(),
                            amount: user_rewards.beclip - pending_lockdrop_incentives.beclip,
                        },
                        Asset {
                            info: cfg.eclip.clone(),
                            amount: user_rewards.eclip - pending_lockdrop_incentives.eclip,
                        },
                    ]
                };
                UserLpLockupInfoResponse {
                    duration,
                    xastro_amount_in_lockups: user_lockup_info.xastro_amount_in_lockups,
                    lp_token_staked: user_lockup_info.total_lp_staked,
                    lp_token_withdrawed: user_lockup_info.total_lp_withdrawed,
                    lockdrop_incentives,
                    withdrawal_flag: user_lockup_info.withdrawal_flag,
                    staking_rewards,
                    countdown_start_at: cfg.countdown_start_at,
                    reward_weights: user_lockup_info.reward_weights,
                }
            })
            .collect::<Vec<UserLpLockupInfoResponse>>())
    } else {
        Ok(LP_USER_LOCKUP_INFO
            .prefix(&user_address)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|r| {
                let (duration, user_lockup_info) = r.unwrap();
                let lockdrop_incentives = if blacklist.contains(&user_address) {
                    LockdropIncentives::default()
                } else {
                    get_user_lp_lockdrop_incentives(
                        deps,
                        user_lockup_info.lockdrop_incentives,
                        user_lockup_info.xastro_amount_in_lockups,
                        duration,
                    )
                    .unwrap()
                };
                UserLpLockupInfoResponse {
                    duration,
                    xastro_amount_in_lockups: user_lockup_info.xastro_amount_in_lockups,
                    lp_token_staked: Uint128::zero(),
                    lp_token_withdrawed: Uint128::zero(),
                    withdrawal_flag: user_lockup_info.withdrawal_flag,
                    lockdrop_incentives,
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

pub fn query_blacklist(deps: Deps) -> StdResult<Vec<String>> {
    Ok(BLACK_LIST.load(deps.storage).unwrap_or_default())
}

pub fn query_blacklist_rewards(deps: Deps, env: Env) -> StdResult<BlacklistRewards> {
    let cfg = CONFIG.load(deps.storage)?;
    let blacklist = BLACK_LIST.load(deps.storage).unwrap_or_default();
    let mut blacklist_rewards = BLACK_LIST_REWARDS.load(deps.storage).unwrap_or_default();
    let single_state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let lp_state = LP_LOCKUP_STATE.load(deps.storage)?;
    let block_time = env.block.time.seconds();
    for user in blacklist.iter() {
        for r in
            SINGLE_USER_LOCKUP_INFO
                .prefix(user)
                .range(deps.storage, None, None, Order::Ascending)
        {
            let (duration, mut user_lockup_info) = r.unwrap();
            let locked_at = if duration == 0u64 {
                0u64
            } else {
                cfg.countdown_start_at
            };
            if user_lockup_info.total_eclipastro_staked.is_zero() {
                user_lockup_info.total_eclipastro_staked =
                    user_lockup_info.xastro_amount_in_lockups.multiply_ratio(
                        single_state.total_eclipastro_lockup,
                        single_state.total_xastro,
                    );
            }
            let single_staking_reward: UserReward = if cfg.single_sided_staking.is_some() {
                deps.querier
                    .query_wasm_smart(
                        cfg.single_sided_staking.clone().unwrap(),
                        &SingleSidedQueryMsg::CalculateReward {
                            amount: user_lockup_info.total_eclipastro_staked
                                - user_lockup_info.total_eclipastro_withdrawed,
                            duration,
                            locked_at: Some(locked_at),
                            from: user_lockup_info
                                .last_claimed
                                .unwrap_or(cfg.countdown_start_at),
                            to: None,
                        },
                    )
                    .unwrap()
            } else {
                UserReward::default()
            };
            let lockdrop_incentives = get_user_single_lockdrop_incentives(
                deps,
                user_lockup_info.lockdrop_incentives,
                user_lockup_info.xastro_amount_in_lockups,
                duration,
            )
            .unwrap();
            let pending_lockdrop_incentives = calculate_pending_lockdrop_incentives(
                deps,
                block_time,
                lockdrop_incentives.clone(),
            )?;
            blacklist_rewards.eclip += pending_lockdrop_incentives.eclip
                + single_staking_reward.eclip
                + user_lockup_info.unclaimed_rewards.eclip;
            blacklist_rewards.beclip += pending_lockdrop_incentives.beclip
                + single_staking_reward.beclip
                + user_lockup_info.unclaimed_rewards.beclip;
            blacklist_rewards.eclipastro +=
                single_staking_reward.eclipastro + user_lockup_info.unclaimed_rewards.eclipastro;
        }
        let lp_total_rewards = calculate_lp_total_rewards(deps, env.contract.address.to_string())?;
        let updated_reward_weights =
            calculate_updated_lp_reward_weights(deps, &lp_total_rewards).unwrap();
        for r in LP_USER_LOCKUP_INFO
            .prefix(user)
            .range(deps.storage, None, None, Order::Ascending)
        {
            let (duration, mut user_lockup_info) = r.unwrap();
            if user_lockup_info.total_lp_staked.is_zero() {
                user_lockup_info.total_lp_staked = user_lockup_info
                    .xastro_amount_in_lockups
                    .multiply_ratio(lp_state.total_lp_lockdrop, lp_state.total_xastro);
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
            let user_rewards = calculate_lp_staking_user_rewards(
                deps,
                updated_reward_weights.clone(),
                pending_lockdrop_incentives.clone(),
                user_lockup_info.clone(),
            )
            .unwrap();
            blacklist_rewards.astro += user_rewards.astro;
            blacklist_rewards.beclip += user_rewards.beclip;
            blacklist_rewards.eclip += user_rewards.eclip;
        }
    }
    Ok(blacklist_rewards)
}

pub fn query_calculate_penalty_amount(
    deps: Deps,
    env: Env,
    amount: Uint128,
    duration: u64,
) -> StdResult<Uint128> {
    let block_time = env.block.time.seconds();
    calculate_penalty_amount(deps, amount, duration, block_time)
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
        .multiply_ratio(reward_cfg.instant, BPS_DENOMINATOR);
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
                    .multiply_ratio(duration_multiplier, BPS_DENOMINATOR)
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
                    .multiply_ratio(duration_multiplier, BPS_DENOMINATOR)
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
                        .multiply_ratio(duration_multiplier, BPS_DENOMINATOR),
                )
                .unwrap()
            });
        LockdropIncentives {
            eclip: LockdropIncentive {
                allocated: Uint256::from(xastro_amount_in_lockups)
                    .multiply_ratio(duration_multiplier, BPS_DENOMINATOR)
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
                    .multiply_ratio(duration_multiplier, BPS_DENOMINATOR)
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
                    .multiply_ratio(duration_multiplier, BPS_DENOMINATOR)
                    .multiply_ratio(lp_lockdrop_incentives.eclip, lp_state.weighted_total_xastro)
                    .try_into()
                    .unwrap_or_default(),
                claimed: Uint128::zero(),
            },
            beclip: LockdropIncentive {
                allocated: Uint256::from(xastro_amount_in_lockups)
                    .multiply_ratio(duration_multiplier, BPS_DENOMINATOR)
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
                        .multiply_ratio(duration_multiplier, BPS_DENOMINATOR),
                )
                .unwrap()
            });
        LockdropIncentives {
            eclip: LockdropIncentive {
                allocated: Uint256::from(xastro_amount_in_lockups)
                    .multiply_ratio(duration_multiplier, BPS_DENOMINATOR)
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
                    .multiply_ratio(duration_multiplier, BPS_DENOMINATOR)
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

pub fn calculate_lp_total_rewards(deps: Deps, user: String) -> StdResult<LpStakingRewards> {
    let cfg = CONFIG.load(deps.storage)?;
    if cfg.lp_staking.is_none() {
        return Ok(LpStakingRewards::default());
    }
    let rewards: Vec<RewardAmount> = deps.querier.query_wasm_smart(
        cfg.lp_staking.unwrap().to_string(),
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

pub fn query_native_token_supply(querier: &QuerierWrapper, denom: String) -> StdResult<Coin> {
    let supply: SupplyResponse = querier.query(&QueryRequest::Bank(BankQuery::Supply { denom }))?;
    Ok(supply.amount)
}

pub fn check_lockdrop_ended(deps: Deps, current_time: u64) -> StdResult<bool> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(current_time >= (cfg.init_timestamp + cfg.deposit_window + cfg.withdrawal_window))
}

pub fn check_deposit_window(deps: Deps, current_time: u64) -> StdResult<bool> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(
        current_time >= cfg.init_timestamp
            && current_time < cfg.init_timestamp + cfg.deposit_window,
    )
}

pub fn check_withdrawal_window(deps: Deps, current_time: u64) -> StdResult<bool> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(current_time >= cfg.init_timestamp + cfg.deposit_window
        && current_time < cfg.init_timestamp + cfg.deposit_window + cfg.withdrawal_window)
}

pub fn query_astro_staking_total_deposit(deps: Deps) -> StdResult<Uint128> {
    let cfg = CONFIG.load(deps.storage)?;
    deps.querier.query_wasm_smart(
        cfg.astro_staking,
        &AstroportStakingQueryMsg::TotalDeposit {},
    )
}

pub fn query_astro_staking_total_shares(deps: Deps) -> StdResult<Uint128> {
    let cfg = CONFIG.load(deps.storage)?;
    deps.querier
        .query_wasm_smart(cfg.astro_staking, &AstroportStakingQueryMsg::TotalShares {})
}

pub fn query_lp_pool_assets(deps: Deps) -> StdResult<Vec<Asset>> {
    let cfg = CONFIG.load(deps.storage)?;
    let response: PoolResponse = deps
        .querier
        .query_wasm_smart(cfg.liquidity_pool.unwrap(), &PoolQueryMsg::Pool {})?;
    Ok(response.assets)
}

pub fn query_user_single_rewards(
    deps: Deps,
    amount: Uint128,
    duration: u64,
    locked_at: Option<u64>,
    from: u64,
) -> StdResult<UserReward> {
    let cfg = CONFIG.load(deps.storage)?;
    deps.querier.query_wasm_smart(
        cfg.single_sided_staking.unwrap(),
        &SingleSidedQueryMsg::CalculateReward {
            amount,
            duration,
            locked_at,
            from,
            to: None,
        },
    )
}

pub fn check_lock_ended(deps: Deps, duration: u64, block_time: u64) -> StdResult<bool> {
    let cfg = CONFIG.load(deps.storage)?;
    let locked_at = if cfg.claims_allowed {
        cfg.countdown_start_at
    } else {
        block_time
    };
    let one_day = 86400u64;
    let lock_end_time = (duration + locked_at) / one_day * one_day + one_day;
    if lock_end_time < block_time {
        return Ok(true);
    }
    Ok(false)
}

pub fn calculate_penalty_amount(
    deps: Deps,
    amount: Uint128,
    duration: u64,
    block_time: u64,
) -> StdResult<Uint128> {
    let cfg = CONFIG.load(deps.storage)?;
    let locked_at = if cfg.claims_allowed {
        cfg.countdown_start_at
    } else {
        block_time
    };
    let one_day = 86400u64;
    let lock_end_time = (duration + locked_at) / one_day * one_day + one_day;
    if lock_end_time < block_time {
        return Ok(Uint128::zero());
    }
    let remain_time = min(lock_end_time - block_time, duration);
    let last_early_unlock_penalty = Decimal::from_str(DEFAULT_LAST_EARLY_UNLOCK_PENALTY).unwrap();
    let penalty_amount = Decimal::from_ratio(amount, 1u128)
        * ((cfg.init_early_unlock_penalty - last_early_unlock_penalty)
            * Decimal::from_ratio(remain_time, duration)
            + last_early_unlock_penalty);
    Ok(penalty_amount.to_uint_floor())
}
