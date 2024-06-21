use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;
use cw_utils::one_coin;

use crate::{
    error::ContractError,
    msg::InstantiateMsg,
    state::{Config, CONFIG, CONTRACT_NAME, OWNER},
};

const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn try_instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    one_coin(&info)?;

    OWNER.set(deps.branch(), Some(info.sender))?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(
        deps.storage,
        &Config {
            astro_token: msg.astro_token,
            xastro_token: msg.xastro_token,
            eclipastro_token: msg.eclipastro_token,
            astro_generator: msg.astro_generator,
            staking_contract: msg.staking_contract,
            lp_token: msg.lp_token,
            lp_contract: msg.lp_contract,
            converter: msg.converter,
        },
    )?;

    Ok(Response::new().add_attributes([("action", "instantiate astro xastro faucet")]))
}
