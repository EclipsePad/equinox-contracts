use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;

use eclipse_base::{
    error::ContractError,
    minter::{
        msg::InstantiateMsg,
        state::{CONFIG, CONTRACT_NAME, CW20_CODE_ID},
        types::Config,
    },
};

const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn try_instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(
        deps.storage,
        &Config {
            admin: info.sender.to_owned(),
            cw20_code_id: Some(msg.cw20_code_id.unwrap_or(CW20_CODE_ID)),
        },
    )?;

    Ok(Response::new().add_attribute("action", "try_instantiate"))
}
