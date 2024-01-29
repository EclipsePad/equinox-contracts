use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::InstantiateMsg,
    state::{Config, RewardConfig, CONFIG, CONTRACT_NAME, OWNER, REWARD_CONFIG},
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
            base_token: deps.api.addr_validate(&msg.base_token)?,
            token: deps.api.addr_validate(&msg.token)?,
            voter: deps.api.addr_validate(&msg.voter)?,
            treasury: deps.api.addr_validate(&msg.treasury)?,
            lp_staking_vault: deps.api.addr_validate(&msg.lp_staking_vault)?,
            staking_reward_distributor: deps.api.addr_validate(&msg.staking_reward_distributor)?,
            pos_reward_distributor: deps.api.addr_validate(&msg.pos_reward_distributor)?,
        }
    )?;

    REWARD_CONFIG.save(
        deps.storage,
        &RewardConfig {
            users: 8000, // 80%
            treasury: 1350, // 20% * 67.5%
            voters: 400, // 20% * 20%
            stability_pool: 250, // 20% * 12.5%
        }
    )?;

    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.set(deps.branch(), Some(owner))?;
    Ok(Response::new()
        .add_attributes([("action", "instantiate token converter")]))
}