use cosmwasm_std::{Addr, Deps, Env, StdResult, Uint128};

use crate::state::{CONFIG, OWNER, STAKING, TOTAL_STAKING};
use equinox_msg::{
    flexible_staking::Config, reward_distributor::{QueryMsg as RewardDistributorQueryMsg, UserRewardResponse},
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

/// query user staking
pub fn query_staking(deps: Deps, _env: Env, user: String) -> StdResult<Uint128> {
    let user_staking = STAKING.load(deps.storage, &user).unwrap_or_default();
    Ok(user_staking)
}

/// query total staking
pub fn query_total_staking(deps: Deps, _env: Env) -> StdResult<Uint128> {
    let total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    Ok(total_staking)
}

/// query user reward
pub fn query_reward(deps: Deps, _env: Env, user: String) -> StdResult<UserRewardResponse> {
    let config = CONFIG.load(deps.storage)?;
    let user_reward = deps.querier.query_wasm_smart(
        config.reward_contract.to_string(),
        &RewardDistributorQueryMsg::Reward { user },
    )?;
    Ok(user_reward)
}
