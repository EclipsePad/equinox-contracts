use cosmwasm_std::{
    ensure, ensure_eq, from_json, to_json_binary, DepsMut, Env, MessageInfo, Response, Uint128,
    WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::{
    entry::query::calculate_penalty,
    error::ContractError,
    state::{ALLOWED_USERS, CONFIG, OWNER, STAKING, TOTAL_STAKING, TOTAL_STAKING_BY_DURATION},
};

use equinox_msg::{
    reward_distributor::ExecuteMsg as RewardDistributorExecuteMsg,
    timelock_staking::{Cw20HookMsg, RelockingDetail, UpdateConfigMsg},
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
    if let Some(timelock_config) = new_config.timelock_config {
        config.timelock_config = timelock_config.clone();
        res = res.add_attribute(
            "timelock_config",
            timelock_config
                .into_iter()
                .map(|i| {
                    "(".to_string()
                        + &i.duration.to_string()
                        + ","
                        + &i.early_unlock_penalty_bps.to_string()
                        + ")"
                })
                .collect::<Vec<String>>()
                .join(","),
        )
    }
    CONFIG.save(deps.storage, &config)?;
    Ok(res)
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

/// Update owner
/// Only owner
pub fn update_owner(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response, ContractError> {
    // only owner can update owner
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let new_owner_addr = deps.api.addr_validate(&new_owner)?;
    OWNER.set(deps.branch(), Some(new_owner_addr))?;
    Ok(Response::new()
        .add_attribute("action", "update owner")
        .add_attribute("to", new_owner))
}

/// Cw20 Receive hook msg handler.
pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
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
        // lock eclipASTRO token with duration
        // check amount, duration
        Cw20HookMsg::Lock {
            duration,
            recipient,
        } => {
            let recipient = recipient.unwrap_or(msg.sender);
            ensure!(
                config
                    .timelock_config
                    .into_iter()
                    .any(|i| i.duration == duration),
                ContractError::NoLockingPeriodFound(duration)
            );

            let mut user_staking = STAKING
                .load(
                    deps.storage,
                    (&recipient.to_string(), duration, env.block.time.seconds()),
                )
                .unwrap_or_default();
            let mut total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
            let mut total_staking_by_duration = TOTAL_STAKING_BY_DURATION
                .load(deps.storage, duration)
                .unwrap_or_default();
            user_staking = user_staking.checked_add(msg.amount).unwrap();
            total_staking = total_staking.checked_add(msg.amount).unwrap();
            total_staking_by_duration = total_staking_by_duration.checked_add(msg.amount).unwrap();
            STAKING.save(
                deps.storage,
                (&recipient.to_string(), duration, env.block.time.seconds()),
                &user_staking,
            )?;
            TOTAL_STAKING.save(deps.storage, &total_staking)?;
            TOTAL_STAKING_BY_DURATION.save(deps.storage, duration, &total_staking_by_duration)?;

            // send stake message to reward_contract
            let stake_msg = WasmMsg::Execute {
                contract_addr: config.reward_contract.to_string(),
                msg: to_json_binary(&RewardDistributorExecuteMsg::TimelockStake {
                    user: recipient.to_string(),
                    amount: msg.amount,
                    duration,
                })?,
                funds: vec![],
            };
            Ok(Response::new().add_message(stake_msg))
        }
        Cw20HookMsg::Relock {
            from_duration,
            to_duration,
            relocks,
            recipient,
        } => {
            let sender = deps.api.addr_validate(&msg.sender)?;
            let recipient = deps
                .api
                .addr_validate(&recipient.unwrap_or(sender.to_string()))?;

            let (relock_amount, relocking) = _relock(
                deps,
                env,
                msg.amount,
                RelockingDetail {
                    sender: sender.clone(),
                    recipient: recipient.to_string(),
                    relocks,
                    from_duration,
                    to_duration,
                },
            )?;
            ensure!(
                from_duration <= to_duration,
                ContractError::ExtendDurationErr(from_duration, to_duration)
            );

            // send stake message to reward_contract
            let relock_msg = WasmMsg::Execute {
                contract_addr: config.reward_contract.to_string(),
                msg: to_json_binary(&RewardDistributorExecuteMsg::Relock {
                    from: sender.clone(),
                    to: recipient.clone(),
                    relocking,
                    adding_amount: Some(msg.amount),
                    from_duration,
                    to_duration,
                })?,
                funds: vec![],
            };
            Ok(Response::new()
                .add_attribute("action", "add lock")
                .add_attribute("user", sender.to_string())
                .add_attribute("amount", msg.amount.to_string())
                .add_attribute("action", "extend duration")
                .add_attribute("from", from_duration.to_string())
                .add_attribute("user", sender.to_string())
                .add_attribute("amount", relock_amount)
                .add_attribute("to", to_duration.to_string())
                .add_attribute("receiver", recipient.to_string())
                .add_message(relock_msg))
        }
    }
}

/// Claim user rewards
pub fn claim(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    duration: u64,
    locked_at: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let claim_msg = WasmMsg::Execute {
        contract_addr: config.reward_contract.to_string(),
        msg: to_json_binary(&RewardDistributorExecuteMsg::TimelockStakeClaim {
            user: info.sender.to_string(),
            duration,
            locked_at,
        })?,
        funds: vec![],
    };
    Ok(Response::new().add_message(claim_msg))
}

pub fn claim_all(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let claim_msg = WasmMsg::Execute {
        contract_addr: config.reward_contract.to_string(),
        msg: to_json_binary(&RewardDistributorExecuteMsg::TimelockStakeClaimAll {
            user: info.sender.to_string(),
        })?,
        funds: vec![],
    };
    Ok(Response::new().add_message(claim_msg))
}

/// Unlock amount and claim rewards of user
pub fn unlock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    duration: u64,
    locked_at: u64,
    amount: Option<Uint128>,
    receiver: Option<String>,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    let config = CONFIG.load(deps.storage)?;
    let unlock_max_amount = STAKING.load(deps.storage, (&sender, duration, locked_at))?;
    let unlock_amount = amount.unwrap_or(unlock_max_amount);
    let receiver = receiver.unwrap_or(info.sender.to_string());
    ensure!(
        unlock_amount <= unlock_max_amount,
        ContractError::ExceedAmount {}
    );
    ensure!(
        unlock_amount.gt(&Uint128::zero()),
        ContractError::NoLockedAmount {}
    );

    let mut total_staking = TOTAL_STAKING.load(deps.storage)?;
    let mut total_staking_by_duration = TOTAL_STAKING_BY_DURATION.load(deps.storage, duration)?;
    total_staking = total_staking.checked_sub(unlock_amount).unwrap();
    total_staking_by_duration = total_staking_by_duration
        .checked_sub(unlock_amount)
        .unwrap();

    if unlock_amount == unlock_max_amount {
        STAKING.remove(deps.storage, (&sender, duration, locked_at));
    } else {
        let remain = unlock_max_amount.checked_sub(unlock_amount).unwrap();
        STAKING.save(deps.storage, (&sender, duration, locked_at), &remain)?;
    }
    TOTAL_STAKING.save(deps.storage, &total_staking)?;
    TOTAL_STAKING_BY_DURATION.save(deps.storage, duration, &total_staking_by_duration)?;

    let penalty_amount = calculate_penalty(deps.as_ref(), env, unlock_amount, duration, locked_at)?;

    let mut msgs = vec![
        WasmMsg::Execute {
            contract_addr: config.token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: receiver,
                amount: unlock_amount.checked_sub(penalty_amount).unwrap(),
            })?,
            funds: vec![],
        },
        WasmMsg::Execute {
            contract_addr: config.reward_contract.to_string(),
            msg: to_json_binary(&RewardDistributorExecuteMsg::TimelockUnstake {
                user: info.sender.to_string(),
                duration,
                locked_at,
            })?,
            funds: vec![],
        },
    ];
    if !penalty_amount.is_zero() {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: config.dao_treasury_address.to_string(),
                amount: penalty_amount,
            })?,
            funds: vec![],
        });
    }
    Ok(Response::new().add_messages(msgs))
}

