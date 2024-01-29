use cosmwasm_std::{from_json, Addr, DepsMut, Env, MessageInfo, Response, Uint128,};
use cw20::Cw20ReceiveMsg;

use crate::{
    error::ContractError, msg::Cw20HookMsg, state::{Config, RewardConfig, CONFIG, OWNER}
};

/// Update config
pub fn update_config(deps: DepsMut, _env: Env, info: MessageInfo, _config: Config) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    // to do
    Ok(Response::new())
}

/// Update reward config
pub fn update_reward_config(deps: DepsMut, _env: Env, info: MessageInfo, _config: RewardConfig) -> Result<Response, ContractError> {
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

/// claim user rewards
pub fn claim(_deps: DepsMut, _env: Env, _info: MessageInfo) -> Result<Response, ContractError> {
    // to do
    Ok(Response::new())
}

/// claim treasury rewards
pub fn claim_treasury_reward(_deps: DepsMut, _env: Env, _info: MessageInfo, _amount: Uint128) -> Result<Response, ContractError> {
    // to do
    Ok(Response::new())
}

/// Cw20 Receive hook msg handler.
pub fn receive_cw20(deps: DepsMut, env: Env, info: MessageInfo, msg: Cw20ReceiveMsg) -> Result<Response, ContractError> {
    match from_json(&msg.msg) {
        Ok(Cw20HookMsg::Stake {}) => {
            let config: Config = CONFIG.load(deps.storage)?;
            // only ASTRO token contract can execute this message
            if config.base_token != info.sender {
                return Err(ContractError::UnknownToken(info.sender.to_string()));
            }

            let cw20_sender = deps.api.addr_validate(&msg.sender)?;
            stake(deps, env, cw20_sender, msg.amount)
        }
        Err(_) => Err(ContractError::UnknownMessage {}),
    }
}

/// ASTRO staking handler
fn stake(_deps: DepsMut, _env: Env, _sender: Addr, _amount: Uint128) -> Result<Response, ContractError> {
    // to do
    // let config: Config = CONFIG.load(deps.storage)?;
    // let sub_msg: Vec<SubMsg> = vec![SubMsg {
    //     id: STAKE_TOKEN_REPLY_ID,
    //     msg: WasmMsg::Execute {
    //         //sending reward to user
    //         contract_addr: config.base_token.to_string(),
    //         msg: to_json_binary(
    //             &Cw20ExecuteMsg::Send { contract: config.voter, amount, msg: to_json_binary(
                    
    //             )}
    //         )?,
    //         funds: vec![],
    //     }
    //     .into(),
    //     gas_limit: None,
    //     reply_on: ReplyOn::Success,
    // }];
    Ok(Response::new())
}
