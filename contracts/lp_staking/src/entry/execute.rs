use astroport::{
    asset::Asset,
    incentives::{Cw20Msg as IncentivesCw20Msg, ExecuteMsg as IncentivesExecuteMsg},
    staking::ExecuteMsg as StakingExecuteMsg,
};
use cosmwasm_std::{
    coin, ensure, ensure_eq, from_json, to_json_binary, BankMsg, CosmosMsg, DepsMut, Env,
    MessageInfo, Response, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use equinox_msg::{
    lp_staking::{CallbackMsg, Cw20HookMsg, RewardConfig, UpdateConfigMsg},
    // token_converter::ExecuteMsg as ConverterExecuteMsg,
};

use crate::{
    entry::query::{
        calculate_beclip_reward, calculate_incentive_pending_rewards,
        calculate_pending_eclipse_rewards, calculate_updated_reward_weights,
        calculate_user_staking_rewards,
    },
    error::ContractError,
    state::{CONFIG, LAST_CLAIMED, OWNER, REWARD_CONFIG, REWARD_WEIGHTS, STAKING, TOTAL_STAKING},
};

/// Update config
/// Only owner
pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_config: UpdateConfigMsg,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let mut config = CONFIG.load(deps.storage)?;
    let mut res: Response = Response::new().add_attribute("action", "update config");
    if let Some(lp_token) = new_config.lp_token {
        config.lp_token = lp_token.clone();
        res = res.add_attribute("lp_token", lp_token.to_string());
    }
    if let Some(lp_contract) = new_config.lp_contract {
        config.lp_contract = lp_contract.clone();
        res = res.add_attribute("lp_contract", lp_contract.to_string());
    }
    if let Some(beclip) = new_config.beclip {
        config.beclip = beclip.clone();
        res = res.add_attribute("beclip", beclip.to_string());
    }
    if let Some(beclip_daily_reward) = new_config.beclip_daily_reward {
        config.beclip_daily_reward = beclip_daily_reward;
        res = res.add_attribute("beclip_daily_reward", beclip_daily_reward.to_string());
    }
    if let Some(converter) = new_config.converter {
        config.converter = converter.clone();
        res = res.add_attribute("converter", converter);
    }
    if let Some(astroport_generator) = new_config.astroport_generator {
        config.astroport_generator = astroport_generator.clone();
        res = res.add_attribute("astroport_generator", astroport_generator.to_string());
    }
    if let Some(treasury) = new_config.treasury {
        config.treasury = treasury.clone();
        res = res.add_attribute("treasury", treasury);
    }
    if let Some(stability_pool) = new_config.stability_pool {
        config.stability_pool = Some(stability_pool.clone());
        res = res.add_attribute("stability_pool", stability_pool.to_string());
    }
    if let Some(ce_reward_distributor) = new_config.ce_reward_distributor {
        config.ce_reward_distributor = Some(ce_reward_distributor.clone());
        res = res.add_attribute("ce_reward_distributor", ce_reward_distributor.to_string());
    }
    CONFIG.save(deps.storage, &config)?;
    Ok(res)
}

/// Update owner
/// Only owner
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

/// Update reward config
pub fn update_reward_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    config: RewardConfig,
) -> Result<Response, ContractError> {
    // only owner can executable
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    // the sum bps should be 10000
    ensure_eq!(
        config.users + config.treasury + config.ce_holders + config.stability_pool,
        10000,
        ContractError::RewardDistributionErr {}
    );
    REWARD_CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update reward config"))
}

pub fn _handle_callback(
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
        CallbackMsg::DistributeEclipseRewards { assets } => {
            distribute_eclipse_rewards(deps, env, info, assets)
        }
    }
}

