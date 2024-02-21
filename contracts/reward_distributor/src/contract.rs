use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

use crate::{
    entry::{
        execute::{claim, restake, stake, unstake, update_config, update_owner},
        instantiate::try_instantiate,
        query::{query_config, query_owner, query_reward},
    },
    error::ContractError,
};
use equinox_msg::reward_distributor::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

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
        ExecuteMsg::FlexibleStake { user, amount } => stake(deps, env, info, user, amount, 0u64),
        ExecuteMsg::TimelockStake {
            user,
            amount,
            duration,
        } => stake(deps, env, info, user, amount, duration),
        ExecuteMsg::Claim { user } => claim(deps, env, info, user),
        ExecuteMsg::FlexibleUnstake { user, amount } => {
            unstake(deps, env, info, user, amount, 0u64)
        }
        ExecuteMsg::TimelockUnstake {
            user,
            amount,
            duration,
        } => unstake(deps, env, info, user, amount, duration),
        ExecuteMsg::Restake {
            user,
            amount,
            from,
            to,
        } => restake(deps, env, info, user, amount, from, to),
    }
}

/// Exposes queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps, env)?)?),
        QueryMsg::Owner {} => Ok(to_json_binary(&query_owner(deps, env)?)?),
        QueryMsg::Reward { user } => Ok(to_json_binary(&query_reward(deps, env, user)?)?),
    }
}

/// Used for contract migration
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
