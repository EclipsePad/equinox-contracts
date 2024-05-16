use cosmwasm_std::{Addr, Deps, Env, StdResult, Uint128};
use equinox_msg::voter::Config;

use crate::state::{CONFIG, OWNER};

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

/// query voting power
pub fn query_voting_power(_deps: Deps, _env: Env) -> StdResult<Uint128> {
    // to do (balance of vxASTRO)
    Ok(Uint128::zero())
}
