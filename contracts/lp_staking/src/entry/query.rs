use std::cmp::{max, min};

use astroport::{
    asset::{Asset, AssetInfo},
    incentives::QueryMsg as IncentivesQueryMsg,
};
use cosmwasm_std::{
    Addr, BankQuery, Coin, Decimal256, Deps, Env, Order, QuerierWrapper, QueryRequest, StdResult,
    SupplyResponse, Uint128,
};
use cw_storage_plus::Bound;
use equinox_msg::{
    lp_staking::{
        Config, Reward, RewardAmount, RewardDistribution, RewardWeight, UserStaking, VaultRewards,
    },
    single_sided_staking::UnbondedItem,
};

use crate::state::{
    BLACK_LIST, BLACK_LIST_REWARDS, CONFIG, LAST_CLAIMED, OWNER, REWARD, REWARD_DISTRIBUTION,
    REWARD_WEIGHTS, STAKING, TOTAL_STAKING, USER_UNBONDED,
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

/// query reward config
pub fn query_reward_distribution(deps: Deps, _env: Env) -> StdResult<RewardDistribution> {
    let config = REWARD_DISTRIBUTION.load(deps.storage)?;
    Ok(config)
}

/// query user staking
pub fn query_staking(deps: Deps, _env: Env, user: String) -> StdResult<UserStaking> {
    let user_staking = STAKING.load(deps.storage, &user).unwrap_or_default();
    Ok(user_staking)
}

pub fn query_unbonded(deps: Deps, _env: Env, user: String) -> StdResult<Vec<UnbondedItem>> {
    let user = deps.api.addr_validate(&user)?;
    Ok(USER_UNBONDED.load(deps.storage, &user).unwrap_or_default())
}

/// query total staking
pub fn query_total_staking(deps: Deps, _env: Env) -> StdResult<Uint128> {
    let total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    Ok(total_staking)
}

/// query user reward
pub fn query_reward(deps: Deps, env: Env, user: String) -> StdResult<Vec<RewardAmount>> {
    let user_staking = STAKING.load(deps.storage, &user).unwrap_or_default();
    let blacklist = BLACK_LIST.load(deps.storage).unwrap_or_default();
    if user_staking.staked.is_zero() || blacklist.contains(&user) {
        Ok(vec![])
    } else {
        let astroport_rewards = calculate_incentive_pending_rewards(deps, env.contract.address)?;
        let vault_rewards = calculate_vault_rewards(deps, env.block.time.seconds())?;
        let updated_reward_weights =
            calculate_updated_reward_weights(deps, astroport_rewards, vault_rewards)?;
        let user_rewards = calculate_user_staking_rewards(deps, user, updated_reward_weights)?;
        Ok(user_rewards)
    }
}

pub fn query_reward_weights(deps: Deps, env: Env) -> StdResult<Vec<RewardWeight>> {
    let astroport_rewards = calculate_incentive_pending_rewards(deps, env.contract.address)?;
    let vault_rewards = calculate_vault_rewards(deps, env.block.time.seconds())?;
    let updated_reward_weights =
        calculate_updated_reward_weights(deps, astroport_rewards, vault_rewards)?;
    Ok(updated_reward_weights)
}

pub fn query_user_reward_weights(
    deps: Deps,
    _env: Env,
    user: String,
) -> StdResult<Vec<RewardWeight>> {
    let user_staking = STAKING.load(deps.storage, &user).unwrap_or_default();
    Ok(user_staking.reward_weights)
}

pub fn query_blacklist(deps: Deps) -> StdResult<Vec<String>> {
    Ok(BLACK_LIST.load(deps.storage).unwrap_or_default())
}

pub fn query_blacklist_rewards(deps: Deps, env: Env) -> StdResult<Vec<RewardAmount>> {
    let blacklist = BLACK_LIST.load(deps.storage).unwrap_or_default();
    let block_time = env.block.time.seconds();
    let mut blacklist_rewards = BLACK_LIST_REWARDS.load(deps.storage).unwrap_or_default();
    let astroport_rewards = calculate_incentive_pending_rewards(deps, env.contract.address)?;
    let vault_rewards = calculate_vault_rewards(deps, block_time)?;
    let updated_reward_weights =
        calculate_updated_reward_weights(deps, astroport_rewards, vault_rewards)?;
    for user in blacklist.iter() {
        let user_rewards =
            calculate_user_staking_rewards(deps, user.to_string(), updated_reward_weights.clone())?;
        for user_reward in user_rewards {
            let position = blacklist_rewards
                .iter()
                .position(|x| x.info.equal(&user_reward.info));
            match position {
                Some(p) => {
                    blacklist_rewards[p].amount += user_reward.amount;
                }
                None => {
                    blacklist_rewards.push(user_reward);
                }
            }
        }
    }
    Ok(blacklist_rewards)
}

pub fn calculate_user_staking_rewards(
    deps: Deps,
    user: String,
    reward_weights: Vec<RewardWeight>,
) -> StdResult<Vec<RewardAmount>> {
    let user_staking = STAKING.load(deps.storage, &user)?;
    let mut user_rewards = vec![];
    for reward_weight in reward_weights {
        let user_reward = user_staking
            .reward_weights
            .clone()
            .into_iter()
            .find(|r| reward_weight.info == r.info);
        let user_reward_weight = match user_reward {
            Some(r) => r.reward_weight,
            None => Decimal256::zero(),
        };
        let user_reward_amount: Uint128 = reward_weight
            .reward_weight
            .checked_sub(user_reward_weight)
            .unwrap_or_default()
            .checked_mul(Decimal256::from_ratio(user_staking.staked, 1u128))
            .unwrap()
            .to_uint_floor()
            .try_into()?;
        user_rewards.push(RewardAmount {
            info: reward_weight.info,
            amount: user_reward_amount,
        });
    }
    Ok(user_rewards)
}

pub fn calculate_incentive_pending_rewards(deps: Deps, contract: Addr) -> StdResult<Vec<Asset>> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(deps
        .querier
        .query_wasm_smart(
            &cfg.astroport_incentives,
            &IncentivesQueryMsg::PendingRewards {
                lp_token: cfg.lp_token.to_string(),
                user: contract.to_string(),
            },
        )
        .unwrap_or_default())
}

