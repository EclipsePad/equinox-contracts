use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    state::{CONFIG, CONTRACT_NAME, CONTRACT_VERSION, OWNER},
};
use equinox_msg::timelock_staking::{Config, InstantiateMsg, TimeLockConfig};

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
            token: deps.api.addr_validate(&msg.token)?,
            reward_contract: Addr::unchecked(""),
            timelock_config: msg.timelock_config.unwrap_or(vec![
                TimeLockConfig {
                    duration: 86400 * 30,
                    early_unlock_penalty_bps: 3000,
                },
                TimeLockConfig {
                    duration: 86400 * 30 * 3,
                    early_unlock_penalty_bps: 3000,
                },
                TimeLockConfig {
                    duration: 86400 * 30 * 6,
                    early_unlock_penalty_bps: 3000,
                },
                TimeLockConfig {
                    duration: 86400 * 30 * 9,
                    early_unlock_penalty_bps: 3000,
                },
                TimeLockConfig {
                    duration: 86400 * 365,
                    early_unlock_penalty_bps: 3000,
                },
            ]),
        },
    )?;

    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.set(deps.branch(), Some(owner))?;
    Ok(Response::new().add_attributes([("action", "instantiate token converter")]))
}
