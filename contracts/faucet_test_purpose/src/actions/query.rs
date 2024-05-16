use cosmwasm_std::{Addr, Deps, Env, StdResult};

use crate::{
    msg::{ConfigResponse, LastClaimedResponse},
    state::{CONFIG, LAST_CLAIMED, OWNER},
};

pub fn last_claimed(deps: Deps, _env: Env, addr: String) -> StdResult<LastClaimedResponse> {
    Ok(LastClaimedResponse {
        last_claim_at: LAST_CLAIMED.load(deps.storage, &deps.api.addr_validate(&addr)?)?,
    })
}

/// query owner
pub fn query_owner(deps: Deps, _env: Env) -> StdResult<Addr> {
    let owner = OWNER.get(deps)?;
    Ok(owner.unwrap())
}

pub fn query_config(deps: Deps, _env: Env) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse {
        astro_token: config.astro_token,
        xastro_token: config.xastro_token,
        astro_generator: config.astro_generator,
        staking_contract: config.staking_contract,
    })
}
