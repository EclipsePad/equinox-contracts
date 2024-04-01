use cosmwasm_std::{
    ensure, ensure_eq, from_json, to_json_binary, DepsMut, Env, MessageInfo, Response, Uint128,
    WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::{
    entry::query::calculate_penalty,
    error::ContractError,
    state::{CONFIG, OWNER, PENALTIES, STAKING, TOTAL_STAKING, TOTAL_STAKING_BY_DURATION},
};

use equinox_msg::{
    reward_distributor::ExecuteMsg as RewardDistributorExecuteMsg,
    timelock_staking::{Cw20HookMsg, RestakingDetail, UpdateConfigMsg},
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
    match from_json(&msg.msg)? {
        // lock eclipASTRO token with duration
        // check amount, duration
        Cw20HookMsg::Lock { duration, user } => {
            let config = CONFIG.load(deps.storage)?;
            let user_address = user.unwrap_or(deps.api.addr_validate(&msg.sender)?);
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
                    (
                        &user_address.to_string(),
                        duration,
                        env.block.time.seconds(),
                    ),
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
                (
                    &user_address.to_string(),
                    duration,
                    env.block.time.seconds(),
                ),
                &user_staking,
            )?;
            TOTAL_STAKING.save(deps.storage, &total_staking)?;
            TOTAL_STAKING_BY_DURATION.save(deps.storage, duration, &total_staking_by_duration)?;

            // send stake message to reward_contract
            let stake_msg = WasmMsg::Execute {
                contract_addr: config.reward_contract.to_string(),
                msg: to_json_binary(&RewardDistributorExecuteMsg::TimelockStake {
                    user: user_address.to_string(),
                    amount: msg.amount,
                    duration,
                })?,
                funds: vec![],
            };
            Ok(Response::new().add_message(stake_msg))
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
pub fn unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    duration: u64,
    locked_at: u64,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    let config = CONFIG.load(deps.storage)?;
    let unlock_max_amount = STAKING.load(deps.storage, (&sender, duration, locked_at))?;
    let unlock_amount = amount.unwrap_or(unlock_max_amount);
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
    if penalty_amount.gt(&Uint128::zero()) {
        let mut penalties = PENALTIES.load(deps.storage).unwrap_or_default();
        penalties = penalties.checked_add(penalties).unwrap();
        PENALTIES.save(deps.storage, &penalties)?;
    }

    let msg = vec![
        WasmMsg::Execute {
            contract_addr: config.token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
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
    Ok(Response::new().add_messages(msg))
}

/// increase locking time
pub fn restake(
    deps: DepsMut,
    env: Env,
    restaking_detail: RestakingDetail,
) -> Result<Response, ContractError> {
    let sender = restaking_detail.sender;
    let from = restaking_detail.from_duration;
    let to = restaking_detail.to_duration;
    let locked_at = restaking_detail.locked_at;
    let config = CONFIG.load(deps.storage)?;
    let current_time = env.block.time.seconds();
    let new_owner = restaking_detail.receiver.clone().unwrap_or(sender.clone());

    let user_staked_amount = STAKING.load(deps.storage, (&sender.to_string(), from, locked_at))?;
    let restaking_amount = restaking_detail.amount.unwrap_or(user_staked_amount);

    ensure!(
        restaking_amount.gt(&Uint128::zero()),
        ContractError::NoLockedAmount {}
    );

    ensure!(
        restaking_amount.le(&user_staked_amount),
        ContractError::ExceedAmount {}
    );

    if restaking_amount == user_staked_amount {
        STAKING.remove(deps.storage, (&sender.to_string(), from, locked_at));
    } else {
        STAKING.save(
            deps.storage,
            (&sender.to_string(), from, locked_at),
            &(user_staked_amount - restaking_amount),
        )?;
    }
    let mut total_staking_by_duration = TOTAL_STAKING_BY_DURATION.load(deps.storage, from)?;
    total_staking_by_duration = total_staking_by_duration
        .checked_sub(restaking_amount)
        .unwrap();
    TOTAL_STAKING_BY_DURATION.save(deps.storage, from, &total_staking_by_duration)?;
    let mut total_staking_by_duration = TOTAL_STAKING_BY_DURATION
        .load(deps.storage, to)
        .unwrap_or_default();
    total_staking_by_duration = total_staking_by_duration
        .checked_add(restaking_amount)
        .unwrap();
    TOTAL_STAKING_BY_DURATION.save(deps.storage, to, &total_staking_by_duration)?;

    let msg = WasmMsg::Execute {
        contract_addr: config.reward_contract.to_string(),
        msg: to_json_binary(&RewardDistributorExecuteMsg::Restake {
            user: sender.to_string(),
            from,
            locked_at,
            to,
            receiver: restaking_detail.receiver,
            amount: restaking_amount,
        })?,
        funds: vec![],
    };
    let mut to_amount: Uint128 = STAKING
        .load(deps.storage, (&new_owner.to_string(), to, current_time))
        .unwrap_or_default();
    to_amount = to_amount.checked_add(restaking_amount).unwrap();
    STAKING.save(
        deps.storage,
        (&new_owner.to_string(), to, current_time),
        &to_amount,
    )?;
    Ok(Response::new().add_message(msg))
}
