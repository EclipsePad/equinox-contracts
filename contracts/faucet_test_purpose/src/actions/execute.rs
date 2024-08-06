use astro_generator::msg::ExecuteMsg as AstroGeneratorExecuteMsg;
use astroport::staking::ExecuteMsg as StakingExecuteMsg;
use cosmwasm_std::{
    coin, to_json_binary, Addr, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128,
    WasmMsg,
};
use cw_utils::nonpayable;

use crate::{
    error::ContractError,
    msg::UpdateConfig,
    state::{CONFIG, LAST_CLAIMED, OWNER},
};

const MINIMUM_BALANCE: u128 = 1_000_000_000u128;
const ADMIN_AMOUNT_TO_SEND: u128 = 1_000_000_000_000u128;
const MINT_AMOUNT: u128 = 2_000_000_000_000u128;

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

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_config: UpdateConfig,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let mut config = CONFIG.load(deps.storage)?;
    if let Some(astro_token) = new_config.astro_token {
        config.astro_token = astro_token;
    }
    if let Some(xastro_token) = new_config.xastro_token {
        config.xastro_token = xastro_token;
    }
    // if let Some(eclipastro_token) = new_config.eclipastro_token {
    //     config.eclipastro_token = eclipastro_token;
    // }
    // if let Some(lp_token) = new_config.lp_token {
    //     config.lp_token = lp_token;
    // }
    if let Some(astro_generator) = new_config.astro_generator {
        config.astro_generator = astro_generator;
    }
    if let Some(staking_contract) = new_config.staking_contract {
        config.staking_contract = staking_contract;
    }
    // if let Some(lp_contract) = new_config.lp_contract {
    //     config.lp_contract = lp_contract;
    // }
    // if let Some(converter) = new_config.converter {
    //     config.converter = converter;
    // }
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update config"))
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
    let mut astro_amount_to_mint = Uint128::zero();

    let astro_balance = deps
        .querier
        .query_balance(creator, config.astro_token.clone())?;
    let xastro_balance = deps
        .querier
        .query_balance(creator, config.xastro_token.clone())?;
    // let eclipastro_balance: BalanceResponse = deps.querier.query_wasm_smart(
    //     config.eclipastro_token.clone(),
    //     &Cw20QueryMsg::Balance {
    //         address: env.contract.address.to_string(),
    //     },
    // )?;
    // let lp_token_balance: BalanceResponse = deps.querier.query_wasm_smart(
    //     config.lp_token.clone(),
    //     &Cw20QueryMsg::Balance {
    //         address: env.contract.address.to_string(),
    //     },
    // )?;

    if now_in_seconds < last_claimed + 3600 && !OWNER.is_admin(deps.as_ref(), &sender_address)? {
        Err(cosmwasm_std::StdError::GenericErr {
            msg: "Come back later".to_owned(),
        })?;
    }

    let mut amount_to_send = Uint128::from(MINIMUM_BALANCE);
    if OWNER.is_admin(deps.as_ref(), &sender_address)? {
        amount_to_send = Uint128::from(ADMIN_AMOUNT_TO_SEND);
    }

    if astro_balance.amount.lt(&amount_to_send) {
        astro_amount_to_mint += Uint128::from(MINT_AMOUNT);
    }
    if xastro_balance.amount.lt(&amount_to_send) {
        astro_amount_to_mint += Uint128::from(MINT_AMOUNT);
        let astro_amount_to_convert = coin(MINT_AMOUNT, config.astro_token.clone());
        msg_list.push(
            WasmMsg::Execute {
                contract_addr: config.staking_contract.to_string(),
                msg: to_json_binary(&StakingExecuteMsg::Enter { receiver: None })?,
                funds: vec![astro_amount_to_convert.clone()],
            }
            .into(),
        );
    }
    // if eclipastro_balance
    //     .balance
    //     .lt(&Uint128::from(MINIMUM_BALANCE))
    // {
    //     astro_amount_to_mint += Uint128::from(MINT_AMOUNT);
    //     msg_list.push(
    //         WasmMsg::Execute {
    //             contract_addr: config.converter.to_string(),
    //             msg: to_json_binary(&ConverterExecuteMsg::Convert { recipient: None })?,
    //             funds: vec![astro_amount_to_convert],
    //         }
    //         .into(),
    //     );
    // }
    // if lp_token_balance.balance.lt(&Uint128::from(MINIMUM_BALANCE)) {
    //     astro_amount_to_mint += Uint128::from(MINT_AMOUNT);
    //     let pool_info: PoolResponse = deps
    //         .querier
    //         .query_wasm_smart(config.lp_contract.to_string(), &PairQueryMsg::Pool {})?;
    //     let astro_staking_total_deposit: Uint128 = deps.querier.query_wasm_smart(
    //         config.staking_contract.to_string(),
    //         &StakingQueryMsg::TotalDeposit {},
    //     )?;
    //     let astro_staking_total_shares: Uint128 = deps.querier.query_wasm_smart(
    //         config.staking_contract.to_string(),
    //         &StakingQueryMsg::TotalShares {},
    //     )?;
    //     let xastro_asset_in_pool = pool_info
    //         .assets
    //         .iter()
    //         .find(|&asset| {
    //             asset.info.equal(&AssetInfo::NativeToken {
    //                 denom: config.xastro_token.clone(),
    //             })
    //         })
    //         .unwrap();
    //     let eclipastro_asset_in_pool = pool_info
    //         .assets
    //         .iter()
    //         .find(|&asset| {
    //             asset.info.equal(&AssetInfo::Token {
    //                 contract_addr: config.eclipastro_token.clone(),
    //             })
    //         })
    //         .unwrap();
    //     let numerator = Uint256::from_uint128(xastro_asset_in_pool.amount)
    //         .checked_mul(astro_staking_total_deposit.into())
    //         .unwrap();
    //     let denominator = numerator
    //         + Uint256::from_uint128(eclipastro_asset_in_pool.amount)
    //             .checked_mul(astro_staking_total_shares.into())
    //             .unwrap();
    //     let astro_amount_to_convert: Uint128 = Uint256::from(MINT_AMOUNT)
    //         .multiply_ratio(numerator, denominator)
    //         .try_into()
    //         .unwrap_or_default();
    //     let eclipastro_amount_to_deposit = Uint128::from(MINT_AMOUNT) - astro_amount_to_convert;
    //     let xastro_amount_to_deposit = astro_amount_to_convert
    //         .clone()
    //         .multiply_ratio(astro_staking_total_shares, astro_staking_total_deposit);
    //     msg_list.push(
    //         WasmMsg::Execute {
    //             contract_addr: config.staking_contract.to_string(),
    //             msg: to_json_binary(&StakingExecuteMsg::Enter { receiver: None })?,
    //             funds: vec![coin(
    //                 astro_amount_to_convert.u128(),
    //                 config.astro_token.clone(),
    //             )],
    //         }
    //         .into(),
    //     );
    //     msg_list.push(
    //         WasmMsg::Execute {
    //             contract_addr: config.converter.to_string(),
    //             msg: to_json_binary(&ConverterExecuteMsg::Convert { recipient: None })?,
    //             funds: vec![coin(
    //                 eclipastro_amount_to_deposit.u128(),
    //                 config.astro_token.clone(),
    //             )],
    //         }
    //         .into(),
    //     );
    //     msg_list.push(CosmosMsg::Wasm(WasmMsg::Execute {
    //         contract_addr: config.eclipastro_token.to_string(),
    //         msg: to_json_binary(&Cw20ExecuteMsg::IncreaseAllowance {
    //             spender: config.lp_contract.to_string(),
    //             amount: eclipastro_amount_to_deposit,
    //             expires: None,
    //         })?,
    //         funds: vec![],
    //     }));
    //     msg_list.push(CosmosMsg::Wasm(WasmMsg::Execute {
    //         contract_addr: config.lp_contract.to_string(),
    //         msg: to_json_binary(&PairExecuteMsg::ProvideLiquidity {
    //             assets: vec![
    //                 Asset {
    //                     info: AssetInfo::Token {
    //                         contract_addr: config.eclipastro_token.clone(),
    //                     },
    //                     amount: eclipastro_amount_to_deposit,
    //                 },
    //                 Asset {
    //                     info: AssetInfo::NativeToken {
    //                         denom: config.xastro_token.clone(),
    //                     },
    //                     amount: xastro_amount_to_deposit,
    //                 },
    //             ],
    //             slippage_tolerance: None,
    //             auto_stake: Some(false),
    //             receiver: None,
    //         })?,
    //         funds: vec![coin(
    //             xastro_amount_to_deposit.u128(),
    //             config.xastro_token.clone(),
    //         )],
    //     }));
    // }

    msg_list.push(CosmosMsg::Bank(BankMsg::Send {
        to_address: sender_address.to_string(),
        amount: vec![coin(amount_to_send.u128(), config.astro_token.clone())],
    }));
    msg_list.push(CosmosMsg::Bank(BankMsg::Send {
        to_address: sender_address.to_string(),
        amount: vec![coin(amount_to_send.u128(), config.xastro_token.clone())],
    }));
    // msg_list.push(
    //     WasmMsg::Execute {
    //         contract_addr: config.eclipastro_token.to_string(),
    //         msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
    //             recipient: sender_address.to_string(),
    //             amount: Uint128::from(MINIMUM_BALANCE),
    //         })?,
    //         funds: vec![],
    //     }
    //     .into(),
    // );
    // msg_list.push(
    //     WasmMsg::Execute {
    //         contract_addr: config.lp_token.to_string(),
    //         msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
    //             recipient: sender_address.to_string(),
    //             amount: Uint128::from(MINIMUM_BALANCE),
    //         })?,
    //         funds: vec![],
    //     }
    //     .into(),
    // );

    LAST_CLAIMED.save(deps.storage, &sender_address, &now_in_seconds)?;
    let mut response = Response::new();

    if !astro_amount_to_mint.is_zero() {
        response = response.add_message(WasmMsg::Execute {
            contract_addr: config.astro_generator.to_string(),
            msg: to_json_binary(&AstroGeneratorExecuteMsg::Mint {
                amount: astro_amount_to_mint,
            })?,
            funds: vec![],
        });
    }

    Ok(response
        .add_messages(msg_list)
        .add_attributes([("action", "try_claim")]))
}
