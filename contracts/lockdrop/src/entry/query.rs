use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::{Addr, Decimal, Deps, Env, Order, StdResult, Uint128};
use equinox_msg::{
    flexible_staking::QueryMsg as FlexibleStakingQueryMsg,
    lockdrop::{
        AssetRewardWeight, Config, LockupInfoResponse, LpLockupStateResponse,
        SingleLockupStateResponse, UserLpLockupInfoResponse, UserSingleLockupInfoResponse,
    },
    lp_staking::{LpRewards, QueryMsg as LpStakingQueryMsg},
    reward_distributor::{
        Config as RewardDistributorConfig, FlexibleReward, QueryMsg as RewardDistributorQueryMsg,
        TimelockReward,
    },
    timelock_staking::QueryMsg as TimelockStakingQueryMsg,
};

use crate::{
    math::{calculate_eclipastro_amount_for_lp, calculate_eclipastro_staked},
    state::{
        CONFIG, LP_LOCKUP_INFO, LP_LOCKUP_STATE, LP_USER_LOCKUP_INFO, OWNER, SINGLE_LOCKUP_INFO,
        SINGLE_LOCKUP_STATE, SINGLE_USER_LOCKUP_INFO,
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
                astro_amount_in_lockups: lockup_info.astro_amount_in_lockups,
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
                astro_amount_in_lockups: lockup_info.astro_amount_in_lockups,
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
        total_eclip_incentives: state.total_eclip_incentives,
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
        total_eclip_incentives: state.total_eclip_incentives,
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

    if state.is_staked == true {
        let flexible_staking_reward_response: FlexibleReward = deps.querier.query_wasm_smart(
            &cfg.flexible_staking.clone().unwrap(),
            &FlexibleStakingQueryMsg::Reward {
                user: env.contract.address.to_string(),
            },
        )?;

        let lock_staking_reward_response: Vec<TimelockReward> = deps.querier.query_wasm_smart(
            &cfg.timelock_staking.clone().unwrap(),
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
            &cfg.reward_distributor.unwrap().to_string(),
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

        let reward_weights = if state.reward_weights.len() > 0 {
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

                let eclipastro_staked = match user_lockup_info.unlock_flag {
                    true => Uint128::zero(),
                    false => calculate_eclipastro_staked(
                        user_lockup_info.astro_amount_in_lockups,
                        user_lockup_info.xastro_amount_in_lockups,
                        state.conversion_rate,
                    )
                    .unwrap(),
                };
                let pending_eclipastro_reward = eclipastro_staked
                    * (eclipastro_reward_weight
                        - user_lockup_info
                            .reward_weights
                            .iter()
                            .find(|w| {
                                w.asset.equal(&AssetInfo::Token {
                                    contract_addr: cfg.eclipastro_token.clone(),
                                })
                            })
                            .unwrap()
                            .weight);
                let pending_eclip_reward = eclipastro_staked
                    * Uint128::from(reward_multiplier)
                    * (eclip_reward_weight
                        - user_lockup_info
                            .reward_weights
                            .iter()
                            .find(|w| {
                                w.asset.equal(&AssetInfo::NativeToken {
                                    denom: cfg.eclip.clone(),
                                })
                            })
                            .unwrap()
                            .weight);
                UserSingleLockupInfoResponse {
                    duration: d,
                    astro_amount_in_lockups: user_lockup_info.astro_amount_in_lockups,
                    xastro_amount_in_lockups: user_lockup_info.xastro_amount_in_lockups,
                    eclipastro_staked: Some(eclipastro_staked),
                    unlock_flag: user_lockup_info.unlock_flag,
                    withdrawal_flag: user_lockup_info.withdrawal_flag,
                    total_eclip_incentives: user_lockup_info.total_eclip_incentives,
                    claimed_eclip_incentives: user_lockup_info.claimed_eclip_incentives,
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
                    countdown_start_at: state.countdown_start_at
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
                    astro_amount_in_lockups: user_lockup_info.astro_amount_in_lockups,
                    xastro_amount_in_lockups: user_lockup_info.xastro_amount_in_lockups,
                    eclipastro_staked: None,
                    unlock_flag: user_lockup_info.unlock_flag,
                    withdrawal_flag: user_lockup_info.withdrawal_flag,
                    total_eclip_incentives: user_lockup_info.total_eclip_incentives,
                    claimed_eclip_incentives: user_lockup_info.claimed_eclip_incentives,
                    staking_rewards: vec![],
                    countdown_start_at: state.countdown_start_at
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

    if state.is_staked == true {
        let lp_staking_reward_response: LpRewards = deps.querier.query_wasm_smart(
            &cfg.lp_staking.clone().unwrap(),
            &LpStakingQueryMsg::Reward {
                user: env.contract.address.to_string(),
            },
        )?;

        let pending_eclip_rewards = lp_staking_reward_response.eclip;
        let pending_astro_rewards = lp_staking_reward_response.astro;

        let total_lp_lockup = LP_LOCKUP_INFO
            .range(deps.storage, None, None, Order::Ascending)
            .fold(Uint128::zero(), |acc, i| {
                let (_, lockup_info) = i.unwrap();
                acc + lockup_info.total_staked - lockup_info.total_withdrawed
            });

        let reward_weights = if state.reward_weights.len() > 0 {
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
                let user_lp_staked = match user_lockup_info.unlock_flag {
                    true => Uint128::zero(),
                    false => calculate_eclipastro_amount_for_lp(
                        user_lockup_info.astro_amount_in_lockups,
                        user_lockup_info.xastro_amount_in_lockups,
                        state.conversion_rate,
                    )
                    .unwrap()
                    .multiply_ratio(state.total_lp_lockdrop, state.total_eclipastro),
                };
                let pending_astro_reward = user_lp_staked
                    * (astro_reward_weight
                        - user_lockup_info
                            .reward_weights
                            .iter()
                            .find(|w| {
                                w.asset.equal(&AssetInfo::Token {
                                    contract_addr: cfg.astro_token.clone(),
                                })
                            })
                            .unwrap()
                            .weight);
                let pending_eclip_reward = user_lp_staked
                    * (eclip_reward_weight
                        - user_lockup_info
                            .reward_weights
                            .iter()
                            .find(|w| {
                                w.asset.equal(&AssetInfo::NativeToken {
                                    denom: cfg.eclip.clone(),
                                })
                            })
                            .unwrap()
                            .weight);
                UserLpLockupInfoResponse {
                    duration: d,
                    astro_amount_in_lockups: user_lockup_info.astro_amount_in_lockups,
                    xastro_amount_in_lockups: user_lockup_info.xastro_amount_in_lockups,
                    lp_token_staked: Some(user_lp_staked),
                    unlock_flag: user_lockup_info.unlock_flag,
                    withdrawal_flag: user_lockup_info.withdrawal_flag,
                    total_eclip_incentives: user_lockup_info.total_eclip_incentives,
                    claimed_eclip_incentives: user_lockup_info.claimed_eclip_incentives,
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
                    countdown_start_at: state.countdown_start_at
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
                    astro_amount_in_lockups: user_lockup_info.astro_amount_in_lockups,
                    xastro_amount_in_lockups: user_lockup_info.xastro_amount_in_lockups,
                    lp_token_staked: None,
                    unlock_flag: user_lockup_info.unlock_flag,
                    withdrawal_flag: user_lockup_info.withdrawal_flag,
                    total_eclip_incentives: user_lockup_info.total_eclip_incentives,
                    claimed_eclip_incentives: user_lockup_info.claimed_eclip_incentives,
                    staking_rewards: vec![],
                    countdown_start_at: state.countdown_start_at
                }
            })
            .collect::<Vec<UserLpLockupInfoResponse>>())
    }
}
