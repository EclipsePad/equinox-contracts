use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::{Addr, Decimal, Deps, Env, Order, StdResult, Uint128, Uint256};
use cw20::BalanceResponse;
use equinox_msg::{
    flexible_staking::QueryMsg as FlexibleStakingQueryMsg,
    lockdrop::{
        AssetRewardWeight, Config, LockupInfoResponse, LpLockupStateResponse,
        SingleLockupStateResponse, UserLpLockupInfoResponse, UserSingleLockupInfoResponse,
    },
    lp_staking::{QueryMsg as LpStakingQueryMsg, UserRewardResponse},
    reward_distributor::{
        Config as RewardDistributorConfig, FlexibleReward, QueryMsg as RewardDistributorQueryMsg,
        TimelockReward,
    },
    timelock_staking::QueryMsg as TimelockStakingQueryMsg,
};

use crate::{
    error::ContractError,
    state::{
        CONFIG, LP_LOCKUP_INFO, LP_LOCKUP_STATE, LP_USER_LOCKUP_INFO, OWNER,
        REWARD_DISTRIBUTION_CONFIG, SINGLE_LOCKUP_INFO, SINGLE_LOCKUP_STATE,
        SINGLE_USER_LOCKUP_INFO, TOTAL_ECLIP_INCENTIVES,
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

/// query eclipASTRO Lockdrop info
pub fn query_single_lockup_info(deps: Deps, _env: Env) -> StdResult<Vec<LockupInfoResponse>> {
    let single_lockup = SINGLE_LOCKUP_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| {
            let (duration, lockup_info) = r.unwrap();
            LockupInfoResponse {
                duration,
                xastro_amount_in_lockups: lockup_info.xastro_amount_in_lockups,
                total_staked: lockup_info.total_staked,
                total_withdrawed: lockup_info.total_withdrawed,
            }
        })
        .collect::<Vec<LockupInfoResponse>>();
    Ok(single_lockup)
}

/// query eclipASTRO/xASTRO Lp token Lockdrop info
pub fn query_lp_lockup_info(deps: Deps, _env: Env) -> StdResult<Vec<LockupInfoResponse>> {
    let lp_lockup = LP_LOCKUP_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| {
            let (duration, lockup_info) = r.unwrap();
            LockupInfoResponse {
                duration,
                xastro_amount_in_lockups: lockup_info.xastro_amount_in_lockups,
                total_staked: lockup_info.total_staked,
                total_withdrawed: lockup_info.total_withdrawed,
            }
        })
        .collect::<Vec<LockupInfoResponse>>();
    Ok(lp_lockup)
}

/// query eclipASTRO lockup state
pub fn query_single_lockup_state(deps: Deps, _env: Env) -> StdResult<SingleLockupStateResponse> {
    let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    Ok(SingleLockupStateResponse {
        are_claims_allowed: state.are_claims_allowed,
        countdown_start_at: state.countdown_start_at,
        is_staked: state.is_staked,
        total_eclipastro_lockup: state.total_eclipastro_lockup,
    })
}

/// query eclipASTRO/xASTRO lp token lockup state
pub fn query_lp_lockup_state(deps: Deps, _env: Env) -> StdResult<LpLockupStateResponse> {
    let state = LP_LOCKUP_STATE.load(deps.storage)?;
    Ok(LpLockupStateResponse {
        are_claims_allowed: state.are_claims_allowed,
        countdown_start_at: state.countdown_start_at,
        is_staked: state.is_staked,
        total_lp_lockdrop: state.total_lp_lockdrop,
    })
}