/// Cw20 Receive hook msg handler.
pub fn receive_cw20(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_json(&msg.msg)? {
        // stake lp token
        // non zero amount
        // update rewards
        // update user staking, total staking amount
        Cw20HookMsg::Stake {} => {
            let cfg = CONFIG.load(deps.storage)?;
            let mut total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
            let mut user_staking = STAKING
                .load(deps.storage, &msg.sender.to_string())
                .unwrap_or_default();

            ensure_eq!(
                cfg.lp_token,
                info.sender,
                ContractError::Cw20AddressesNotMatch {
                    got: info.sender.to_string(),
                    expected: cfg.lp_token.to_string(),
                }
            );
            ensure!(
                msg.amount.gt(&Uint128::zero()),
                ContractError::ZeroAmount {}
            );

            let mut msgs = vec![];
            let mut response = Response::new();

            // stake LP token to Astroport generator contract
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.lp_token.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Send {
                    contract: cfg.astroport_generator.to_string(),
                    amount: msg.amount,
                    msg: to_json_binary(&IncentivesCw20Msg::Deposit { recipient: None })?,
                })?,
                funds: vec![],
            }));

            if total_staking.gt(&Uint128::zero()) {
                response = _claim(deps.branch(), env, msg.sender.clone())?;
            } else {
                LAST_CLAIMED.save(deps.storage, &env.block.time.seconds())?;
            }

            total_staking += msg.amount;
            user_staking.staked = user_staking.staked.checked_add(msg.amount).unwrap();

            TOTAL_STAKING.save(deps.storage, &total_staking)?;
            STAKING.save(deps.storage, &msg.sender, &user_staking)?;

            Ok(response
                .add_attribute("action", "stake")
                .add_attribute("sender", msg.sender.clone().to_string())
                .add_attribute("amount", msg.amount.to_string())
                .add_messages(msgs))
        }
    }
}

pub fn _claim(deps: DepsMut, env: Env, sender: String) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    let mut user_staking = STAKING.load(deps.storage, &sender).unwrap_or_default();
    let mut msgs = vec![];

    ensure!(
        !total_staking.is_zero(),
        ContractError::InvalidStakingAmount {}
    );

    // claim astro reward
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.astroport_generator.to_string(),
        msg: to_json_binary(&IncentivesExecuteMsg::ClaimRewards {
            lp_tokens: vec![cfg.lp_token.to_string()],
        })?,
        funds: vec![],
    }));

    let mut response = Response::new()
        .add_attribute("action", "claim rewards")
        .add_attribute("recipient", sender.clone());

    let astroport_rewards =
        calculate_incentive_pending_rewards(deps.as_ref(), env.contract.address.clone())?;
    let beclip_reward = calculate_beclip_reward(deps.as_ref(), env.block.time.seconds())?;
    let pending_eclipse_rewards =
        calculate_pending_eclipse_rewards(deps.as_ref(), astroport_rewards.clone())?;
    let updated_reward_weights =
        calculate_updated_reward_weights(deps.as_ref(), astroport_rewards, beclip_reward)?;
    if !user_staking.staked.is_zero() {
        let user_rewards = calculate_user_staking_rewards(
            deps.as_ref(),
            sender.clone(),
            updated_reward_weights.clone(),
        )?;
        let mut coins = vec![];
        for r in user_rewards {
            if !r.amount.is_zero() {
                if r.info.is_native_token() {
                    // if r.info.to_string() == cfg.astro {
                    //     msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    //         contract_addr: cfg.converter.clone().into_string(),
                    //         msg: to_json_binary(&ConverterExecuteMsg::Convert {
                    //             recipient: Some(sender.clone()),
                    //         })?,
                    //         funds: vec![coin(r.amount.u128(), r.info.to_string())],
                    //     }));
                    // } else {
                    coins.push(coin(r.amount.u128(), r.info.to_string()));
                    response = response
                        .add_attribute("action", "claim")
                        .add_attribute("denom", r.info.to_string())
                        .add_attribute("amount", r.amount);
                    // }
                } else {
                    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: r.info.to_string(),
                        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                            recipient: sender.clone(),
                            amount: r.amount,
                        })?,
                        funds: vec![],
                    }));
                    response = response
                        .add_attribute("action", "claim")
                        .add_attribute("address", r.info.to_string())
                        .add_attribute("amount", r.amount);
                }
            }
        }
        if !coins.is_empty() {
            msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: sender.clone(),
                amount: coins,
            }));
        }
    }
    user_staking.reward_weights = updated_reward_weights.clone();

    if !pending_eclipse_rewards.is_empty() {
        msgs.push(
            CallbackMsg::DistributeEclipseRewards {
                assets: pending_eclipse_rewards.clone(),
            }
            .to_cosmos_msg(&env)?,
        );
    }

    REWARD_WEIGHTS.save(deps.storage, &updated_reward_weights)?;
    STAKING.save(deps.storage, &sender, &user_staking)?;
    LAST_CLAIMED.save(deps.storage, &env.block.time.seconds())?;

    Ok(response.add_messages(msgs))
}