pub fn calculate_vault_rewards(deps: Deps, current_time: u64) -> StdResult<VaultRewards> {
    let last_claimed = LAST_CLAIMED.load(deps.storage).unwrap_or(current_time);
    let (eclip_reward, beclip_reward) =
        calculate_eclip_beclip_reward(deps, last_claimed, current_time)?;
    Ok(VaultRewards {
        eclip: eclip_reward,
        beclip: beclip_reward,
    })
}

pub fn calculate_pending_eclipse_rewards(
    deps: Deps,
    astroport_rewards: Vec<Asset>,
) -> StdResult<Vec<Asset>> {
    let rwd_dst = REWARD_DISTRIBUTION.load(deps.storage)?;
    let mut eclipse_rewards = vec![];
    for reward in astroport_rewards {
        let user_reward = reward.amount.multiply_ratio(rwd_dst.users, 10_000u32);
        let eclipse_reward = Asset {
            info: reward.info.clone(),
            amount: reward.amount.checked_sub(user_reward).unwrap(),
        };
        eclipse_rewards.push(eclipse_reward.clone());
    }
    Ok(eclipse_rewards)
}

pub fn calculate_updated_reward_weights(
    deps: Deps,
    astroport_rewards: Vec<Asset>,
    vault_rewards: VaultRewards,
) -> StdResult<Vec<RewardWeight>> {
    let total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    let rwd_dist = REWARD_DISTRIBUTION.load(deps.storage)?;
    let cfg = CONFIG.load(deps.storage)?;
    if total_staking.is_zero() {
        return Ok(vec![]);
    }
    let mut reward_weights = REWARD_WEIGHTS.load(deps.storage).unwrap_or_default();
    let mut is_beclip = false;
    let mut is_eclip = false;
    let beclip_user_reward_weight = Decimal256::from_ratio(vault_rewards.beclip, total_staking);
    let eclip_user_reward_weight = Decimal256::from_ratio(vault_rewards.eclip, total_staking);
    for reward in astroport_rewards {
        let mut is_exist = false;
        let user_reward = reward.amount.multiply_ratio(rwd_dist.users, 10_000u32);
        reward_weights = reward_weights
            .into_iter()
            .map(|mut a| {
                if a.info.equal(&reward.info) {
                    a.reward_weight = a
                        .reward_weight
                        .checked_add(Decimal256::from_ratio(user_reward, total_staking))
                        .unwrap();
                    if reward.info.to_string() == cfg.beclip {
                        a.reward_weight += beclip_user_reward_weight;
                        is_beclip = true;
                    }
                    if reward.info.to_string() == cfg.eclip.clone() {
                        a.reward_weight += eclip_user_reward_weight;
                        is_eclip = true;
                    }
                    is_exist = true;
                }
                a
            })
            .collect::<Vec<RewardWeight>>();
        if !is_exist {
            let mut reward_weight = Decimal256::from_ratio(user_reward, total_staking);
            if reward.info.to_string() == cfg.beclip {
                reward_weight += beclip_user_reward_weight;
                is_beclip = true;
            }
            if reward.info.to_string() == cfg.eclip.clone() {
                reward_weight += eclip_user_reward_weight;
                is_eclip = true;
            }
            reward_weights.push(RewardWeight {
                info: reward.info,
                reward_weight,
            })
        }
    }
    if !is_beclip {
        let mut is_exist = false;
        reward_weights = reward_weights
            .into_iter()
            .map(|mut a| {
                if a.info.to_string() == cfg.beclip {
                    a.reward_weight = a
                        .reward_weight
                        .checked_add(beclip_user_reward_weight)
                        .unwrap();
                    is_exist = true;
                }
                a
            })
            .collect::<Vec<RewardWeight>>();
        if !is_exist {
            reward_weights.push(RewardWeight {
                info: AssetInfo::Token {
                    contract_addr: cfg.beclip,
                },
                reward_weight: beclip_user_reward_weight,
            })
        }
    }
    if !is_eclip {
        let mut is_exist = false;
        reward_weights = reward_weights
            .into_iter()
            .map(|mut a| {
                if a.info.to_string() == cfg.eclip.clone() {
                    a.reward_weight = a
                        .reward_weight
                        .checked_add(eclip_user_reward_weight)
                        .unwrap();
                    is_exist = true;
                }
                a
            })
            .collect::<Vec<RewardWeight>>();
        if !is_exist {
            reward_weights.push(RewardWeight {
                info: AssetInfo::NativeToken { denom: cfg.eclip },
                reward_weight: eclip_user_reward_weight,
            })
        }
    }
    Ok(reward_weights)
}

pub fn check_native_token_denom(querier: &QuerierWrapper, denom: String) -> StdResult<bool> {
    let total_supply = query_native_token_supply(querier, denom)?;
    Ok(!total_supply.amount.is_zero())
}

pub fn query_native_token_supply(querier: &QuerierWrapper, denom: String) -> StdResult<Coin> {
    let supply: SupplyResponse = querier.query(&QueryRequest::Bank(BankQuery::Supply { denom }))?;
    Ok(supply.amount)
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
