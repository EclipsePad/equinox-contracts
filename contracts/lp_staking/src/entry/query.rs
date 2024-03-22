use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::{Addr, Decimal256, Deps, Env, StdResult, Uint128};
use equinox_msg::lp_staking::{
    AstroportRewardWeight, Config, RewardConfig, TotalStaking, UserAstroportReward,
    UserRewardResponse, UserStaking,
};

use crate::state::{CONFIG, OWNER, REWARD_CONFIG, STAKING, TOTAL_STAKING};

use super::execute::{calculate_eclip_rewards, get_incentive_pending_rewards};

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
pub fn query_total_staking(deps: Deps, env: Env) -> StdResult<TotalStaking> {
    let mut total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    if total_staking.total_staked.gt(&Uint128::zero()) {
        let res =
            update_total_staking_rewards(deps, env.contract.address, env.block.time.seconds())?;
        total_staking = res.0;
    }
    Ok(total_staking)
}

/// query user reward
pub fn query_reward(deps: Deps, env: Env, user: String) -> StdResult<Vec<UserRewardResponse>> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    let mut user_staking = STAKING.load(deps.storage, &user).unwrap_or_default();
    if total_staking.total_staked.gt(&Uint128::zero()) {
        let res =
            update_total_staking_rewards(deps, env.contract.address, env.block.time.seconds())?;
        total_staking = res.0;
    }
    if user_staking.staked.gt(&Uint128::zero()) {
        user_staking = update_user_staking_rewards(deps, user, total_staking)?;
    } else {
        user_staking = initialize_user_staking_rewards(total_staking)?;
    }
    let mut response = vec![];
    for asset in user_staking.astroport_rewards {
        response.push(UserRewardResponse {
            asset: asset.asset,
            amount: asset.amount,
        });
    }
    response.push(UserRewardResponse {
        asset: AssetInfo::NativeToken { denom: cfg.eclip },
        amount: user_staking.pending_eclip_rewards,
    });
    Ok(response)
}

pub fn update_total_staking_rewards(
    deps: Deps,
    contract: Addr,
    current_time: u64,
) -> StdResult<(TotalStaking, Vec<Asset>)> {
    let reward_cfg = REWARD_CONFIG.load(deps.storage)?;
    let mut total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    let astroport_pending_rewards = get_incentive_pending_rewards(deps, contract)?;
    let eclip_pending_rewards = calculate_eclip_rewards(deps, current_time)?;
    let mut eclipse_rewards = vec![];
    for reward in astroport_pending_rewards {
        let mut is_exist = false;
        let user_reward = reward.amount.multiply_ratio(reward_cfg.users, 10_000u32);
        let eclipse_reward = Asset {
            info: reward.info.clone(),
            amount: reward.amount.checked_sub(user_reward).unwrap(),
        };
        eclipse_rewards.push(eclipse_reward.clone());
        total_staking.astroport_reward_weights = total_staking
            .astroport_reward_weights
            .into_iter()
            .map(|mut a| {
                if a.asset.equal(&reward.info) {
                    a.reward_weight = a
                        .reward_weight
                        .checked_add(Decimal256::from_ratio(
                            user_reward,
                            total_staking.total_staked,
                        ))
                        .unwrap();
                    is_exist = true;
                }
                a
            })
            .collect::<Vec<AstroportRewardWeight>>();
        if !is_exist {
            total_staking
                .astroport_reward_weights
                .push(AstroportRewardWeight {
                    asset: reward.info,
                    reward_weight: Decimal256::from_ratio(user_reward, total_staking.total_staked),
                })
        }
    }
    total_staking.eclip_reward_weight = total_staking
        .eclip_reward_weight
        .checked_add(Decimal256::from_ratio(
            eclip_pending_rewards,
            total_staking.total_staked,
        ))
        .unwrap();
    Ok((total_staking, eclipse_rewards))
}

pub fn update_user_staking_rewards(
    deps: Deps,
    user: String,
    total_staking: TotalStaking,
) -> StdResult<UserStaking> {
    let mut user_staking = STAKING.load(deps.storage, &user)?;
    for reward_weight_data in total_staking.astroport_reward_weights {
        let mut is_exist = false;
        user_staking.astroport_rewards = user_staking
            .astroport_rewards
            .into_iter()
            .map(|mut r| {
                if r.asset.equal(&reward_weight_data.asset) {
                    r.amount = r
                        .amount
                        .checked_add(
                            reward_weight_data
                                .reward_weight
                                .checked_sub(r.reward_weight)
                                .unwrap_or_default()
                                .checked_mul(Decimal256::from_ratio(user_staking.staked, 1u128))
                                .unwrap()
                                .to_uint_floor()
                                .try_into()?,
                        )
                        .unwrap();
                    r.reward_weight = reward_weight_data.reward_weight;
                    is_exist = true;
                }
                Ok(r)
            })
            .collect::<StdResult<Vec<UserAstroportReward>>>()
            .unwrap();
        if !is_exist {
            user_staking.astroport_rewards.push(UserAstroportReward {
                asset: reward_weight_data.asset,
                amount: reward_weight_data
                    .reward_weight
                    .checked_mul(Decimal256::from_ratio(user_staking.staked, 1u128))
                    .unwrap()
                    .to_uint_floor()
                    .try_into()?,
                reward_weight: reward_weight_data.reward_weight,
            });
        }
    }
    user_staking.pending_eclip_rewards = user_staking
        .pending_eclip_rewards
        .checked_add(
            total_staking
                .eclip_reward_weight
                .checked_sub(user_staking.eclip_reward_weight)
                .unwrap_or_default()
                .checked_mul(Decimal256::from_ratio(user_staking.staked, 1u128))
                .unwrap()
                .to_uint_floor()
                .try_into()?,
        )
        .unwrap();
    user_staking.eclip_reward_weight = total_staking.eclip_reward_weight;
    Ok(user_staking)
}

pub fn initialize_user_staking_rewards(total_staking: TotalStaking) -> StdResult<UserStaking> {
    let mut user_staking = UserStaking {
        staked: Uint128::zero(),
        astroport_rewards: vec![],
        eclip_reward_weight: total_staking.eclip_reward_weight,
        pending_eclip_rewards: Uint128::zero(),
    };
    for reward_weight_data in total_staking.astroport_reward_weights {
        user_staking.astroport_rewards.push(UserAstroportReward {
            asset: reward_weight_data.asset,
            amount: Uint128::zero(),
            reward_weight: reward_weight_data.reward_weight,
        });
    }
    user_staking.eclip_reward_weight = total_staking.eclip_reward_weight;
    Ok(user_staking)
}