/// query eclipASTRO user lockup info
pub fn query_user_single_lockup_info(
    deps: Deps,
    env: Env,
    user_address: Addr,
) -> StdResult<Vec<UserSingleLockupInfoResponse>> {
    let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let cfg = CONFIG.load(deps.storage)?;
    let reward_cfg = REWARD_DISTRIBUTION_CONFIG.load(deps.storage)?;

    if state.is_staked {
        let flexible_staking_reward_response: FlexibleReward = deps.querier.query_wasm_smart(
            cfg.flexible_staking.clone().unwrap(),
            &FlexibleStakingQueryMsg::Reward {
                user: env.contract.address.to_string(),
            },
        )?;

        let lock_staking_reward_response: Vec<TimelockReward> = deps.querier.query_wasm_smart(
            cfg.timelock_staking.clone().unwrap(),
            &TimelockStakingQueryMsg::Reward {
                user: env.contract.address.to_string(),
            },
        )?;

        let mut pending_eclip_rewards = flexible_staking_reward_response.eclip;
        let mut pending_eclipastro_rewards = flexible_staking_reward_response.eclipastro;
        for r in lock_staking_reward_response {
            pending_eclip_rewards += r.eclip;
            pending_eclipastro_rewards += r.eclipastro;
        }

        let toal_eclipastro_staked = SINGLE_LOCKUP_INFO
            .range(deps.storage, None, None, Order::Ascending)
            .fold(Uint128::zero(), |acc, r| {
                let (_, lockup_info) = r.unwrap();
                acc + lockup_info.total_staked - lockup_info.total_withdrawed
            });

        let reward_distributor_config: RewardDistributorConfig = deps.querier.query_wasm_smart(
            cfg.reward_distributor.unwrap().to_string(),
            &RewardDistributorQueryMsg::Config {},
        )?;
        let locking_reward_config = reward_distributor_config.locking_reward_config;

        let total_eclipastro_staked_with_reward_multiplier = SINGLE_LOCKUP_INFO
            .range(deps.storage, None, None, Order::Ascending)
            .fold(Uint128::zero(), |acc, cur| {
                let (d, lockup_info) = cur.unwrap();
                let reward_config = locking_reward_config.iter().find(|c| c.duration == d);
                let reward_multiplier = match reward_config {
                    Some(c) => c.multiplier,
                    None => 0u64,
                };
                acc + (lockup_info.total_staked - lockup_info.total_withdrawed)
                    .checked_mul(Uint128::from(reward_multiplier))
                    .unwrap()
            });

        let reward_weights = if !state.reward_weights.is_empty() {
            state.reward_weights
        } else {
            vec![
                AssetRewardWeight {
                    asset: AssetInfo::Token {
                        contract_addr: cfg.eclipastro_token.clone(),
                    },
                    weight: Decimal::zero(),
                },
                AssetRewardWeight {
                    asset: AssetInfo::NativeToken {
                        denom: cfg.eclip.clone(),
                    },
                    weight: Decimal::zero(),
                },
            ]
        };

        let eclipastro_reward_weight = reward_weights
            .iter()
            .find(|w| {
                w.asset.equal(&AssetInfo::Token {
                    contract_addr: cfg.eclipastro_token.clone(),
                })
            })
            .unwrap()
            .weight
            + Decimal::from_ratio(pending_eclipastro_rewards, toal_eclipastro_staked);
        let eclip_reward_weight = reward_weights
            .iter()
            .find(|w| {
                w.asset.equal(&AssetInfo::NativeToken {
                    denom: cfg.eclip.clone(),
                })
            })
            .unwrap()
            .weight
            + Decimal::from_ratio(
                pending_eclip_rewards,
                total_eclipastro_staked_with_reward_multiplier,
            );

        Ok(SINGLE_USER_LOCKUP_INFO
            .prefix(&user_address)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|r| {
                let (d, user_lockup_info) = r.unwrap();
                let reward_config = locking_reward_config.iter().find(|c| c.duration == d);
                let reward_multiplier = match reward_config {
                    Some(c) => c.multiplier,
                    None => 0u64,
                };

                let eclipastro_staked = if user_lockup_info.xastro_amount_in_lockups.is_zero() {
                    Uint128::zero()
                } else if user_lockup_info.total_eclipastro_staked.is_zero() {
                    user_lockup_info
                        .xastro_amount_in_lockups
                        .multiply_ratio(state.total_eclipastro_lockup, state.total_xastro)
                } else {
                    user_lockup_info.total_eclipastro_staked
                        - user_lockup_info.total_eclipastro_withdrawed
                };
                let user_eclipastro_reward_data =
                    user_lockup_info.reward_weights.iter().find(|w| {
                        w.asset.equal(&AssetInfo::Token {
                            contract_addr: cfg.eclipastro_token.clone(),
                        })
                    });
                let user_eclipastro_reward_weight = match user_eclipastro_reward_data {
                    Some(r) => r.weight,
                    None => Decimal::zero(),
                };
                let pending_eclipastro_reward =
                    eclipastro_staked * (eclipastro_reward_weight - user_eclipastro_reward_weight);
                let user_eclip_reward_data = user_lockup_info.reward_weights.iter().find(|w| {
                    w.asset.equal(&AssetInfo::NativeToken {
                        denom: cfg.eclip.clone(),
                    })
                });
                let user_eclip_reward_weight = match user_eclip_reward_data {
                    Some(r) => r.weight,
                    None => Decimal::zero(),
                };
                let pending_eclip_reward = eclipastro_staked
                    * Uint128::from(reward_multiplier)
                    * (eclip_reward_weight - user_eclip_reward_weight);
                let user_total_eclip_incentives = if user_lockup_info
                    .total_eclip_incentives
                    .is_zero()
                {
                    calculate_user_eclip_incentives_for_single_lockup(deps, user_address.clone(), d)
                        .unwrap()
                } else {
                    user_lockup_info.total_eclip_incentives
                };
                let instant_amount =
                    user_total_eclip_incentives.multiply_ratio(reward_cfg.instant, 10000u64);
                let claimable_eclip_incentives = if env.block.time.seconds()
                    > state.countdown_start_at + reward_cfg.vesting_period
                {
                    user_total_eclip_incentives
                } else {
                    instant_amount
                        + (user_total_eclip_incentives - instant_amount).multiply_ratio(
                            env.block.time.seconds() - state.countdown_start_at,
                            reward_cfg.vesting_period,
                        )
                };
                let user_pending_eclip_incentives =
                    claimable_eclip_incentives - user_lockup_info.claimed_eclip_incentives;
                UserSingleLockupInfoResponse {
                    duration: d,
                    xastro_amount_in_lockups: user_lockup_info.xastro_amount_in_lockups,
                    eclipastro_staked,
                    eclipastro_withdrawed: user_lockup_info.total_eclipastro_withdrawed,
                    withdrawal_flag: user_lockup_info.withdrawal_flag,
                    total_eclip_incentives: user_total_eclip_incentives,
                    claimed_eclip_incentives: user_lockup_info.claimed_eclip_incentives,
                    pending_eclip_incentives: user_pending_eclip_incentives,
                    staking_rewards: vec![
                        Asset {
                            info: AssetInfo::Token {
                                contract_addr: cfg.eclipastro_token.clone(),
                            },
                            amount: pending_eclipastro_reward,
                        },
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: cfg.eclip.clone(),
                            },
                            amount: pending_eclip_reward,
                        },
                    ],
                    countdown_start_at: state.countdown_start_at,
                }
            })
            .collect::<Vec<UserSingleLockupInfoResponse>>())
    } else {
        Ok(SINGLE_USER_LOCKUP_INFO
            .prefix(&user_address)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|r| {
                let (duration, user_lockup_info) = r.unwrap();
                UserSingleLockupInfoResponse {
                    duration,
                    xastro_amount_in_lockups: user_lockup_info.xastro_amount_in_lockups,
                    eclipastro_staked: Uint128::zero(),
                    eclipastro_withdrawed: Uint128::zero(),
                    pending_eclip_incentives: Uint128::zero(),
                    withdrawal_flag: user_lockup_info.withdrawal_flag,
                    total_eclip_incentives: user_lockup_info.total_eclip_incentives,
                    claimed_eclip_incentives: user_lockup_info.claimed_eclip_incentives,
                    staking_rewards: vec![],
                    countdown_start_at: state.countdown_start_at,
                }
            })
            .collect::<Vec<UserSingleLockupInfoResponse>>())
    }
}