/// increase locking time
pub fn relock(
    deps: DepsMut,
    env: Env,
    relocking_detail: RelockingDetail,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let recipient = deps.api.addr_validate(&relocking_detail.recipient)?;
    let (relock_amount, relocking) = _relock(deps, env, Uint128::zero(), relocking_detail.clone())?;

    ensure!(
        relocking_detail.from_duration <= relocking_detail.to_duration,
        ContractError::ExtendDurationErr(
            relocking_detail.from_duration,
            relocking_detail.to_duration
        )
    );
    // send stake message to reward_contract
    let relock_msg = WasmMsg::Execute {
        contract_addr: config.reward_contract.to_string(),
        msg: to_json_binary(&RewardDistributorExecuteMsg::Relock {
            from: relocking_detail.sender.clone(),
            to: recipient,
            relocking,
            adding_amount: Some(Uint128::zero()),
            from_duration: relocking_detail.from_duration,
            to_duration: relocking_detail.to_duration,
        })?,
        funds: vec![],
    };
    Ok(Response::new()
        .add_attribute("action", "extend duration")
        .add_attribute("from", relocking_detail.from_duration.to_string())
        .add_attribute("user", relocking_detail.sender.to_string())
        .add_attribute("amount", relock_amount)
        .add_attribute("to", relocking_detail.to_duration.to_string())
        .add_attribute("receiver", relocking_detail.recipient)
        .add_message(relock_msg))
}

