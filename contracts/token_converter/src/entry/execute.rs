use std::str::FromStr;

use cosmwasm_std::{
    ensure, ensure_eq, ensure_ne, from_json, to_json_binary, DepsMut, Env, MessageInfo, Reply,
    ReplyOn, Response, SubMsg, Uint128, WasmMsg,
};

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use equinox_msg::token_converter::{Cw20HookMsg, RewardConfig, UpdateConfig};
use equinox_msg::voter::{
    Cw20HookMsg as VoterCw20HookMsg, ExecuteMsg as VoterExecuteMsg, QueryMsg as VoterQueryMsg,
};

use crate::{
    contract::STAKE_TOKEN_REPLY_ID,
    error::ContractError,
    math::{calculate_claimable, convert_token},
    state::{
        UserStake, CONFIG, OWNER, REWARD_CONFIG, TOTAL_STAKE_INFO, TREASURY_REWARD, USER_STAKING,
        WITHDRAWABLE_BALANCE,
    },
};

/// Update config
pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_config: UpdateConfig,
) -> Result<Response, ContractError> {
    // only owner can executable
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let mut config = CONFIG.load(deps.storage)?;
    let mut res: Response = Response::new().add_attribute("action", "update config");
    if let Some(token_in) = new_config.token_in {
        config.token_in = deps.api.addr_validate(&token_in)?;
        res = res.add_attribute("token_in", token_in);
    }
    if let Some(token_out) = new_config.token_out {
        config.token_out = deps.api.addr_validate(&token_out)?;
        res = res.add_attribute("token_out", token_out);
    }
    if let Some(xtoken) = new_config.xtoken {
        config.xtoken = deps.api.addr_validate(&xtoken)?;
        res = res.add_attribute("xtoken", xtoken);
    }
    if let Some(vxtoken_holder) = new_config.vxtoken_holder {
        config.vxtoken_holder = deps.api.addr_validate(&vxtoken_holder)?;
        res = res.add_attribute("vxtoken_holder", vxtoken_holder);
    }
    if let Some(treasury) = new_config.treasury {
        config.treasury = deps.api.addr_validate(&treasury)?;
        res = res.add_attribute("treasury", treasury);
    }
    if let Some(stability_pool) = new_config.stability_pool {
        config.stability_pool = deps.api.addr_validate(&stability_pool)?;
        res = res.add_attribute("stability_pool", stability_pool);
    }
    if let Some(staking_reward_distributor) = new_config.staking_reward_distributor {
        config.staking_reward_distributor = deps.api.addr_validate(&staking_reward_distributor)?;
        res = res.add_attribute("staking_reward_distributor", staking_reward_distributor);
    }
    if let Some(ce_reward_distributor) = new_config.ce_reward_distributor {
        config.ce_reward_distributor = deps.api.addr_validate(&ce_reward_distributor)?;
        res = res.add_attribute("ce_reward_distributor", ce_reward_distributor);
    }
    CONFIG.save(deps.storage, &config)?;
    Ok(res)
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

/// Update owner
pub fn update_owner(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response, ContractError> {
    // only owner can executable
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let new_owner_addr = deps.api.addr_validate(&new_owner)?;
    OWNER.set(deps.branch(), Some(new_owner_addr))?;
    Ok(Response::new()
        .add_attribute("action", "update owner")
        .add_attribute("to", new_owner))
}

/// claim user rewards
pub fn claim(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let reward_config = REWARD_CONFIG.load(deps.storage)?;
    // only staking reward distributor contract can execute this function
    ensure_eq!(
        info.sender,
        config.staking_reward_distributor,
        ContractError::Unauthorized {}
    );
    // ASTRO / xASTRO rate from voter contract
    let (total_deposit, total_shares): (Uint128, Uint128) = deps.querier.query_wasm_smart(
        config.vxtoken_holder.to_string(),
        &VoterQueryMsg::ConvertRatio {},
    )?;
    let mut total_stake_info = TOTAL_STAKE_INFO.load(deps.storage).unwrap_or_default();
    let mut withdrawable = WITHDRAWABLE_BALANCE.load(deps.storage).unwrap_or_default();
    let mut treasury_reward = TREASURY_REWARD.load(deps.storage).unwrap_or_default();

    // calculate user rewards as xASTRO
    let claimable = calculate_claimable(
        total_stake_info.xtoken,
        total_stake_info.stake,
        total_shares,
        total_deposit,
        total_stake_info.claimed,
    );
    // calculate users reward as xASTRO
    let users_reward = claimable.multiply_ratio(reward_config.users, 10000u32);
    // must exist users reward
    ensure_ne!(
        users_reward,
        Uint128::zero(),
        ContractError::NoRewardClaimable {}
    );
    // add message to mint eclipASTRO to staking_reward_distributor
    let mut msgs = vec![WasmMsg::Execute {
        contract_addr: config.token_out.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Mint {
            recipient: config.staking_reward_distributor.to_string(),
            amount: convert_token(users_reward, total_shares, total_deposit),
        })?,
        funds: vec![],
    }];
    // withdrawable is withdrawable xASTRO amount that DAO withdraws. equal to staking eclipASTRO rewards part
    withdrawable = withdrawable.checked_add(users_reward).unwrap();
    WITHDRAWABLE_BALANCE.save(deps.storage, &withdrawable)?;

    // check dao claimable
    let dao_reward_point =
        reward_config.treasury + reward_config.ce_holders + reward_config.stability_pool;
    let dao_claimable = claimable.checked_sub(users_reward).unwrap();
    // amount to withdraw for staking pools
    let reward_ce_holders =
        dao_claimable.multiply_ratio(reward_config.ce_holders, dao_reward_point);
    let reward_stability_pool =
        dao_claimable.multiply_ratio(reward_config.stability_pool, dao_reward_point);
    // add messsage to withdraw xASTRO DAO rewards - treasury rewards
    if reward_ce_holders.gt(&Uint128::zero()) {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.vxtoken_holder.to_string(),
            msg: to_json_binary(&VoterExecuteMsg::Withdraw {
                amount: reward_ce_holders,
                recipient: config.ce_reward_distributor.to_string(),
            })?,
            funds: vec![],
        });
    }
    if reward_stability_pool.gt(&Uint128::zero()) {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.vxtoken_holder.to_string(),
            msg: to_json_binary(&VoterExecuteMsg::Withdraw {
                amount: reward_stability_pool,
                recipient: config.stability_pool.to_string(),
            })?,
            funds: vec![],
        });
    }
    // deduct claimable
    total_stake_info.claimed = total_stake_info.claimed.checked_add(claimable).unwrap();
    treasury_reward = treasury_reward
        .checked_add(
            dao_claimable
                .checked_sub(reward_ce_holders)
                .unwrap()
                .checked_sub(reward_stability_pool)
                .unwrap(),
        )
        .unwrap();
    TOTAL_STAKE_INFO.save(deps.storage, &total_stake_info)?;
    TREASURY_REWARD.save(deps.storage, &treasury_reward)?;

    Ok(Response::new()
        .add_attribute("action", "claim reward")
        .add_attribute("token", "eclipASTRO")
        .add_attribute("recipient", config.staking_reward_distributor.to_string())
        .add_attribute("amount", users_reward.to_string())
        .add_attribute("token", "xASTRO")
        .add_attribute("recipient", config.ce_reward_distributor.to_string())
        .add_attribute("amount", reward_ce_holders.to_string())
        .add_attribute("token", "xASTRO")
        .add_attribute("recipient", config.stability_pool)
        .add_attribute("amount", reward_stability_pool.to_string())
        .add_messages(msgs))
}

