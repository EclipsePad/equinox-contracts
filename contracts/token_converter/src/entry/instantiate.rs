use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;
use equinox_msg::token_converter::{Config, InstantiateMsg, RewardConfig};

use crate::{
    error::ContractError,
    state::{CONFIG, CONTRACT_NAME, OWNER, REWARD_CONFIG},
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
            token_in: deps.api.addr_validate(&msg.token_in)?,
            token_out: deps.api.addr_validate(&msg.token_out)?,
            xtoken: deps.api.addr_validate(&msg.xtoken)?,
            vxtoken_holder: deps.api.addr_validate(&msg.vxtoken_holder)?,
            treasury: deps.api.addr_validate(&msg.treasury)?,
            stability_pool: deps.api.addr_validate(&msg.stability_pool)?,
            staking_reward_distributor: deps.api.addr_validate(&msg.staking_reward_distributor)?,
            ce_reward_distributor: deps.api.addr_validate(&msg.ce_reward_distributor)?,
        },
    )?;

    REWARD_CONFIG.save(
        deps.storage,
        &RewardConfig {
            users: 8000,         // 80%
            treasury: 1350,      // 20% * 67.5%
            ce_holders: 400,     // 20% * 20%
            stability_pool: 250, // 20% * 12.5%
        },
    )?;

    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.set(deps.branch(), Some(owner))?;
    Ok(Response::new().add_attributes([("action", "instantiate token converter")]))
}
