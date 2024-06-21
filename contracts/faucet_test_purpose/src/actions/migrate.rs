use crate::{error::ContractError, msg::MigrateMsg};
use cosmwasm_std::{DepsMut, Env, Response};

pub fn migrate_contract(
    _deps: DepsMut,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, ContractError> {
    Ok(Response::new())
}