/// claim treasury rewards
pub fn claim_treasury_reward(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let reward_config = REWARD_CONFIG.load(deps.storage)?;
    let treasury_reward = TREASURY_REWARD.load(deps.storage).unwrap_or_default();
    let mut total_stake_info = TOTAL_STAKE_INFO.load(deps.storage).unwrap_or_default();
    // ASTRO / xASTRO
    let (total_deposit, total_shares): (Uint128, Uint128) = deps.querier.query_wasm_smart(
        config.vxtoken_holder.to_string(),
        &VoterQueryMsg::ConvertRatio {},
    )?;
    let dao_reward_point =
        reward_config.treasury + reward_config.ce_holders + reward_config.stability_pool;
    let claimable = calculate_claimable(
        total_stake_info.xtoken,
        total_stake_info.stake,
        total_shares,
        total_deposit,
        total_stake_info.claimed,
    );
    let users_reward = claimable.multiply_ratio(reward_config.users, 10000u32);
    let mut msgs = vec![];
    if users_reward.gt(&Uint128::zero()) {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.token_out.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Mint {
                recipient: config.staking_reward_distributor.to_string(),
                amount: convert_token(users_reward, total_shares, total_deposit),
            })?,
            funds: vec![],
        });
    }
    let dao_claimable = claimable.checked_sub(users_reward).unwrap();
    // amount to withdraw for staking pools
    let reward_ce_holders =
        dao_claimable.multiply_ratio(reward_config.ce_holders, dao_reward_point);
    let reward_stability_pool =
        dao_claimable.multiply_ratio(reward_config.stability_pool, dao_reward_point);
    // add messsage to withdraw xASTRO DAO rewards - treasury rewards
    if reward_ce_holders.gt(&Uint128::zero()) {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.vxtoken_holder.to_string(),
            msg: to_json_binary(&VoterExecuteMsg::Withdraw {
                amount: reward_ce_holders,
                recipient: config.ce_reward_distributor.to_string(),
            })?,
            funds: vec![],
        });
    }
    if reward_stability_pool.gt(&Uint128::zero()) {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.vxtoken_holder.to_string(),
            msg: to_json_binary(&VoterExecuteMsg::Withdraw {
                amount: reward_stability_pool,
                recipient: config.stability_pool.to_string(),
            })?,
            funds: vec![],
        });
    }
    let mut treasury_reward_withdrawable = treasury_reward
        .checked_add(
            dao_claimable
                .checked_sub(reward_ce_holders)
                .unwrap()
                .checked_sub(reward_stability_pool)
                .unwrap(),
        )
        .unwrap();
    ensure!(
        amount.le(&treasury_reward_withdrawable),
        ContractError::NotEnoughBalance {}
    );
    treasury_reward_withdrawable -= amount;
    total_stake_info.claimed = total_stake_info.claimed.checked_add(claimable).unwrap();
    TOTAL_STAKE_INFO.save(deps.storage, &total_stake_info)?;
    TREASURY_REWARD.save(deps.storage, &treasury_reward_withdrawable)?;
    msgs.push(WasmMsg::Execute {
        contract_addr: config.vxtoken_holder.to_string(),
        msg: to_json_binary(&VoterExecuteMsg::Withdraw {
            amount,
            recipient: config.treasury.to_string(),
        })?,
        funds: vec![],
    });
    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "claim treasury reward")
        .add_attribute("amount", amount.to_string()))
}