fn _relock(
    deps: DepsMut,
    env: Env,
    adding_amount: Uint128,
    relocking_detail: RelockingDetail,
) -> Result<(Uint128, Vec<(u64, Uint128)>), ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let from_duration = relocking_detail.from_duration;
    let to_duration = relocking_detail.to_duration;
    let sender = relocking_detail.sender;
    let receiver = relocking_detail.recipient;

    let mut total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    let mut total_staking_by_duration_from = TOTAL_STAKING_BY_DURATION
        .load(deps.storage, from_duration)
        .unwrap_or_default();
    let mut total_staking_by_duration_to = TOTAL_STAKING_BY_DURATION
        .load(deps.storage, to_duration)
        .unwrap_or_default();
    let is_allowed_user = ALLOWED_USERS
        .load(deps.storage, &sender.to_string())
        .unwrap_or_default();
    ensure!(
        config
            .timelock_config
            .clone()
            .into_iter()
            .any(|i| i.duration == from_duration),
        ContractError::NoLockingPeriodFound(from_duration)
    );
    ensure!(
        config
            .timelock_config
            .into_iter()
            .any(|i| i.duration == to_duration),
        ContractError::NoLockingPeriodFound(to_duration)
    );
    let mut relock_amount = adding_amount;
    let mut relocking = vec![];
    for relock in relocking_detail.relocks {
        let user_staking_from =
            STAKING.load(deps.storage, (&sender.to_string(), from_duration, relock.0))?;
        let amount =
            if is_allowed_user && relock.1.is_some() && relock.1.unwrap().le(&user_staking_from) {
                relock.1.unwrap()
            } else {
                user_staking_from
            };
        if amount.eq(&user_staking_from) {
            STAKING.remove(deps.storage, (&sender.to_string(), from_duration, relock.0));
        } else {
            STAKING.save(
                deps.storage,
                (&sender.to_string(), from_duration, relock.0),
                &(user_staking_from - amount),
            )?;
        }
        total_staking_by_duration_from -= amount;
        relock_amount += amount;
        relocking.push((relock.0, amount));
    }
    let mut user_staking_to = STAKING
        .load(
            deps.storage,
            (&receiver, to_duration, env.block.time.seconds()),
        )
        .unwrap_or_default();
    user_staking_to += relock_amount;
    total_staking_by_duration_to += relock_amount;
    total_staking = total_staking.checked_add(adding_amount).unwrap();
    STAKING.save(
        deps.storage,
        (&receiver.to_string(), to_duration, env.block.time.seconds()),
        &user_staking_to,
    )?;
    TOTAL_STAKING.save(deps.storage, &total_staking)?;
    TOTAL_STAKING_BY_DURATION.save(deps.storage, from_duration, &total_staking_by_duration_from)?;
    TOTAL_STAKING_BY_DURATION.save(deps.storage, to_duration, &total_staking_by_duration_to)?;
    Ok((user_staking_to, relocking))
}