/// query lp token user lockup info
pub fn query_user_lp_lockup_info(
    deps: Deps,
    env: Env,
    user_address: Addr,
) -> StdResult<Vec<UserLpLockupInfoResponse>> {
    let state = LP_LOCKUP_STATE.load(deps.storage)?;
    let cfg = CONFIG.load(deps.storage)?;
    let reward_cfg = REWARD_DISTRIBUTION_CONFIG.load(deps.storage)?;

    if state.is_staked {
        let lp_staking_reward_response: Vec<UserRewardResponse> = deps.querier.query_wasm_smart(
            cfg.lp_staking.clone().unwrap(),
            &LpStakingQueryMsg::Reward {
                user: env.contract.address.to_string(),
            },
        )?;

        let pending_eclip_reward_data = lp_staking_reward_response.clone().into_iter().find(|r| {
            r.asset.equal(&AssetInfo::NativeToken {
                denom: cfg.eclip.clone(),
            })
        });
        let pending_eclip_rewards = match pending_eclip_reward_data {
            Some(d) => d.amount,
            None => Uint128::zero(),
        };
        let pending_astro_reward_data = lp_staking_reward_response.into_iter().find(|r| {
            r.asset.equal(&AssetInfo::Token {
                contract_addr: cfg.astro_token.clone(),
            })
        });
        let pending_astro_rewards = match pending_astro_reward_data {
            Some(d) => d.amount,
            None => Uint128::zero(),
        };

        let total_lp_lockup = LP_LOCKUP_INFO
            .range(deps.storage, None, None, Order::Ascending)
            .fold(Uint128::zero(), |acc, i| {
                let (_, lockup_info) = i.unwrap();
                acc + lockup_info.total_staked - lockup_info.total_withdrawed
            });

        let reward_weights = if !state.reward_weights.is_empty() {
            state.reward_weights
        } else {
            vec![
                AssetRewardWeight {
                    asset: AssetInfo::Token {
                        contract_addr: cfg.astro_token.clone(),
                    },
                    weight: Decimal::zero(),
                },
                AssetRewardWeight {
                    asset: AssetInfo::NativeToken {
                        denom: cfg.eclip.clone(),
                    },
                    weight: Decimal::zero(),
                },
            ]
        };

        let astro_reward_weight = reward_weights
            .iter()
            .find(|w| {
                w.asset.equal(&AssetInfo::Token {
                    contract_addr: cfg.astro_token.clone(),
                })
            })
            .unwrap()
            .weight
            + Decimal::from_ratio(pending_astro_rewards, total_lp_lockup);
        let eclip_reward_weight = reward_weights
            .iter()
            .find(|w| {
                w.asset.equal(&AssetInfo::NativeToken {
                    denom: cfg.eclip.clone(),
                })
            })
            .unwrap()
            .weight
            + Decimal::from_ratio(pending_eclip_rewards, total_lp_lockup);

        Ok(LP_USER_LOCKUP_INFO
            .prefix(&user_address)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|r| {
                let (d, user_lockup_info) = r.unwrap();

                let user_lp_staked = if user_lockup_info.xastro_amount_in_lockups.is_zero() {
                    Uint128::zero()
                } else if user_lockup_info.total_lp_staked.is_zero() {
                    user_lockup_info
                        .xastro_amount_in_lockups
                        .multiply_ratio(state.total_lp_lockdrop, state.total_xastro)
                } else {
                    user_lockup_info.total_lp_staked - user_lockup_info.total_lp_withdrawed
                };
                let user_astro_reward_data = user_lockup_info.reward_weights.iter().find(|w| {
                    w.asset.equal(&AssetInfo::Token {
                        contract_addr: cfg.astro_token.clone(),
                    })
                });
                let user_astro_reward_weight = match user_astro_reward_data {
                    Some(r) => r.weight,
                    None => Decimal::zero(),
                };
                let pending_astro_reward =
                    user_lp_staked * (astro_reward_weight - user_astro_reward_weight);

                let user_eclip_reward_data = user_lockup_info.reward_weights.iter().find(|w| {
                    w.asset.equal(&AssetInfo::NativeToken {
                        denom: cfg.eclip.clone(),
                    })
                });
                let user_eclip_reward_weight = match user_eclip_reward_data {
                    Some(r) => r.weight,
                    None => Decimal::zero(),
                };
                let pending_eclip_reward =
                    user_lp_staked * (eclip_reward_weight - user_eclip_reward_weight);
                let user_total_eclip_incentives =
                    if user_lockup_info.total_eclip_incentives.is_zero() {
                        calculate_user_eclip_incentives_for_lp_lockup(deps, user_address.clone(), d)
                            .unwrap()
                    } else {
                        user_lockup_info.total_eclip_incentives
                    };

                let instant_amount =
                    user_total_eclip_incentives.multiply_ratio(reward_cfg.instant, 10000u64);
                let claimable_eclip_incentives = if env.block.time.seconds()
                    > state.countdown_start_at + reward_cfg.vesting_period
                {
                    user_total_eclip_incentives
                } else {
                    instant_amount
                        + (user_total_eclip_incentives - instant_amount).multiply_ratio(
                            env.block.time.seconds() - state.countdown_start_at,
                            reward_cfg.vesting_period,
                        )
                };
                let user_pending_eclip_incentives =
                    claimable_eclip_incentives - user_lockup_info.claimed_eclip_incentives;
                UserLpLockupInfoResponse {
                    duration: d,
                    xastro_amount_in_lockups: user_lockup_info.xastro_amount_in_lockups,
                    lp_token_staked: user_lp_staked,
                    lp_token_withdrawed: user_lockup_info.total_lp_withdrawed,
                    withdrawal_flag: user_lockup_info.withdrawal_flag,
                    total_eclip_incentives: user_total_eclip_incentives,
                    claimed_eclip_incentives: user_lockup_info.claimed_eclip_incentives,
                    pending_eclip_incentives: user_pending_eclip_incentives,
                    staking_rewards: vec![
                        Asset {
                            info: AssetInfo::Token {
                                contract_addr: cfg.astro_token.clone(),
                            },
                            amount: pending_astro_reward,
                        },
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: cfg.eclip.clone(),
                            },
                            amount: pending_eclip_reward,
                        },
                    ],
                    countdown_start_at: state.countdown_start_at,
                }
            })
            .collect::<Vec<UserLpLockupInfoResponse>>())
    } else {
        Ok(LP_USER_LOCKUP_INFO
            .prefix(&user_address)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|r| {
                let (duration, user_lockup_info) = r.unwrap();
                UserLpLockupInfoResponse {
                    duration,
                    xastro_amount_in_lockups: user_lockup_info.xastro_amount_in_lockups,
                    lp_token_staked: Uint128::zero(),
                    lp_token_withdrawed: Uint128::zero(),
                    withdrawal_flag: user_lockup_info.withdrawal_flag,
                    total_eclip_incentives: user_lockup_info.total_eclip_incentives,
                    claimed_eclip_incentives: user_lockup_info.claimed_eclip_incentives,
                    pending_eclip_incentives: Uint128::zero(),
                    staking_rewards: vec![],
                    countdown_start_at: state.countdown_start_at,
                }
            })
            .collect::<Vec<UserLpLockupInfoResponse>>())
    }
}

