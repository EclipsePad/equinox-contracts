use cosmwasm_std::{Addr, Deps, Env, StdResult};

use crate::state::{Config, RewardConfig, CONFIG, OWNER, REWARD_CONFIG};

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

/// query rewards
pub fn query_rewards(_deps: Deps, _env: Env) -> StdResult<()> {
    // to do
    Ok(())
}

/// query reward
pub fn query_reward(_deps: Deps, _env: Env, _user: String) -> StdResult<()> {
    // to do
    Ok(())
}
