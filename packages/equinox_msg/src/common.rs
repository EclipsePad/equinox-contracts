use cosmwasm_schema::cw_serde;
use cosmwasm_std::{attr, Addr, DepsMut, Env, MessageInfo, Response, StdError, StdResult};
use cw_storage_plus::Item;

const MAX_PROPOSAL_TTL: u64 = 1209600;

#[cw_serde]
pub struct OwnershipProposal {
    /// The newly proposed contract owner
    pub owner: Addr,
    /// Time until the proposal to change ownership expires
    pub ttl: u64,
}

pub fn propose_new_owner(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    new_owner: String,
    expires_in: u64,
    owner: Addr,
    proposal: Item<OwnershipProposal>,
) -> StdResult<Response> {
    // Permission check
    if info.sender != owner {
        return Err(StdError::generic_err("Unauthorized"));
    }

    let new_owner = deps.api.addr_validate(&new_owner)?;

    // Check that the new owner is not the same as the current one
    if new_owner == owner {
        return Err(StdError::generic_err("New owner cannot be same"));
    }

    if MAX_PROPOSAL_TTL < expires_in {
        return Err(StdError::generic_err(format!(
            "Parameter expires_in cannot be higher than {}",
            MAX_PROPOSAL_TTL
        )));
    }

    proposal.save(
        deps.storage,
        &OwnershipProposal {
            owner: new_owner.clone(),
            ttl: env.block.time.seconds() + expires_in,
        },
    )?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "propose_new_owner"),
        attr("new_owner", new_owner),
    ]))
}

pub fn drop_ownership_proposal(
    deps: DepsMut,
    info: MessageInfo,
    owner: Addr,
    proposal: Item<OwnershipProposal>,
) -> StdResult<Response> {
    // Permission check
    if info.sender != owner {
        return Err(StdError::generic_err("Unauthorized"));
    }

    proposal.remove(deps.storage);

    Ok(Response::new().add_attributes(vec![attr("action", "drop_ownership_proposal")]))
}
