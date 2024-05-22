use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use equinox_msg::lp_staking::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

use crate::{
    entry::{
        execute::{
            _handle_callback, claim, receive_cw20, unstake, update_config, update_owner,
            update_reward_config,
        },
        instantiate::try_instantiate,
        query::{
            query_config, query_owner, query_reward, query_reward_config, query_reward_weights,
            query_staking, query_total_staking,
        },
    },
    error::ContractError,
};

// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    try_instantiate(deps, env, info, msg)
}

/// Exposes execute functions available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { config } => update_config(deps, env, info, config),
        ExecuteMsg::UpdateOwner { owner } => update_owner(deps, env, info, owner),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Claim {} => claim(deps, env, info.clone(), info.sender.to_string()),
        ExecuteMsg::Callback(msg) => _handle_callback(deps, env, info, msg),
        ExecuteMsg::Unstake { amount, recipient } => unstake(deps, env, info, amount, recipient),
        ExecuteMsg::UpdateRewardConfig { config } => update_reward_config(deps, env, info, config),
    }
}

/// Exposes queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps, env)?)?),
        QueryMsg::RewardConfig {} => Ok(to_json_binary(&query_reward_config(deps, env)?)?),
        QueryMsg::Owner {} => Ok(to_json_binary(&query_owner(deps, env)?)?),
        QueryMsg::Staking { user } => Ok(to_json_binary(&query_staking(deps, env, user)?)?),
        QueryMsg::TotalStaking {} => Ok(to_json_binary(&query_total_staking(deps, env)?)?),
        QueryMsg::Reward { user } => Ok(to_json_binary(&query_reward(deps, env, user)?)?),
        QueryMsg::RewardWeights {} => Ok(to_json_binary(&query_reward_weights(deps, env)?)?),
    }
}

/// Manages contract migration.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::new())
}
