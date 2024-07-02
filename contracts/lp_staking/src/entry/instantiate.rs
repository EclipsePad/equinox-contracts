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
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // update config
    CONFIG.save(
        deps.storage,
        &Config {
            lp_token: msg.lp_token,
            lp_contract: msg.lp_contract,
            rewards: msg.rewards,
            astro: msg.astro,
            xastro: msg.xastro,
            astro_staking: msg.astro_staking,
            converter: msg.converter,
            astroport_incentives: msg.astroport_incentives,
            treasury: msg.treasury,
            stability_pool: msg.stability_pool,
            ce_reward_distributor: msg.ce_reward_distributor,
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
    let owner = msg.owner.unwrap_or(info.sender);
    OWNER.set(deps.branch(), Some(owner))?;
    Ok(Response::new())
}
