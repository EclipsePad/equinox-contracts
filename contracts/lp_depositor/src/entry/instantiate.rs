use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;
use equinox_msg::lp_depositor::{Config, InstantiateMsg};

use crate::{
    state::{CONFIG, CONTRACT_NAME, CONTRACT_VERSION},
    ContractError,
};

pub fn try_instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let _ = env;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(
        deps.storage,
        &Config {
            astro: msg.astro,
            xastro: msg.xastro,
            staking_contract: msg.staking_contract,
            eclipastro: deps.api.addr_validate(msg.eclipastro.as_str())?,
            converter_contract: deps.api.addr_validate(msg.converter_contract.as_str())?,
            lp_contract: deps.api.addr_validate(msg.lp_contract.as_str())?,
            lp_token: msg.lp_token,
        },
    )?;

    Ok(Response::new().add_attribute("action", "instantiate config"))
}
