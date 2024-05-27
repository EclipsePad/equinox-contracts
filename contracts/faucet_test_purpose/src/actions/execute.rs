use astro_generator::msg::ExecuteMsg as AstroGeneratorExecuteMsg;
use astroport::staking::ExecuteMsg;
use cosmwasm_std::{
    coin, to_json_binary, Addr, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128,
    WasmMsg,
};
use cw_utils::nonpayable;

use crate::{
    error::ContractError,
    state::{CONFIG, LAST_CLAIMED, OWNER},
};

pub fn update_owner(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_owner: Addr,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    OWNER.set(deps.branch(), Some(new_owner.clone()))?;
    Ok(Response::new()
        .add_attribute("action", "update owner")
        .add_attribute("to", new_owner.to_string()))
}

pub fn try_claim(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let config = CONFIG.load(deps.storage)?;

    let sender_address = info.sender;
    let now_in_seconds = env.block.time.seconds();
    let creator = &env.contract.address;
    let last_claimed = LAST_CLAIMED
        .load(deps.storage, &sender_address)
        .unwrap_or_default();

    let mut msg_list: Vec<CosmosMsg> = vec![];

    let astro_balance = deps
        .querier
        .query_balance(creator, config.astro_token.clone())?;
    let xastro_balance = deps
        .querier
        .query_balance(creator, config.xastro_token.clone())?;

    if astro_balance.amount < Uint128::from(20_000_000_000u128) {
        msg_list.push(
            WasmMsg::Execute {
                contract_addr: config.astro_generator.to_string(),
                msg: to_json_binary(&AstroGeneratorExecuteMsg::Mint {
                    amount: Uint128::from(1_000_000_000_000u128),
                })?,
                funds: vec![],
            }
            .into(),
        )
    }

    let astro_amount_to_convert = coin(10_000_000_000, config.astro_token.clone());

    if xastro_balance.amount < Uint128::from(1_000_000_000u128) {
        msg_list.push(
            WasmMsg::Execute {
                contract_addr: config.staking_contract.to_string(),
                msg: to_json_binary(&ExecuteMsg::Enter { receiver: None })?,
                funds: vec![astro_amount_to_convert],
            }
            .into(),
        )
    }

    let astro_amount_to_mint = coin(1_000_000_000, config.astro_token.clone());
    let xastro_amount_to_mint = coin(1_000_000_000, config.xastro_token.clone());
    if now_in_seconds < last_claimed + 3600 {
        Err(cosmwasm_std::StdError::GenericErr {
            msg: "Come back later".to_owned(),
        })?;
    }
    msg_list.push(CosmosMsg::Bank(BankMsg::Send {
        to_address: sender_address.to_string(),
        amount: vec![astro_amount_to_mint],
    }));
    msg_list.push(CosmosMsg::Bank(BankMsg::Send {
        to_address: sender_address.to_string(),
        amount: vec![xastro_amount_to_mint],
    }));

    LAST_CLAIMED.save(deps.storage, &sender_address, &now_in_seconds)?;

    Ok(Response::new()
        .add_messages(msg_list)
        .add_attributes([("action", "try_claim")]))
}
