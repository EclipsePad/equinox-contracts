use cosmwasm_std::{Addr, Deps, Env, StdResult, Uint128};

use crate::state::{Config, CONFIG, OWNER};
use astroport::staking::QueryMsg as AstroStakingQueryMsg;

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

/// query convert ratio
pub fn query_convert_ratio(deps: Deps, _env: Env) -> StdResult<(Uint128, Uint128)> {
    let config = CONFIG.load(deps.storage)?;
    // xASTRO amount
    let total_shares: Uint128 = deps.querier.query_wasm_smart(
        &config.staking_contract.to_string(),
        &AstroStakingQueryMsg::TotalShares {},
    )?;
    // ASTRO amount
    let total_deposit: Uint128 = deps.querier.query_wasm_smart(
        &config.staking_contract.to_string(),
        &AstroStakingQueryMsg::TotalDeposit {},
    )?;
    Ok((total_deposit, total_shares))
}
