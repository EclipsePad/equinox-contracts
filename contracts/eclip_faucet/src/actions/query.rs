use cosmwasm_std::{Deps, Env, StdResult};

use crate::{
    msg::LastClaimedResponse,
    state::{LAST_CLAIMED, TOKEN},
};

pub fn query_denom(deps: Deps, _env: Env) -> StdResult<String> {
    TOKEN.load(deps.storage)
}

pub fn last_claimed(deps: Deps, _env: Env, addr: String) -> StdResult<LastClaimedResponse> {
    Ok(LastClaimedResponse {
        last_claim_at: LAST_CLAIMED.load(deps.storage, &deps.api.addr_validate(&addr)?)?,
    })
}
