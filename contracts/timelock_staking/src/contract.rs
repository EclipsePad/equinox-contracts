use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

use crate::{
    entry::{
        execute::{claim, receive_cw20, restake, unstake, update_config, update_owner},
        instantiate::try_instantiate,
        query::{calculate_penalty, query_config, query_owner, query_reward, query_staking, query_total_staking},
    },
    error::ContractError,
};
use equinox_msg::timelock_staking::{ExecuteMsg, InstantiateMsg, QueryMsg};

// Note, you can use StdResult in some functions where you do not
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
        ExecuteMsg::Claim {} => claim(deps, env, info),
        ExecuteMsg::Unstake {
            duration,
            locked_at,
        } => unstake(deps, env, info, duration, locked_at),
        ExecuteMsg::Restake {
            from_duration,
            locked_at,
            to_duration,
        } => restake(deps, env, info, from_duration, locked_at, to_duration),
    }
}

/// Exposes queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps, env)?)?),
        QueryMsg::Owner {} => Ok(to_json_binary(&query_owner(deps, env)?)?),
        QueryMsg::Staking { user } => Ok(to_json_binary(&query_staking(deps, env, user)?)?),
        QueryMsg::TotalStaking {} => Ok(to_json_binary(&query_total_staking(deps, env)?)?),
        QueryMsg::Reward { user } => Ok(to_json_binary(&query_reward(deps, env, user)?)?),
        QueryMsg::CalculatePenalty { amount, duration, locked_at } => Ok(to_json_binary(&calculate_penalty(deps, env, amount, duration, locked_at)?)?),
    }
}
