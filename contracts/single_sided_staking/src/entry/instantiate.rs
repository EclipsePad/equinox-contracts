use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};
use cw2::set_contract_version;

use crate::{
    config::{DEFAULT_BECLIP_DAILY_REWARD, DEFAULT_TIMELOCK_CONFIG}, error::ContractError, state::{CONFIG, CONTRACT_NAME, CONTRACT_VERSION, OWNER}
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
            beclip: msg.beclip,
            timelock_config: msg.timelock_config.unwrap_or(DEFAULT_TIMELOCK_CONFIG.to_vec()),
            token_converter: msg.token_converter,
            beclip_daily_reward: msg
                .beclip_daily_reward
                .unwrap_or(Uint128::from(DEFAULT_BECLIP_DAILY_REWARD)),
            treasury: msg.treasury,
        },
    )?;

    let owner = deps.api.addr_validate(&msg.owner.to_string())?;
    OWNER.set(deps.branch(), Some(owner))?;
    Ok(Response::new().add_attributes([("action", "instantiate token converter")]))
}