/// withdraw xtoken
pub fn withdraw_xtoken(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
    recipient: String,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let config = CONFIG.load(deps.storage)?;
    let mut withdrawable_balance = WITHDRAWABLE_BALANCE.load(deps.storage)?;
    ensure!(
        amount.le(&withdrawable_balance),
        ContractError::NotEnoughBalance {}
    );
    withdrawable_balance -= amount;
    WITHDRAWABLE_BALANCE.save(deps.storage, &withdrawable_balance)?;
    Ok(Response::new()
        .add_message(WasmMsg::Execute {
            contract_addr: config.vxtoken_holder.to_string(),
            msg: to_json_binary(&VoterExecuteMsg::Withdraw { amount, recipient })?,
            funds: vec![],
        })
        .add_attribute("action", "withdraw xtoken")
        .add_attribute("amount", amount.to_string()))
}

/// Cw20 Receive hook msg handler.
pub fn receive_cw20(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_json(&msg.msg) {
        Ok(Cw20HookMsg::Convert {}) => {
            let config = CONFIG.load(deps.storage)?;
            let mut amount = msg.amount;
            // only ASTRO token or xASTRO token can execute this message
            ensure!(
                info.sender == config.token_in || info.sender == config.xtoken,
                ContractError::UnknownToken(info.sender.to_string())
            );
            // send stake message to vxtoken holder contract and handle response
            let mut stake_msg = SubMsg {
                id: STAKE_TOKEN_REPLY_ID,
                msg: WasmMsg::Execute {
                    contract_addr: config.token_in.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Send {
                        contract: config.vxtoken_holder.to_string(),
                        amount,
                        msg: to_json_binary(&VoterCw20HookMsg::Stake {})?,
                    })?,
                    funds: vec![],
                }
                .into(),
                gas_limit: None,
                reply_on: ReplyOn::Success,
            };
            if info.sender == config.xtoken {
                let (total_deposit, total_shares): (Uint128, Uint128) =
                    deps.querier.query_wasm_smart(
                        config.vxtoken_holder.to_string(),
                        &VoterQueryMsg::ConvertRatio {},
                    )?;
                stake_msg = SubMsg {
                    id: STAKE_TOKEN_REPLY_ID,
                    msg: WasmMsg::Execute {
                        contract_addr: config.xtoken.to_string(),
                        msg: to_json_binary(&Cw20ExecuteMsg::Send {
                            contract: config.vxtoken_holder.to_string(),
                            amount,
                            msg: to_json_binary(&VoterCw20HookMsg::Stake {})?,
                        })?,
                        funds: vec![],
                    }
                    .into(),
                    gas_limit: None,
                    reply_on: ReplyOn::Success,
                };
                // calculate related ASTRO amount from xASTRO amount
                amount = amount.multiply_ratio(total_deposit, total_shares);
            }

            let cw20_sender = deps.api.addr_validate(&msg.sender)?;
            // save user staking data temporarily
            USER_STAKING.save(
                deps.storage,
                &UserStake {
                    user: cw20_sender.to_string(),
                    stake: amount,
                },
            )?;
            Ok(Response::new().add_submessage(stake_msg))
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
            if attr.key == "xASTRO" {
                xtoken_amount = Uint128::from_str(&attr.value).unwrap();
            }
        }
    }
    // update user staking info, total staking info
    let config = CONFIG.load(deps.storage)?;
    let user_staking = USER_STAKING.load(deps.storage)?;
    let mut total_stake_info = TOTAL_STAKE_INFO.load(deps.storage).unwrap_or_default();
    total_stake_info.stake = total_stake_info
        .stake
        .checked_add(user_staking.stake)
        .unwrap();
    total_stake_info.xtoken = total_stake_info.xtoken.checked_add(xtoken_amount).unwrap();
    TOTAL_STAKE_INFO.save(deps.storage, &total_stake_info)?;
    // mint eclipASTRO to user
    let msg = WasmMsg::Execute {
        contract_addr: config.token_out.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Mint {
            recipient: user_staking.user.clone(),
            amount: user_staking.stake,
        })?,
        funds: vec![],
    };
    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "convert token")
        .add_attribute("from", "ASTRO")
        .add_attribute("to", "eclipASTRO")
        .add_attribute("user", user_staking.user)
        .add_attribute("amount", user_staking.stake.to_string()))
}