pub fn query_total_eclip_incentives(deps: Deps) -> StdResult<BalanceResponse> {
    Ok(BalanceResponse {
        balance: TOTAL_ECLIP_INCENTIVES
            .load(deps.storage)
            .unwrap_or_default(),
    })
}

pub fn calculate_user_eclip_incentives_for_single_lockup(
    deps: Deps,
    user_address: Addr,
    duration: u64,
) -> Result<Uint128, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let total_eclip_incentives = TOTAL_ECLIP_INCENTIVES
        .load(deps.storage)
        .unwrap_or_default();
    let single_sided_state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let lp_state = LP_LOCKUP_STATE.load(deps.storage)?;
    let user_lockup_info = SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&user_address, duration))?;
    let duration_multiplier = cfg
        .lock_configs
        .into_iter()
        .find(|c| c.duration == duration)
        .unwrap_or_default()
        .multiplier;
    // convert user xastro amount to eclipastro amount, multiply duration multiplier, calculate eclip incentives
    let amount = Uint256::from(user_lockup_info.xastro_amount_in_lockups)
        .checked_mul(Uint256::from(duration_multiplier))
        .unwrap()
        .multiply_ratio(
            total_eclip_incentives,
            single_sided_state.weighted_total_xastro + lp_state.weighted_total_xastro,
        )
        .try_into()
        .unwrap();
    Ok(amount)
}

pub fn calculate_user_eclip_incentives_for_lp_lockup(
    deps: Deps,
    user_address: Addr,
    duration: u64,
) -> Result<Uint128, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let total_eclip_incentives = TOTAL_ECLIP_INCENTIVES
        .load(deps.storage)
        .unwrap_or_default();
    let single_sided_state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let lp_state = LP_LOCKUP_STATE.load(deps.storage)?;
    let user_lockup_info = LP_USER_LOCKUP_INFO.load(deps.storage, (&user_address, duration))?;
    let duration_multiplier = cfg
        .lock_configs
        .into_iter()
        .find(|c| c.duration == duration)
        .unwrap_or_default()
        .multiplier;
    let amount = Uint256::from(user_lockup_info.xastro_amount_in_lockups)
        .checked_mul(Uint256::from(duration_multiplier))
        .unwrap()
        .multiply_ratio(
            total_eclip_incentives,
            single_sided_state.weighted_total_xastro + lp_state.weighted_total_xastro,
        )
        .try_into()
        .unwrap();
    Ok(amount)
}
