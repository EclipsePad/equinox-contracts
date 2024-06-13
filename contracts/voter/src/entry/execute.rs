use std::str::FromStr;

use cosmwasm_std::{
    ensure, ensure_eq, to_json_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, ReplyOn,
    Response, SubMsg, SubMsgResult, Uint128, WasmMsg,
};

use astroport::staking::Cw20HookMsg as AstroportStakingCw20HookMsg;

use cw20::Cw20ExecuteMsg;

use equinox_msg::voter::{Config, UpdateConfig, Vote, MAX_ESCROW_VOTING_LOCK_PERIOD};

use crate::{
    contract::{STAKE_ASTRO_REPLY_ID, STAKE_TOKEN_REPLY_ID},
    error::ContractError,
    state::{CONFIG, OWNER, RECIPIENT},
};

/// Update config
pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_config: UpdateConfig,
) -> Result<Response, ContractError> {
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
    if let Some(astroport_voting_escrow_contract) = new_config.astroport_voting_escrow_contract {
        config.astroport_voting_escrow_contract =
            deps.api.addr_validate(&astroport_voting_escrow_contract)?;
        res = res.add_attribute(
            "astroport_voting_escrow_contract",
            astroport_voting_escrow_contract,
        );
    }
    CONFIG.save(deps.storage, &config)?;
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
    // let vxtoken_balance: BalanceResponse = deps.querier.query_wasm_smart(
    //     config.vxtoken.to_string(),
    //     &AstroportVotingEscrowQueryMsg::Balance {
    //         address: env.contract.address.to_string(),
    //     },
    // )?;
    let msgs = vec![
        // Not implemented
        // WasmMsg::Execute {
        //     contract_addr: config.vxtoken.to_string(),
        //     msg: to_json_binary(&AstroportVotingEscrowExecuteMsg::Withdraw {})?,
        //     funds: vec![],
        // },
        // WasmMsg::Execute {
        //     contract_addr: config.xtoken.to_string(),
        //     msg: to_json_binary(&Cw20ExecuteMsg::Send {
        //         contract: config.vxtoken.to_string(),
        //         amount: remaining,
        //         msg: to_json_binary(&AstroportVotingEscrowCw20HookMsg::CreateLock { time: 0u64 })?,
        //     })?,
        //     funds: vec![],
        // },
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

pub fn try_stake(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    sender: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // only ASTRO token or xASTRO token can execute this message
    ensure!(
        info.sender == config.base_token || info.sender == config.xtoken,
        ContractError::UnknownToken(info.sender.to_string())
    );

    // Check sender is converter
    ensure_eq!(
        sender,
        config.converter_contract,
        ContractError::Unauthorized {}
    );

    if info.sender == config.xtoken {
        return Ok(Response::new()
            .add_attribute("action", "lock xASTRO")
            .add_attribute("xASTRO", amount.to_string()));
    }

    let stake_msg = SubMsg {
        id: STAKE_TOKEN_REPLY_ID,
        msg: WasmMsg::Execute {
            contract_addr: config.base_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: config.staking_contract.to_string(),
                amount,
                msg: to_json_binary(&AstroportStakingCw20HookMsg::Enter {})?,
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
        .add_attribute("ASTRO", amount.to_string()))
}

pub fn handle_stake_reply(
    _deps: DepsMut,
    _env: Env,
    result: &SubMsgResult,
) -> Result<Response, ContractError> {
    let res = result
        .to_owned()
        .into_result()
        .map_err(|_| ContractError::StakeError {})?;

    let mut xtoken_amount = Uint128::zero();
    for event in res.events.iter() {
        for attr in event.attributes.iter() {
            if attr.key == "xastro_amount" {
                xtoken_amount = Uint128::from_str(&attr.value).unwrap();
            }
        }
    }
    // let config = CONFIG.load(deps.storage)?;
    // lock
    // Not implemented
    // let lock_msg = WasmMsg::Execute {
    //     contract_addr: config.vxtoken.to_string(),
    //     msg: to_json_binary(&Cw20ExecuteMsg::Send {
    //         contract: config.xtoken.to_string(),
    //         amount: xtoken_amount,
    //         msg: to_json_binary(&AstroportVotingEscrowCw20HookMsg::CreateLock { time: 0u64 })?,
    //     })?,
    //     funds: vec![],
    // };
    Ok(Response::new()
        // .add_message(lock_msg)
        .add_attribute("action", "lock xASTRO")
        .add_attribute("xASTRO", xtoken_amount.to_string()))
}

pub fn try_swap_to_eclip_astro(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let token_in = &info.sender;
    let user_address = &deps.api.addr_validate(&sender)?;
    let Config {
        base_token: astro,
        xtoken: xastro,
        staking_contract,
        ..
    } = CONFIG.load(deps.storage)?;

    // check if ASTRO or xASTRO was sent
    if token_in != astro && token_in != xastro {
        Err(ContractError::UnknownToken(token_in.to_string()))?;
    }

    // check if amount isn't zero
    if amount.is_zero() {
        Err(ContractError::ZeroAmount {})?;
    }

    // get xastro first
    if token_in == astro {
        RECIPIENT.save(deps.storage, user_address)?;

        let msg = SubMsg {
            id: STAKE_ASTRO_REPLY_ID,
            msg: WasmMsg::Execute {
                contract_addr: astro.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Send {
                    contract: staking_contract.to_string(),
                    amount,
                    msg: to_json_binary(&AstroportStakingCw20HookMsg::Enter {})?,
                })?,
                funds: vec![],
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Success,
        };

        return Ok(Response::new().add_submessage(msg));
    }

    lock_xastro(deps, env, amount, user_address)
}

pub fn handle_stake_astro_reply(
    deps: DepsMut,
    env: Env,
    result: &SubMsgResult,
) -> Result<Response, ContractError> {
    let res = result
        .to_owned()
        .into_result()
        .map_err(|_| ContractError::StakeError {})?;

    let mut xastro_amount = Uint128::zero();
    for event in res.events.iter() {
        for attr in event.attributes.iter() {
            if attr.key == "xastro_amount" {
                xastro_amount = Uint128::from_str(&attr.value).unwrap();
            }
        }
    }

    let recipient = &RECIPIENT.load(deps.storage)?;
    lock_xastro(deps, env, xastro_amount, recipient)
}

fn lock_xastro(
    deps: DepsMut,
    env: Env,
    xastro_amount: Uint128,
    recipient: &Addr,
) -> Result<Response, ContractError> {
    let Config {
        xtoken: xastro,
        astroport_voting_escrow_contract,
        converter_contract,
        staking_contract,
        ..
    } = CONFIG.load(deps.storage)?;

    // calculate eclipASTRO amount
    let total_xastro_amount: Uint128 = deps.querier.query_wasm_smart(
        staking_contract.to_string(),
        &astroport::staking::QueryMsg::TotalShares {},
    )?;
    let total_astro_amount: Uint128 = deps.querier.query_wasm_smart(
        staking_contract.to_string(),
        &astroport::staking::QueryMsg::TotalDeposit {},
    )?;
    let eclip_astro_amount = total_astro_amount * xastro_amount / total_xastro_amount;

    // check lock position
    let lock_info = deps
        .querier
        .query_wasm_smart::<astroport_governance::voting_escrow::LockInfoResponse>(
            astroport_voting_escrow_contract.to_string(),
            &astroport_governance::voting_escrow::QueryMsg::LockInfo {
                user: env.contract.address.to_string(),
            },
        );

    let hook_msg = match lock_info {
        Ok(_) => {
            to_json_binary(&astroport_governance::voting_escrow::Cw20HookMsg::ExtendLockAmount {})
        }
        Err(_) => to_json_binary(
            &astroport_governance::voting_escrow::Cw20HookMsg::CreateLock {
                time: MAX_ESCROW_VOTING_LOCK_PERIOD,
            },
        ),
    };

    let msg_list = vec![
        // replenish existent lock or create new one for 2 years
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: xastro.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: astroport_voting_escrow_contract.to_string(),
                amount: xastro_amount,
                msg: hook_msg?,
            })?,
            funds: vec![],
        }),
        // mint eclipAstro to user
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: converter_contract.to_string(),
            msg: to_json_binary(&equinox_msg::token_converter::ExecuteMsg::MintEclipAstro {
                amount: eclip_astro_amount,
                recipient: recipient.to_string(),
            })?,
            funds: vec![],
        }),
    ];

    Ok(Response::new()
        .add_messages(msg_list)
        .add_attribute("action", "try_swap_to_eclip_astro")
        .add_attribute("eclip_astro_amount", eclip_astro_amount))
}
