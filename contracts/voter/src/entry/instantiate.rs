use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;
use equinox_msg::voter::{Config, InstantiateMsg};

use crate::{
    error::ContractError,
    state::{CONFIG, CONTRACT_NAME, OWNER},
};

const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn try_instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(
        deps.storage,
        &Config {
            base_token: deps.api.addr_validate(&msg.base_token)?,
            xtoken: deps.api.addr_validate(&msg.xtoken)?,
            vxtoken: deps.api.addr_validate(&msg.vxtoken)?,
            staking_contract: deps.api.addr_validate(&msg.staking_contract)?,
            converter_contract: deps.api.addr_validate(&msg.converter_contact)?,
            gauge_contract: Addr::unchecked(""),
            astroport_gauge_contract: Addr::unchecked(""),
        },
    )?;

    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.set(deps.branch(), Some(owner))?;
    Ok(Response::new().add_attributes([("action", "instantiate vxASTRO holder")]))
}
