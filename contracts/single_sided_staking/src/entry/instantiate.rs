use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    state::{CONFIG, CONTRACT_NAME, CONTRACT_VERSION, OWNER},
};
use equinox_msg::single_sided_staking::{Config, InstantiateMsg, TimeLockConfig};

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
            timelock_config: msg.timelock_config.unwrap_or(vec![
                TimeLockConfig {
                    duration: 0,
                    early_unlock_penalty_bps: 0,
                    reward_multiplier: 1,
                },
                TimeLockConfig {
                    duration: 86400 * 30,
                    early_unlock_penalty_bps: 5000,
                    reward_multiplier: 2,
                },
                TimeLockConfig {
                    duration: 86400 * 30 * 3,
                    early_unlock_penalty_bps: 5000,
                    reward_multiplier: 6,
                },
                TimeLockConfig {
                    duration: 86400 * 30 * 6,
                    early_unlock_penalty_bps: 5000,
                    reward_multiplier: 12,
                },
                TimeLockConfig {
                    duration: 86400 * 30 * 9,
                    early_unlock_penalty_bps: 5000,
                    reward_multiplier: 18,
                },
                TimeLockConfig {
                    duration: 86400 * 365,
                    early_unlock_penalty_bps: 5000,
                    reward_multiplier: 24,
                },
            ]),
            token_converter: msg.token_converter,
            beclip_daily_reward: msg
                .beclip_daily_reward
                .unwrap_or(Uint128::from(1_000_000_000u128)),
            treasury: msg.treasury,
        },
    )?;

    let owner = deps.api.addr_validate(&msg.owner.to_string())?;
    OWNER.set(deps.branch(), Some(owner))?;
    Ok(Response::new().add_attributes([("action", "instantiate token converter")]))
}
