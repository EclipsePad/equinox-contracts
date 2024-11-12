use astroport::{
    asset::{Asset, AssetInfo},
    pair_concentrated_inj::ExecuteMsg as PairExecuteMsg,
    staking::ExecuteMsg as AstroportStakingExecuteMsg,
};
use cosmwasm_std::{
    coin, ensure, ensure_eq, to_json_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response,
    Uint128, WasmMsg,
};
use cw_utils::one_coin;
use eclipse_base::voter::msg::ExecuteMsg as VoterExecuteMsg;
use equinox_msg::lp_depositor::CallbackMsg;

use crate::{entry::query::get_asset_amount_to_convert_eclipastro, state::CONFIG, ContractError};

pub fn try_convert(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let asset = one_coin(&info)?;
    let recipient = recipient.unwrap_or(info.sender.to_string());

    ensure!(
        asset.denom == config.astro
            || asset.denom == config.xastro
            || asset.denom == config.eclipastro,
        ContractError::InvalidCoinAsset(asset.denom)
    );

    let mut msgs = vec![];

    if asset.denom != config.eclipastro {
        let amount_to_eclipastro = get_asset_amount_to_convert_eclipastro(
            deps.as_ref(),
            &Asset {
                info: AssetInfo::NativeToken {
                    denom: asset.denom.clone(),
                },
                amount: asset.amount,
            },
        )?;
        let amount_to_xastro = asset.amount - amount_to_eclipastro;

        if asset.denom == config.astro {
            if !amount_to_xastro.is_zero() {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: config.staking_contract.to_string(),
                    msg: to_json_binary(&AstroportStakingExecuteMsg::Enter { receiver: None })?,
                    funds: vec![coin(amount_to_xastro.u128(), config.astro.clone())],
                }));
            }
            if !amount_to_eclipastro.is_zero() {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: config.voter.to_string(),
                    msg: to_json_binary(&VoterExecuteMsg::SwapToEclipAstro {})?,
                    funds: vec![coin(amount_to_eclipastro.u128(), config.astro)],
                }));
            }
        }
        if asset.denom == config.xastro {
            if !amount_to_eclipastro.is_zero() {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: config.voter.to_string(),
                    msg: to_json_binary(&VoterExecuteMsg::SwapToEclipAstro {})?,
                    funds: vec![coin(amount_to_eclipastro.u128(), config.xastro)],
                }));
            }
        }
    }
    msgs.push(CallbackMsg::DepositIntoPool { recipient }.to_cosmos_msg(&env)?);
    Ok(Response::new().add_messages(msgs))
}

pub fn _try_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CallbackMsg,
) -> Result<Response, ContractError> {
    // Only the contract itself can call callbacks
    ensure_eq!(
        info.sender,
        env.contract.address,
        ContractError::InvalidCallbackInvoke {}
    );
    match msg {
        CallbackMsg::DepositIntoPool { recipient } => try_deposit_into_pool(deps, env, recipient),
    }
}

fn try_deposit_into_pool(
    deps: DepsMut,
    env: Env,
    recipient: String,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let eclipastro_balance = deps
        .querier
        .query_balance(env.contract.address.to_string(), cfg.eclipastro.clone())?;
    let xastro_balance = deps
        .querier
        .query_balance(env.contract.address, cfg.xastro.clone())?;
    ensure!(
        eclipastro_balance.amount.gt(&Uint128::zero())
            || xastro_balance.amount.gt(&Uint128::zero()),
        ContractError::InvalidTokenBalance {}
    );
    let mut funds = vec![];
    if !eclipastro_balance.amount.is_zero() {
        funds.push(coin(
            eclipastro_balance.amount.u128(),
            cfg.eclipastro.clone(),
        ));
    }
    if !xastro_balance.amount.is_zero() {
        funds.push(coin(xastro_balance.amount.u128(), cfg.xastro.clone()));
    }
    let msgs = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.lp_contract.to_string(),
        msg: to_json_binary(&PairExecuteMsg::ProvideLiquidity {
            assets: vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: cfg.eclipastro.clone(),
                    },
                    amount: eclipastro_balance.amount,
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: cfg.xastro.clone(),
                    },
                    amount: xastro_balance.amount,
                },
            ],
            slippage_tolerance: None,
            auto_stake: Some(false),
            receiver: Some(recipient),
        })?,
        funds,
    })];
    Ok(Response::new().add_messages(msgs))
}
