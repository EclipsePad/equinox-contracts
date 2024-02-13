use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;
use equinox_msg::reward_distributor::{Config, InstantiateMsg};

use crate::{
    error::ContractError,
    state::{CONFIG, CONTRACT_NAME, CONTRACT_VERSION, OWNER},
};

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
            eclipastro: deps.api.addr_validate(&msg.eclipastro)?,
            eclip: msg.eclip,
            flexible_staking: deps.api.addr_validate(&msg.flexible_staking)?,
            timelock_staking: deps.api.addr_validate(&msg.timelock_staking)?,
            token_converter: deps.api.addr_validate(&msg.token_converter)?,
            eclip_daily_reward: msg.eclip_daily_reward,
            locking_reward_config: msg.locking_reward_config,
        },
    )?;

    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.set(deps.branch(), Some(owner))?;
    Ok(Response::new().add_attributes([("action", "instantiate token converter")]))
}
