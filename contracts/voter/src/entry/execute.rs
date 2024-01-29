use cosmwasm_std::{from_json, DepsMut, Env, MessageInfo, Response, Uint128};
use cw20::Cw20ReceiveMsg;

use crate::{
    error::ContractError, msg::Cw20HookMsg, state::{Config, OWNER}
};

/// Update config
pub fn update_config(deps: DepsMut, _env: Env, info: MessageInfo, _config: Config) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    // to do
    Ok(Response::new())
}

/// Update owner
pub fn update_owner(deps: DepsMut, _env: Env, info: MessageInfo, _owner: String) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    // to do
    Ok(Response::new())
}

/// Withdraw xASTRO
pub fn withdraw(_deps: DepsMut, _env: Env, _info: MessageInfo, _amount: Uint128) -> Result<Response, ContractError> {
    // to do
    Ok(Response::new())
}

/// Withdraw bribe rewards
pub fn withdraw_bribe_rewards(_deps: DepsMut, _env: Env, _info: MessageInfo) -> Result<Response, ContractError> {
    // to do
    Ok(Response::new())
}

/// Vote
pub fn vote(_deps: DepsMut, _env: Env, _info: MessageInfo) -> Result<Response, ContractError> {
    // to do
    Ok(Response::new())
}

/// Cw20 Receive hook msg handler.
pub fn receive_cw20(_deps: DepsMut, _env: Env, _info: MessageInfo, msg: Cw20ReceiveMsg) -> Result<Response, ContractError> {
    match from_json(&msg.msg) {
        Ok(Cw20HookMsg::Stake {}) => {
            // to do
            Ok(Response::new())
        }
        Err(_) => Err(ContractError::UnknownMessage {}),
    }
}
