use astroport::{
    asset::{Asset, AssetInfo},
    incentives::{
        Cw20Msg as IncentivesCw20Msg, ExecuteMsg as IncentivesExecuteMsg,
        QueryMsg as IncentivesQueryMsg,
    },
};
use cosmwasm_std::{
    coins, ensure, ensure_eq, from_json, to_json_binary, Addr, BankMsg, CosmosMsg, Deps, DepsMut,
    Env, MessageInfo, Response, StdResult, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use equinox_msg::lp_staking::{
    CallbackMsg, Cw20HookMsg, RewardConfig, UpdateConfigMsg, UserAstroportReward,
};

use crate::{
    entry::query::{initialize_user_staking_rewards, update_user_staking_rewards},
    error::ContractError,
    state::{CONFIG, LAST_CLAIMED, OWNER, REWARD_CONFIG, STAKING, TOTAL_STAKING},
};

use super::query::update_total_staking_rewards;

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
        config.lp_token = deps.api.addr_validate(&lp_token)?;
        res = res.add_attribute("lp_token", lp_token);
    }
    if let Some(eclip) = new_config.eclip {
        config.eclip = eclip.clone();
        res = res.add_attribute("eclip", eclip);
    }
    if let Some(eclip_daily_reward) = new_config.eclip_daily_reward {
        config.eclip_daily_reward = eclip_daily_reward;
        res = res.add_attribute("eclip_daily_reward", eclip_daily_reward);
    }
    if let Some(astroport_generator) = new_config.astroport_generator {
        config.astroport_generator = deps.api.addr_validate(&astroport_generator)?;
        res = res.add_attribute("astroport_generator", astroport_generator);
    }
    if let Some(treasury) = new_config.treasury {
        config.treasury = deps.api.addr_validate(&treasury)?;
        res = res.add_attribute("treasury", treasury);
    }
    if let Some(stability_pool) = new_config.stability_pool {
        config.stability_pool = deps.api.addr_validate(&stability_pool)?;
        res = res.add_attribute("stability_pool", stability_pool);
    }
    if let Some(ce_reward_distributor) = new_config.ce_reward_distributor {
        config.ce_reward_distributor = Some(deps.api.addr_validate(&ce_reward_distributor)?);
        res = res.add_attribute("ce_reward_distributor", ce_reward_distributor);
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
        CallbackMsg::Claim { user } => claim(deps, env, info, user),
        CallbackMsg::DistributeEclipseRewards { assets } => {
            distribute_eclipse_rewards(deps, env, info, assets)
        }
    }
}

/// Cw20 Receive hook msg handler.
pub fn receive_cw20(
    deps: DepsMut,
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

            let mut total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
            let mut user_staking = STAKING
                .load(deps.storage, &msg.sender.to_string())
                .unwrap_or_default();
            let mut msgs = vec![];
            let mut pending_eclipse_rewards = vec![];

            if total_staking.total_staked.gt(&Uint128::zero()) {
                let res = update_total_staking_rewards(
                    deps.as_ref(),
                    env.contract.address.clone(),
                    env.block.time.seconds(),
                )?;
                total_staking = res.0;
                pending_eclipse_rewards = res.1;
            }

            total_staking.total_staked =
                total_staking.total_staked.checked_add(msg.amount).unwrap();
            TOTAL_STAKING.save(deps.storage, &total_staking)?;

            if user_staking.staked.gt(&Uint128::zero()) {
                user_staking =
                    update_user_staking_rewards(deps.as_ref(), msg.sender.clone(), total_staking)?;
            } else {
                user_staking = initialize_user_staking_rewards(total_staking)?;
            }
            user_staking.staked = user_staking.staked.checked_add(msg.amount).unwrap();
            STAKING.save(deps.storage, &msg.sender, &user_staking)?;
            LAST_CLAIMED.save(deps.storage, &env.block.time.seconds())?;

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

            if pending_eclipse_rewards.len() > 0 {
                msgs.push(
                    CallbackMsg::DistributeEclipseRewards {
                        assets: pending_eclipse_rewards,
                    }
                    .to_cosmos_msg(&env)?,
                );
            }

            Ok(Response::new()
                .add_attribute("action", "stake")
                .add_attribute("sender", msg.sender.clone().to_string())
                .add_attribute("amount", msg.amount.to_string())
                .add_messages(msgs))
        }
    }
}

