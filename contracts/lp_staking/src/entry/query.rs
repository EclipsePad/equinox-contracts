use astroport::{asset::Asset, incentives::QueryMsg as IncentivesQueryMsg};
use cosmwasm_std::{Addr, Decimal256, Deps, Env, StdResult, Uint128};
use equinox_msg::lp_staking::{
    Config, RewardAmount, RewardConfig, RewardWeight, UserStaking, VaultRewards,
};

use crate::state::{
    CONFIG, LAST_CLAIMED, OWNER, REWARD_CONFIG, REWARD_WEIGHTS, STAKING, TOTAL_STAKING,
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
pub fn query_reward_config(deps: Deps, _env: Env) -> StdResult<RewardConfig> {
    let config = REWARD_CONFIG.load(deps.storage)?;
    Ok(config)
}

/// query user staking
pub fn query_staking(deps: Deps, _env: Env, user: String) -> StdResult<UserStaking> {
    let user_staking = STAKING.load(deps.storage, &user).unwrap_or_default();
    Ok(user_staking)
}

/// query total staking
pub fn query_total_staking(deps: Deps, _env: Env) -> StdResult<Uint128> {
    let total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    Ok(total_staking)
}

/// query user reward
pub fn query_reward(deps: Deps, env: Env, user: String) -> StdResult<Vec<RewardAmount>> {
    let user_staking = STAKING.load(deps.storage, &user).unwrap_or_default();
    if user_staking.staked.is_zero() {
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
    deps.querier.query_wasm_smart(
        &cfg.astroport_generator,
        &IncentivesQueryMsg::PendingRewards {
            lp_token: cfg.lp_token.to_string(),
            user: contract.to_string(),
        },
    )
}

pub fn calculate_vault_rewards(deps: Deps, current_time: u64) -> StdResult<VaultRewards> {
    let cfg = CONFIG.load(deps.storage)?;
    let last_claimed = LAST_CLAIMED.load(deps.storage).unwrap_or(current_time);
    Ok(VaultRewards {
        eclip: cfg
            .rewards
            .eclip
            .daily_reward
            .multiply_ratio(current_time - last_claimed, 86400u64),
        beclip: cfg
            .rewards
            .beclip
            .daily_reward
            .multiply_ratio(current_time - last_claimed, 86400u64),
    })
}

pub fn calculate_pending_eclipse_rewards(
    deps: Deps,
    astroport_rewards: Vec<Asset>,
) -> StdResult<Vec<Asset>> {
    let reward_cfg = REWARD_CONFIG.load(deps.storage)?;
    let mut eclipse_rewards = vec![];
    for reward in astroport_rewards {
        let user_reward = reward.amount.multiply_ratio(reward_cfg.users, 10_000u32);
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
    let config = CONFIG.load(deps.storage)?;
    let reward_cfg = REWARD_CONFIG.load(deps.storage)?;
    let total_staking = TOTAL_STAKING.load(deps.storage)?;
    let mut reward_weights = REWARD_WEIGHTS.load(deps.storage).unwrap_or_default();
    let mut is_beclip = false;
    let mut is_eclip = false;
    let beclip_user_reward_weight = Decimal256::from_ratio(vault_rewards.beclip, total_staking);
    let eclip_user_reward_weight = Decimal256::from_ratio(vault_rewards.eclip, total_staking);
    for reward in astroport_rewards {
        let mut is_exist = false;
        let user_reward = reward.amount.multiply_ratio(reward_cfg.users, 10_000u32);
        reward_weights = reward_weights
            .into_iter()
            .map(|mut a| {
                if a.info.equal(&reward.info) {
                    a.reward_weight = a
                        .reward_weight
                        .checked_add(Decimal256::from_ratio(user_reward, total_staking))
                        .unwrap();
                    if reward.info.equal(&config.rewards.beclip.info) {
                        a.reward_weight += beclip_user_reward_weight;
                        is_beclip = true;
                    }
                    if reward.info.equal(&config.rewards.eclip.info) {
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
            if reward.info.equal(&config.rewards.beclip.info) {
                reward_weight += beclip_user_reward_weight;
                is_beclip = true;
            }
            if reward.info.equal(&config.rewards.eclip.info) {
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
                if a.info.equal(&config.rewards.beclip.info) {
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
                info: config.rewards.beclip.info,
                reward_weight: beclip_user_reward_weight,
            })
        }
    }
    if !is_eclip {
        let mut is_exist = false;
        reward_weights = reward_weights
            .into_iter()
            .map(|mut a| {
                if a.info.equal(&config.rewards.eclip.info) {
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
                info: config.rewards.eclip.info,
                reward_weight: eclip_user_reward_weight,
            })
        }
    }
    Ok(reward_weights)
}
