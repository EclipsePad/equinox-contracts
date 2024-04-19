use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};
use cw2::set_contract_version;
use equinox_msg::reward_distributor::{Config, InstantiateMsg, LockingRewardConfig};

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
            eclip_daily_reward: msg
                .eclip_daily_reward
                .unwrap_or(Uint128::from(1_000_000_000u128)),
            locking_reward_config: msg.locking_reward_config.unwrap_or(vec![
                LockingRewardConfig {
                    duration: 0,
                    multiplier: 1,
                },
                LockingRewardConfig {
                    duration: 86400 * 30,
                    multiplier: 2,
                },
                LockingRewardConfig {
                    duration: 86400 * 30 * 3,
                    multiplier: 3,
                },
                LockingRewardConfig {
                    duration: 86400 * 30 * 6,
                    multiplier: 4,
                },
                LockingRewardConfig {
                    duration: 86400 * 30 * 9,
                    multiplier: 5,
                },
                LockingRewardConfig {
                    duration: 86400 * 365,
                    multiplier: 6,
                },
            ]),
        },
    )?;

    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.set(deps.branch(), Some(owner))?;
    Ok(Response::new().add_attributes([("action", "instantiate token converter")]))
}
