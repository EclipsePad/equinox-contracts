use std::str::FromStr;

use astroport_msg::staking as AstroMsg;
use astroport_msg::voting_escrow as AstroVotingEscrowMsg;
use cosmwasm_std::{
    ensure_eq, from_json, to_json_binary, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response,
    SubMsg, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::{
    contract::STAKE_TOKEN_REPLY_ID,
    error::ContractError,
    msg::{Cw20HookMsg, UpdateConfig},
    state::{Vote, CONFIG, OWNER},
};

/// Update config
pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_config: UpdateConfig,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let mut config = CONFIG.load(deps.storage)?;
    let mut res: Response = Response::new().add_attribute("action", "update config");
    if let Some(base_token) = new_config.base_token {
        config.base_token = deps.api.addr_validate(&base_token)?;
        res = res.add_attribute("base_token", base_token);
    }
    if let Some(xtoken) = new_config.xtoken {
        config.xtoken = deps.api.addr_validate(&xtoken)?;
        res = res.add_attribute("xtoken", xtoken);
    }
    if let Some(vxtoken) = new_config.vxtoken {
        config.vxtoken = deps.api.addr_validate(&vxtoken)?;
        res = res.add_attribute("vxtoken", vxtoken);
    }
    if let Some(staking_contract) = new_config.staking_contract {
        config.staking_contract = deps.api.addr_validate(&staking_contract)?;
        res = res.add_attribute("staking_contract", staking_contract);
    }
    if let Some(converter_contract) = new_config.converter_contract {
        config.converter_contract = deps.api.addr_validate(&converter_contract)?;
        res = res.add_attribute("converter_contract", converter_contract);
    }
    if let Some(gauge_contract) = new_config.gauge_contract {
        config.gauge_contract = deps.api.addr_validate(&gauge_contract)?;
        res = res.add_attribute("gauge_contract", gauge_contract);
    }
    if let Some(astroport_gauge_contract) = new_config.astroport_gauge_contract {
        config.astroport_gauge_contract = deps.api.addr_validate(&astroport_gauge_contract)?;
        res = res.add_attribute("astroport_gauge_contract", astroport_gauge_contract);
    }
    Ok(res)
}

/// Update owner
pub fn update_owner(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let new_owner_addr = deps.api.addr_validate(&new_owner)?;
    OWNER.set(deps.branch(), Some(new_owner_addr))?;
    Ok(Response::new()
        .add_attribute("action", "update owner")
        .add_attribute("to", new_owner))
}

/// Withdraw xASTRO
pub fn withdraw(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
    recipient: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_eq!(
        info.sender,
        config.converter_contract,
        ContractError::Unauthorized {}
    );
    let msgs = vec![
        WasmMsg::Execute {
            contract_addr: config.vxtoken.to_string(),
            msg: to_json_binary(&AstroVotingEscrowMsg::ExecuteMsg::Withdraw { amount })?,
            funds: vec![],
        },
        WasmMsg::Execute {
            contract_addr: config.xtoken.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: recipient.clone(),
                amount,
            })?,
            funds: vec![],
        },
    ];
    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "withdraw xASTRO")
        .add_attribute("amount", amount.to_string())
        .add_attribute("to", recipient))
}

/// Withdraw bribe rewards
pub fn withdraw_bribe_rewards(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    // to do
    Ok(Response::new())
}

/// Vote
pub fn place_vote(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _gauge: u64,
    _votes: Option<Vec<Vote>>,
) -> Result<Response, ContractError> {
    // to do
    Ok(Response::new())
}

/// Cw20 Receive hook msg handler.
pub fn receive_cw20(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_json(&msg.msg) {
        Ok(Cw20HookMsg::Stake {}) => {
            let config = CONFIG.load(deps.storage)?;
            // Check sender is ASTRO token
            ensure_eq!(
                info.sender,
                config.base_token,
                ContractError::UnknownToken(info.sender.to_string())
            );
            let stake_msg = SubMsg {
                id: STAKE_TOKEN_REPLY_ID,
                msg: WasmMsg::Execute {
                    contract_addr: config.base_token.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Send {
                        contract: config.xtoken.to_string(),
                        amount: msg.amount,
                        msg: to_json_binary(&AstroMsg::Cw20HookMsg::Enter {})?,
                    })?,
                    funds: vec![],
                }
                .into(),
                gas_limit: None,
                reply_on: ReplyOn::Success,
            };
            Ok(Response::new()
                .add_submessage(stake_msg)
                .add_attribute("action", "stake ASTRO")
                .add_attribute("ASTRO", msg.amount.to_string()))
        }
        Err(_) => Err(ContractError::UnknownMessage {}),
    }
}

pub fn handle_stake_reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::StakeError {});
    }
    let mut xtoken_amount = Uint128::zero();
    for event in msg.result.unwrap().events.iter() {
        for attr in event.attributes.iter() {
            if attr.key == "xastro_amount" {
                xtoken_amount = Uint128::from_str(&attr.value).unwrap();
            }
        }
    }
    let config = CONFIG.load(deps.storage)?;
    // lock
    let lock_msg = WasmMsg::Execute {
        contract_addr: config.vxtoken.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Send {
            contract: config.xtoken.to_string(),
            amount: xtoken_amount,
            msg: to_json_binary(&AstroVotingEscrowMsg::Cw20HookMsg::CreateLock {})?,
        })?,
        funds: vec![],
    };
    Ok(Response::new()
        .add_message(lock_msg)
        .add_attribute("action", "lock xASTRO")
        .add_attribute("xASTRO", xtoken_amount.to_string()))
}
