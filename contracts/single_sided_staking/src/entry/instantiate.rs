use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;

use crate::{
    config::DEFAULT_TIMELOCK_CONFIG,
    error::ContractError,
    state::{CONFIG, CONTRACT_NAME, CONTRACT_VERSION, OWNER},
};
use equinox_msg::single_sided_staking::{Config, InstantiateMsg};

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
            token: msg.token,
            timelock_config: msg
                .timelock_config
                .unwrap_or(DEFAULT_TIMELOCK_CONFIG.to_vec()),
            voter: deps.api.addr_validate(&msg.voter)?,
            treasury: deps.api.addr_validate(&msg.treasury)?,
            eclip_staking: deps.api.addr_validate(&msg.eclip_staking)?,
            eclip: msg.eclip,
            beclip: deps.api.addr_validate(&msg.beclip)?,
        },
    )?;
    let owner = deps.api.addr_validate(msg.owner.as_str())?;
    OWNER.set(deps.branch(), Some(owner))?;
    Ok(Response::new().add_attributes([("action", "instantiate token converter")]))
}