/// Claim user rewards
pub fn claim(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    sender: String,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    let mut user_staking = STAKING.load(deps.storage, &sender).unwrap_or_default();
    let mut msgs = vec![];
    let mut pending_eclipse_rewards = vec![];

    if total_staking.total_staked.gt(&Uint128::zero()) {
        let res = update_total_staking_rewards(
            deps.as_ref(),
            env.contract.address.clone(),
            env.block.time.seconds(),
        )?;
        total_staking = res.0;
        pending_eclipse_rewards = res.1;
    }

    TOTAL_STAKING.save(deps.storage, &total_staking)?;

    if user_staking.staked.gt(&Uint128::zero()) {
        user_staking = update_user_staking_rewards(deps.as_ref(), sender.clone(), total_staking)?;
    }
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
        .add_attribute("user", sender.clone());
    user_staking.astroport_rewards = user_staking
        .astroport_rewards
        .into_iter()
        .map(|mut pending_reward| {
            if pending_reward.amount.gt(&Uint128::zero()) {
                match pending_reward.asset.clone() {
                    AssetInfo::NativeToken { denom } => msgs.push(CosmosMsg::Bank(BankMsg::Send {
                        to_address: sender.clone(),
                        amount: coins(pending_reward.amount.u128(), denom),
                    })),
                    AssetInfo::Token { contract_addr } => {
                        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: contract_addr.to_string(),
                            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                                recipient: sender.clone(),
                                amount: pending_reward.amount,
                            })?,
                            funds: vec![],
                        }))
                    }
                }
            }
            response = response
                .clone()
                .add_attribute("asset", pending_reward.asset.clone().to_string())
                .add_attribute("amount", pending_reward.amount);
            pending_reward.amount = Uint128::zero();
            Ok(pending_reward)
        })
        .collect::<Result<Vec<UserAstroportReward>, ContractError>>()
        .unwrap();
    if user_staking.pending_eclip_rewards.gt(&Uint128::zero()) {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: sender.clone(),
            amount: coins(user_staking.pending_eclip_rewards.u128(), cfg.eclip.clone()),
        }))
    }
    response = response
        .add_attribute("asset", cfg.eclip.to_string())
        .add_attribute("amount", user_staking.pending_eclip_rewards.clone());

    user_staking.pending_eclip_rewards = Uint128::zero();
    STAKING.save(deps.storage, &sender, &user_staking)?;
    LAST_CLAIMED.save(deps.storage, &env.block.time.seconds())?;

    if pending_eclipse_rewards.len() > 0 {
        msgs.push(
            CallbackMsg::DistributeEclipseRewards {
                assets: pending_eclipse_rewards,
            }
            .to_cosmos_msg(&env)?,
        );
    }

    Ok(response.add_messages(msgs))
}

/// Unstake amount and claim rewards of user
/// check unstake amount
pub fn unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    let mut user_staking = STAKING.load(deps.storage, &info.sender.to_string())?;
    let mut total_staking = TOTAL_STAKING.load(deps.storage)?;
    let mut msgs = vec![];
    let mut pending_eclipse_rewards = vec![];

    ensure!(
        amount.le(&user_staking.staked),
        ContractError::ExeedingUnstakeAmount {
            got: amount.u128(),
            expected: user_staking.staked.u128()
        }
    );

    if total_staking.total_staked.gt(&Uint128::zero()) {
        let res = update_total_staking_rewards(
            deps.as_ref(),
            env.contract.address.clone(),
            env.block.time.seconds(),
        )?;
        total_staking = res.0;
        pending_eclipse_rewards = res.1;
    }

    total_staking.total_staked = total_staking.total_staked.checked_sub(amount).unwrap();
    TOTAL_STAKING.save(deps.storage, &total_staking)?;

    if user_staking.staked.gt(&Uint128::zero()) {
        user_staking = update_user_staking_rewards(
            deps.as_ref(),
            info.sender.clone().to_string(),
            total_staking,
        )?;
    }

    user_staking.staked = user_staking.staked.checked_sub(amount).unwrap();
    STAKING.save(deps.storage, &info.sender.to_string(), &user_staking)?;

    // send lp_token to user, send unstake message to reward contract
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
            recipient: info.sender.to_string(),
            amount,
        })?,
        funds: vec![],
    }));
    msgs.push(
        CallbackMsg::Claim {
            user: info.sender.to_string(),
        }
        .to_cosmos_msg(&env)?,
    );
    if pending_eclipse_rewards.len() > 0 {
        msgs.push(
            CallbackMsg::DistributeEclipseRewards {
                assets: pending_eclipse_rewards,
            }
            .to_cosmos_msg(&env)?,
        );
    }
    Ok(Response::new()
        .add_attribute("action", "unstake")
        .add_attribute("from", info.sender.to_string())
        .add_attribute("amount", amount.to_string())
        .add_messages(msgs))
}

pub fn get_incentive_pending_rewards(deps: Deps, contract: Addr) -> StdResult<Vec<Asset>> {
    let cfg = CONFIG.load(deps.storage)?;
    deps.querier.query_wasm_smart(
        &cfg.astroport_generator,
        &IncentivesQueryMsg::PendingRewards {
            lp_token: cfg.lp_token.to_string(),
            user: contract.to_string(),
        },
    )
}

pub fn calculate_eclip_rewards(deps: Deps, current_time: u64) -> StdResult<Uint128> {
    let cfg = CONFIG.load(deps.storage)?;
    let last_claimed = LAST_CLAIMED.load(deps.storage).unwrap_or(current_time);
    let pending_rewards = cfg
        .eclip_daily_reward
        .multiply_ratio(current_time - last_claimed, 86400u64);
    Ok(pending_rewards)
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
        match asset.info {
            AssetInfo::NativeToken { denom: _ } => {}
            AssetInfo::Token { contract_addr } => {
                if contract_addr == cfg.astro {
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
                            contract_addr: cfg.astro.to_string(),
                            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                                recipient: cfg.ce_reward_distributor.clone().unwrap().to_string(),
                                amount: ce_holders_rewards,
                            })?,
                            funds: vec![],
                        }));
                    }
                    if stability_pool_rewards.gt(&Uint128::zero()) {
                        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: cfg.astro.to_string(),
                            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                                recipient: cfg.stability_pool.to_string(),
                                amount: stability_pool_rewards,
                            })?,
                            funds: vec![],
                        }));
                    }
                    if treasury_rewards.gt(&Uint128::zero()) {
                        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: cfg.astro.to_string(),
                            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                                recipient: cfg.treasury.to_string(),
                                amount: treasury_rewards,
                            })?,
                            funds: vec![],
                        }));
                    }
                }
            }
        }
    }
    Ok(Response::new().add_messages(msgs))
}
