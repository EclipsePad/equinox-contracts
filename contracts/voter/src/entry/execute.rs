use cosmwasm_std::{coin, BankMsg, CosmosMsg};
use cosmwasm_std::{ensure_eq, DepsMut, Env, MessageInfo, Response, Uint128};
use cw_utils::must_pay;

use equinox_msg::voter::UpdateConfig;
use equinox_msg::voter::Vote;

use crate::{
    error::ContractError,
    state::{CONFIG, OWNER},
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
    if let Some(astro) = new_config.astro {
        config.astro = astro.clone();
        res = res.add_attribute("astro", astro);
    }
    if let Some(xastro) = new_config.xastro {
        config.xastro = xastro.clone();
        res = res.add_attribute("xastro", xastro);
    }
    if let Some(vxastro) = new_config.vxastro {
        config.vxastro = vxastro.clone();
        res = res.add_attribute("vxastro", vxastro);
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
    let xastro_amount_to_send = coin(amount.u128(), config.xastro.clone());
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
        CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient.to_string(),
            amount: vec![xastro_amount_to_send],
        }),
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

/// Stake ASTRO/xASTRO
pub fn try_stake(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let amount = must_pay(&info, &config.xastro)?;
    // Check sender is converter
    ensure_eq!(
        info.sender,
        config.converter_contract,
        ContractError::Unauthorized {}
    );
    Ok(Response::new()
        .add_attribute("action", "lock xASTRO")
        .add_attribute("xASTRO", amount.to_string()))
}
