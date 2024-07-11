use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;

use eclipse_base::error::ContractError;

use crate::{
    msg::InstantiateMsg,
    state::{CONTRACT_NAME, INSTANTIATION_DATE},
};

const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn try_instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    INSTANTIATION_DATE.save(deps.storage, &env.block.time.seconds())?;

    Ok(Response::new().add_attribute("action", "try_instantiate"))
}
