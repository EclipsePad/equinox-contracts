use cosmwasm_std::{
    to_json_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128, WasmMsg,
};
use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};

use crate::{
    error::ContractError,
    state::{CONFIG, LAST_CLAIMED},
};

pub fn try_claim(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    let sender_address = &info.sender;
    let now_in_seconds = env.block.time.seconds();

    let last_claimed = LAST_CLAIMED
        .load(deps.storage, sender_address)
        .unwrap_or_default();

    if now_in_seconds < last_claimed + 3600 {
        Err(cosmwasm_std::StdError::GenericErr {
            msg: "Come back later".to_owned(),
        })?;
    }

    let astro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        cfg.astro.clone(),
        &Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    let mut msg_list = vec![];

    if astro_balance.balance.lt(&cfg.daily_amount) {
        msg_list.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.astro.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Mint {
                recipient: env.contract.address.to_string(),
                amount: Uint128::from(1_000_000_000_000u128),
            })?,
            funds: vec![],
        }))
    }

    LAST_CLAIMED.save(deps.storage, sender_address, &now_in_seconds)?;

    msg_list.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.astro.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: sender_address.to_string(),
            amount: cfg.daily_amount,
        })?,
        funds: vec![],
    }));

    let xastro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        cfg.xastro.clone(),
        &Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    if xastro_balance.balance.lt(&cfg.daily_amount) {
        msg_list.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.astro.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Mint {
                recipient: env.contract.address.to_string(),
                amount: Uint128::from(1_000_000_000_000u128),
            })?,
            funds: vec![],
        }));
        // msg_list.push(CosmosMsg::Wasm(WasmMsg::Execute {
        //     contract_addr: cfg.astro.to_string(),
        //     msg: to_json_binary(&Cw20ExecuteMsg::Send {
        //         contract: cfg.astro_staking.to_string(),
        //         amount: Uint128::from(1_000_000_000_000u128),
        //         msg: to_json_binary(&Cw20HookMsg::Enter {})?,
        //     })?,
        //     funds: vec![],
        // }));
    }

    msg_list.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.xastro.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: sender_address.to_string(),
            amount: cfg.daily_amount,
        })?,
        funds: vec![],
    }));

    Ok(Response::new()
        .add_messages(msg_list)
        .add_attributes([("action", "try_claim")]))
}
