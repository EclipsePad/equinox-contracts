use crate::{
    external_queriers::query_rates_astro_staking,
    math::{calculate_claimable, calculate_eclipastro_amount},
    state::{
        CONFIG, OWNER, REWARD_CONFIG, TOTAL_STAKE_INFO, TREASURY_REWARD, WITHDRAWABLE_BALANCE,
    },
};
use cosmwasm_std::{Addr, Deps, StdResult, Uint128};
use equinox_msg::token_converter::{Config, Reward, RewardConfig, RewardResponse, StakeInfo};

/// query owner
pub fn query_owner(deps: Deps) -> StdResult<Addr> {
    let owner = OWNER.get(deps)?;
    Ok(owner.unwrap())
}

/// query config
pub fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

/// query reward config
pub fn query_reward_config(deps: Deps) -> StdResult<RewardConfig> {
    let config = REWARD_CONFIG.load(deps.storage)?;
    Ok(config)
}

pub fn _query_rewards(deps: Deps) -> StdResult<(RewardResponse, Uint128)> {
    let config = CONFIG.load(deps.storage)?;
    let reward_config = REWARD_CONFIG.load(deps.storage)?;
    let mut treasury_reward = TREASURY_REWARD.load(deps.storage).unwrap_or_default();
    let total_staking = TOTAL_STAKE_INFO.load(deps.storage).unwrap_or_default();

    // ASTRO / xASTRO rate from voter contract
    let rate = query_rates_astro_staking(deps, config.staking_contract.to_string())?;
    // calculate user rewards as xASTRO
    let claimable_xastro = calculate_claimable(
        total_staking.xastro,
        total_staking.astro,
        rate.total_shares,
        rate.total_deposit,
        total_staking.claimed_xastro,
    );

    let users_reward = claimable_xastro.multiply_ratio(reward_config.users, 10000u32);
    let dao_reward_point =
        reward_config.treasury + reward_config.ce_holders + reward_config.stability_pool;
    let dao_claimable = claimable_xastro.checked_sub(users_reward).unwrap();
    // amount to withdraw for staking pools
    let ce_holders_reward =
        dao_claimable.multiply_ratio(reward_config.ce_holders, dao_reward_point);
    let stability_pool_reward =
        dao_claimable.multiply_ratio(reward_config.stability_pool, dao_reward_point);
    // amount to withdraw for staking pools
    treasury_reward = treasury_reward
        .checked_add(
            dao_claimable
                .checked_sub(ce_holders_reward)
                .unwrap()
                .checked_sub(stability_pool_reward)
                .unwrap(),
        )
        .unwrap();
    Ok((
        RewardResponse {
            users_reward: Reward {
                token: config.eclipastro.to_string(),
                amount: calculate_eclipastro_amount(rate, users_reward),
            },
            ce_holders_reward: Reward {
                token: config.xastro.to_string(),
                amount: ce_holders_reward,
            },
            stability_pool_reward: Reward {
                token: config.xastro.to_string(),
                amount: stability_pool_reward,
            },
            treasury_reward: Reward {
                token: config.xastro,
                amount: treasury_reward,
            },
        },
        claimable_xastro,
    ))
}

/// query reward
pub fn query_rewards(deps: Deps) -> StdResult<RewardResponse> {
    let res: (RewardResponse, Uint128) = _query_rewards(deps)?;
    Ok(res.0)
}

/// query treasury reward
pub fn query_treasury_reward(deps: Deps) -> StdResult<Uint128> {
    let res: (RewardResponse, Uint128) = _query_rewards(deps)?;
    Ok(res.0.treasury_reward.amount)
}

/// query treasury reward
pub fn query_withdrawable_balance(deps: Deps) -> StdResult<Uint128> {
    let withdrawable_balance = WITHDRAWABLE_BALANCE.load(deps.storage).unwrap_or_default();
    Ok(withdrawable_balance)
}

pub fn query_stake_info(deps: Deps) -> StdResult<StakeInfo> {
    let total_stake_info = TOTAL_STAKE_INFO.load(deps.storage).unwrap_or_default();
    Ok(total_stake_info)
}
