use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;
use equinox_msg::lp_staking::{Config, InstantiateMsg, RewardConfig};

use crate::{
    error::ContractError,
    state::{CONFIG, CONTRACT_NAME, CONTRACT_VERSION, OWNER, REWARD_CONFIG},
};

pub fn try_instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let ce_reward_distributor = match msg.ce_reward_distributor {
        Some(contract_addr) => Some(deps.api.addr_validate(&contract_addr)?),
        None => None,
    };
    // update config
    CONFIG.save(
        deps.storage,
        &Config {
            lp_token: deps.api.addr_validate(&msg.lp_token)?,
            eclip: msg.eclip,
            astro: deps.api.addr_validate(&msg.astro)?,
            eclip_daily_reward: msg.eclip_daily_reward,
            astroport_generator: deps.api.addr_validate(&msg.astroport_generator)?,
            treasury: deps.api.addr_validate(&msg.treasury)?,
            stability_pool: deps.api.addr_validate(&msg.stability_pool)?,
            ce_reward_distributor,
        },
    )?;
    // update reward config
    REWARD_CONFIG.save(
        deps.storage,
        &RewardConfig {
            users: 8000,
            treasury: 1350,
            ce_holders: 400,
            stability_pool: 250,
        },
    )?;
    // update owner
    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.set(deps.branch(), Some(owner))?;
    Ok(Response::new())
}
