use cosmwasm_std::{Addr, Deps, Env, StdResult};

use crate::state::{OWNER, TOKEN};

pub fn query_denom(deps: Deps, _env: Env) -> StdResult<String> {
    TOKEN.load(deps.storage)
}

/// query owner
pub fn query_owner(deps: Deps, _env: Env) -> StdResult<Addr> {
    let owner = OWNER.get(deps)?;
    Ok(owner.unwrap())
}
