use std::vec;

use cosmwasm_std::{
    ensure, ensure_eq, from_json, to_json_binary, DepsMut, Env, MessageInfo, Response, Uint128,
    WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::{
    error::ContractError,
    state::{ALLOWED_USERS, CONFIG, OWNER, STAKING, TOTAL_STAKING},
};
use equinox_msg::{
    flexible_staking::{Cw20HookMsg, UpdateConfigMsg},
    reward_distributor::ExecuteMsg as RewardDistributorExecuteMsg,
    timelock_staking::Cw20HookMsg as TimelockStakingCw20HookMsg,
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
    if let Some(token) = new_config.token {
        config.token = deps.api.addr_validate(&token)?;
        res = res.add_attribute("token", token);
    }
    if let Some(reward_contract) = new_config.reward_contract {
        config.reward_contract = deps.api.addr_validate(&reward_contract)?;
        res = res.add_attribute("reward_contract", reward_contract);
    }
    if let Some(timelock_contract) = new_config.timelock_contract {
        config.timelock_contract = deps.api.addr_validate(&timelock_contract)?;
        res = res.add_attribute("timelock_contract", timelock_contract);
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

pub fn allow_users(
    deps: DepsMut,
    info: MessageInfo,
    users: Vec<String>,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    for user in users {
        ensure_eq!(
            ALLOWED_USERS.load(deps.storage, &user).unwrap_or_default(),
            false,
            ContractError::DuplicatedAddress(user)
        );
        ALLOWED_USERS.save(deps.storage, &user, &true)?;
    }
    Ok(Response::new().add_attribute("action", "update allowed users"))
}

pub fn block_users(
    deps: DepsMut,
    info: MessageInfo,
    users: Vec<String>,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    for user in users {
        ensure_eq!(
            ALLOWED_USERS.load(deps.storage, &user)?,
            true,
            ContractError::NotAllowed(user)
        );
        ALLOWED_USERS.remove(deps.storage, &user);
    }
    Ok(Response::new().add_attribute("action", "update allowed users"))
}

/// Cw20 Receive hook msg handler.
pub fn receive_cw20(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_eq!(
        config.token,
        info.sender,
        ContractError::Cw20AddressesNotMatch {
            got: info.sender.to_string(),
            expected: config.token.to_string(),
        }
    );
    ensure!(
        msg.amount.gt(&Uint128::zero()),
        ContractError::ZeroAmount {}
    );
    match from_json(&msg.msg)? {
        // stake eclipASTRO token
        // non zero amount
        // update user staking, total staking amount
        // send stake msg to reward distributor contract
        Cw20HookMsg::Stake {} => {
            let mut total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
            let mut user_staking = STAKING
                .load(deps.storage, &msg.sender.to_string())
                .unwrap_or_default();

            total_staking = total_staking.checked_add(msg.amount).unwrap();
            user_staking = user_staking.checked_add(msg.amount).unwrap();

            STAKING.save(deps.storage, &msg.sender, &user_staking)?;
            TOTAL_STAKING.save(deps.storage, &total_staking)?;

            // send stake message to reward_contract
            let stake_msg = WasmMsg::Execute {
                contract_addr: config.reward_contract.to_string(),
                msg: to_json_binary(&RewardDistributorExecuteMsg::FlexibleStake {
                    user: msg.sender.clone(),
                    amount: msg.amount,
                })?,
                funds: vec![],
            };
            Ok(Response::new()
                .add_attribute("action", "stake")
                .add_attribute("sender", msg.sender.clone().to_string())
                .add_attribute("amount", msg.amount.to_string())
                .add_message(stake_msg))
        }
        Cw20HookMsg::Relock {
            duration,
            amount,
            recipient,
        } => {
            let mut user_staking = STAKING.load(deps.storage, &msg.sender.to_string())?;
            let mut total_staking = TOTAL_STAKING.load(deps.storage)?;
            let sender = msg.sender;
            let is_allowed_user = ALLOWED_USERS
                .load(deps.storage, &sender)
                .unwrap_or_default();
            let amount = match amount {
                Some(a) => {
                    if is_allowed_user && a.le(&user_staking) {
                        a
                    } else {
                        user_staking
                    }
                }
                None => user_staking,
            };
            let recipient = recipient.unwrap_or(sender.clone());

            user_staking = user_staking.checked_sub(amount).unwrap();
            total_staking = total_staking.checked_sub(amount).unwrap();

            // update user staking, total staking amount
            STAKING.save(deps.storage, &sender, &user_staking)?;
            TOTAL_STAKING.save(deps.storage, &total_staking)?;

            let wasm_msg = vec![
                WasmMsg::Execute {
                    contract_addr: config.token.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Send {
                        contract: config.timelock_contract.to_string(),
                        amount: amount + msg.amount,
                        msg: to_json_binary(&TimelockStakingCw20HookMsg::Lock {
                            duration,
                            recipient: Some(recipient),
                        })?,
                    })?,
                    funds: vec![],
                },
                WasmMsg::Execute {
                    contract_addr: config.reward_contract.to_string(),
                    msg: to_json_binary(&RewardDistributorExecuteMsg::FlexibleUnstake {
                        user: sender.clone(),
                        amount,
                    })?,
                    funds: vec![],
                },
            ];
            Ok(Response::new()
                .add_attribute("action", "relock")
                .add_attribute("from", info.sender.to_string())
                .add_attribute("amount", (amount + msg.amount).to_string())
                .add_messages(wasm_msg))
        }
    }
}

/// Claim user rewards
/// send claim message to reward distributor contract
pub fn claim(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let claim_msg = WasmMsg::Execute {
        contract_addr: config.reward_contract.to_string(),
        msg: to_json_binary(&RewardDistributorExecuteMsg::FlexibleStakeClaim {
            user: info.sender.to_string(),
        })?,
        funds: vec![],
    };
    Ok(Response::new()
        .add_attribute("action", "claim")
        .add_message(claim_msg))
}

/// Unstake amount and claim rewards of user
/// check unstake amount
pub fn unstake(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let mut user_staking = STAKING.load(deps.storage, &info.sender.to_string())?;
    let mut total_staking = TOTAL_STAKING.load(deps.storage)?;
    let recipient = recipient.unwrap_or(info.sender.to_string());

    ensure!(
        amount.le(&user_staking),
        ContractError::ExeedingUnstakeAmount {
            got: amount.u128(),
            expected: user_staking.u128()
        }
    );

    user_staking = user_staking.checked_sub(amount).unwrap();
    total_staking = total_staking.checked_sub(amount).unwrap();

    // update user staking, total staking amount
    STAKING.save(deps.storage, &info.sender.to_string(), &user_staking)?;
    TOTAL_STAKING.save(deps.storage, &total_staking)?;

    // send eclipASTRO to user, send unstake message to reward contract
    let msg = vec![
        WasmMsg::Execute {
            contract_addr: config.token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer { recipient, amount })?,
            funds: vec![],
        },
        WasmMsg::Execute {
            contract_addr: config.reward_contract.to_string(),
            msg: to_json_binary(&RewardDistributorExecuteMsg::FlexibleUnstake {
                user: info.sender.to_string(),
                amount,
            })?,
            funds: vec![],
        },
    ];
    Ok(Response::new()
        .add_attribute("action", "unstake")
        .add_attribute("from", info.sender.to_string())
        .add_attribute("amount", amount.to_string())
        .add_messages(msg))
}

pub fn handle_lock(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
    duration: u64,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let mut user_staking = STAKING.load(deps.storage, &info.sender.to_string())?;
    let mut total_staking = TOTAL_STAKING.load(deps.storage)?;

    let is_allowed_user = ALLOWED_USERS
        .load(deps.storage, &info.sender.to_string())
        .unwrap_or_default();
    let amount = match amount {
        Some(a) => {
            if is_allowed_user && a.le(&user_staking) {
                a
            } else {
                user_staking
            }
        }
        None => user_staking,
    };
    let recipient = recipient.unwrap_or(info.sender.to_string());

    ensure!(
        amount.le(&user_staking),
        ContractError::ExeedingUnstakeAmount {
            got: amount.u128(),
            expected: user_staking.u128()
        }
    );

    user_staking = user_staking.checked_sub(amount).unwrap();
    total_staking = total_staking.checked_sub(amount).unwrap();

    // update user staking, total staking amount
    STAKING.save(deps.storage, &info.sender.to_string(), &user_staking)?;
    TOTAL_STAKING.save(deps.storage, &total_staking)?;

    // send eclipASTRO to user, send unstake message to reward contract
    let msg = vec![
        WasmMsg::Execute {
            contract_addr: config.token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: config.timelock_contract.to_string(),
                amount,
                msg: to_json_binary(&TimelockStakingCw20HookMsg::Lock {
                    duration,
                    recipient: Some(recipient),
                })?,
            })?,
            funds: vec![],
        },
        WasmMsg::Execute {
            contract_addr: config.reward_contract.to_string(),
            msg: to_json_binary(&RewardDistributorExecuteMsg::FlexibleUnstake {
                user: info.sender.to_string(),
                amount,
            })?,
            funds: vec![],
        },
    ];
    Ok(Response::new()
        .add_attribute("action", "relock")
        .add_attribute("from", info.sender.to_string())
        .add_attribute("amount", amount.to_string())
        .add_messages(msg))
}