/// Claim user rewards
pub fn claim(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    sender: String,
) -> Result<Response, ContractError> {
    _claim(deps, env, sender)
}

/// Unstake amount and claim rewards of user
/// check unstake amount
pub fn unstake(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    let mut user_staking = STAKING.load(deps.storage, &info.sender.to_string())?;
    let mut total_staking = TOTAL_STAKING.load(deps.storage)?;

    let receiver = recipient.unwrap_or(info.sender.to_string());
    let mut msgs = vec![];
    let mut response = Response::new();

    ensure!(
        amount.le(&user_staking.staked),
        ContractError::ExeedingUnstakeAmount {
            got: amount.u128(),
            expected: user_staking.staked.u128()
        }
    );

    if total_staking.gt(&Uint128::zero()) {
        response = _claim(deps.branch(), env, receiver.clone())?;
    }

    total_staking = total_staking.checked_sub(amount).unwrap();
    user_staking.staked = user_staking.staked.checked_sub(amount).unwrap();

    TOTAL_STAKING.save(deps.storage, &total_staking)?;
    STAKING.save(deps.storage, &info.sender.to_string(), &user_staking)?;

    // send lp_token to user
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.astroport_generator.to_string(),
        msg: to_json_binary(&IncentivesExecuteMsg::Withdraw {
            lp_token: cfg.lp_token.clone().to_string(),
            amount,
        })?,
        funds: vec![],
    }));
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.lp_token.clone().to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: receiver,
            amount,
        })?,
        funds: vec![],
    }));
    Ok(response
        .add_attribute("action", "unstake")
        .add_attribute("from", info.sender.to_string())
        .add_attribute("amount", amount.to_string())
        .add_messages(msgs))
}

pub fn distribute_eclipse_rewards(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    assets: Vec<Asset>,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let reward_cfg = REWARD_CONFIG.load(deps.storage)?;
    let mut msgs = vec![];
    for asset in assets {
        if asset.info.to_string() == cfg.astro.clone() {
            let ce_holders_rewards = asset
                .amount
                .multiply_ratio(reward_cfg.ce_holders, 10_000 - reward_cfg.users);
            let stability_pool_rewards = asset
                .amount
                .multiply_ratio(reward_cfg.stability_pool, 10_000 - reward_cfg.users);
            let treasury_rewards = asset
                .amount
                .checked_sub(ce_holders_rewards)
                .unwrap_or_default()
                .checked_sub(stability_pool_rewards)
                .unwrap_or_default();
            if ce_holders_rewards.gt(&Uint128::zero()) {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.astro_staking.to_string(),
                    msg: to_json_binary(&StakingExecuteMsg::Enter {
                        receiver: Some(cfg.ce_reward_distributor.clone().unwrap().to_string()),
                    })?,
                    funds: vec![coin(ce_holders_rewards.u128(), cfg.astro.clone())],
                }));
            }
            if stability_pool_rewards.gt(&Uint128::zero()) {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.astro_staking.to_string(),
                    msg: to_json_binary(&StakingExecuteMsg::Enter {
                        receiver: Some(cfg.stability_pool.clone().unwrap().to_string()),
                    })?,
                    funds: vec![coin(stability_pool_rewards.u128(), cfg.astro.clone())],
                }));
            }
            if treasury_rewards.gt(&Uint128::zero()) {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.astro_staking.to_string(),
                    msg: to_json_binary(&StakingExecuteMsg::Enter {
                        receiver: Some(cfg.treasury.clone().to_string()),
                    })?,
                    funds: vec![coin(treasury_rewards.u128(), cfg.astro.clone())],
                }));
            }
        }
    }
    Ok(Response::new().add_messages(msgs))
}
