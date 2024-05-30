use astroport::asset::AssetInfo;
use cosmwasm_std::{
    coin, ensure, ensure_eq, from_json, to_json_binary, BankMsg, CosmosMsg, DepsMut, Env,
    MessageInfo, Response, StdResult, Uint128, WasmMsg,
};
use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg, Cw20ReceiveMsg};
use cw_utils::one_coin;

use crate::{
    entry::query::calculate_penalty,
    error::ContractError,
    state::{
        ALLOWED_USERS, CONFIG, LAST_CLAIM_TIME, OWNER, PENDING_ECLIPASTRO_REWARDS, REWARD_WEIGHTS,
        TOTAL_STAKING, TOTAL_STAKING_BY_DURATION, USER_STAKED,
    },
};

use equinox_msg::{
    single_sided_staking::{
        CallbackMsg, Cw20HookMsg, RestakeData, UpdateConfigMsg, UserStaked, VaultRewards,
    },
    token_converter::ExecuteMsg as ConverterExecuteMsg,
};

use super::query::{
    calculate_total_user_reward, calculate_updated_reward_weights, calculate_user_reward,
    query_eclipastro_pending_rewards,
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
    if let Some(token_converter) = new_config.token_converter {
        config.token_converter = token_converter.clone();
        res = res.add_attribute("token_converter", token_converter.to_string());
    }
    if let Some(treasury) = new_config.treasury {
        config.treasury = treasury.clone();
        res = res.add_attribute("treasury", treasury.to_string());
    }
    if let Some(rewards) = new_config.rewards {
        config.rewards = rewards;
        res = res.add_attribute("rewards", "update rewards");
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
                        + ","
                        + &i.reward_multiplier.to_string()
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
        CallbackMsg::Convert {
            prev_eclipastro_balance,
            duration,
            sender,
            recipient,
        } => {
            let config = CONFIG.load(deps.storage)?;
            let eclipastro_balance: BalanceResponse = deps.querier.query_wasm_smart(
                config.token,
                &Cw20QueryMsg::Balance {
                    address: env.contract.address.to_string(),
                },
            )?;
            _stake(
                deps,
                env,
                duration,
                sender,
                recipient,
                prev_eclipastro_balance - eclipastro_balance.balance,
            )
        }
    }
}

pub fn stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    duration: u64,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    let recipient = recipient.unwrap_or(sender.clone());
    let received_asset = one_coin(&info)?;
    let config = CONFIG.load(deps.storage)?;
    let eclipastro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &config.token,
        &Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;
    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.token_converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Convert { recipient: None })?,
            funds: vec![received_asset],
        }))
        .add_message(
            CallbackMsg::Convert {
                prev_eclipastro_balance: eclipastro_balance.balance,
                duration,
                sender,
                recipient,
            }
            .to_cosmos_msg(&env)?,
        ))
}

pub fn _stake(
    mut deps: DepsMut,
    env: Env,
    lock_duration: u64,
    sender: String,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure!(
        config
            .timelock_config
            .into_iter()
            .any(|i| i.duration == lock_duration),
        ContractError::NoLockingPeriodFound(lock_duration)
    );

    let current_time = env.block.time.seconds();
    let updated_reward_weights = calculate_updated_reward_weights(deps.as_ref(), current_time)?;

    let mut response = Response::new();

    let locked_at = match lock_duration {
        0u64 => 0u64,
        _ => current_time,
    };

    let mut total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();

    let mut user_staking = USER_STAKED
        .load(deps.storage, (&recipient, lock_duration, locked_at))
        .unwrap_or(UserStaked {
            staked: Uint128::zero(),
            reward_weights: updated_reward_weights.clone(),
        });
    if !user_staking.staked.is_zero() {
        response = _claim_single(
            deps.branch(),
            env.clone(),
            sender.clone(),
            lock_duration,
            locked_at,
        )?;
    } else {
        LAST_CLAIM_TIME.save(deps.storage, &env.block.time.seconds())?;
    }

    let mut total_staking_by_duration = TOTAL_STAKING_BY_DURATION
        .load(deps.storage, lock_duration)
        .unwrap_or_default();
    user_staking.staked = user_staking.staked.checked_add(amount).unwrap();
    total_staking = total_staking.checked_add(amount).unwrap();
    total_staking_by_duration = total_staking_by_duration.checked_add(amount).unwrap();
    USER_STAKED.save(
        deps.storage,
        (&recipient.to_string(), lock_duration, locked_at),
        &user_staking,
    )?;
    TOTAL_STAKING.save(deps.storage, &total_staking)?;
    TOTAL_STAKING_BY_DURATION.save(deps.storage, lock_duration, &total_staking_by_duration)?;
    REWARD_WEIGHTS.save(deps.storage, &updated_reward_weights)?;
    Ok(response
        .add_attribute("action", "stake eclipastro")
        .add_attribute("duration", lock_duration.to_string())
        .add_attribute("amount", amount.to_string()))
}

/// Cw20 Receive hook msg handler.
pub fn receive_cw20(
    mut deps: DepsMut,
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
        Cw20HookMsg::Stake {
            lock_duration,
            recipient,
        } => {
            let sender = msg.sender;
            let recipient = recipient.unwrap_or(sender.clone());
            _stake(
                deps.branch(),
                env,
                lock_duration,
                sender,
                recipient,
                msg.amount,
            )
        }
        Cw20HookMsg::Restake {
            from_duration,
            locked_at,
            amount,
            to_duration,
            recipient,
        } => {
            let sender = msg.sender;
            let recipient = recipient.unwrap_or(sender.to_string());
            let locked_at = locked_at.unwrap_or_default();
            restake(
                deps,
                env,
                RestakeData {
                    from_duration,
                    locked_at,
                    amount,
                    to_duration,
                    add_amount: Some(msg.amount),
                    sender,
                    recipient,
                },
            )
        }
    }
}

pub fn restake(mut deps: DepsMut, env: Env, data: RestakeData) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let from_duration = data.from_duration;
    let locked_at = data.locked_at;
    let to_duration = data.to_duration;
    let sender = data.sender;
    let amount = data.amount;
    let add_amount = data.add_amount;
    let recipient = data.recipient;

    ensure!(
        from_duration <= to_duration,
        ContractError::ExtendDurationErr(from_duration, to_duration)
    );

    ensure!(
        config
            .timelock_config
            .into_iter()
            .any(|i| i.duration == to_duration),
        ContractError::NoLockingPeriodFound(to_duration)
    );

    let user_staking_from = USER_STAKED.load(deps.storage, (&sender, from_duration, locked_at))?;
    ensure!(
        !user_staking_from.staked.is_zero(),
        ContractError::NoLockedAmount {}
    );

    let mut restake_amount = user_staking_from.staked;

    if let Some(amount) = amount {
        let is_allowed_user = ALLOWED_USERS
            .load(deps.storage, &sender.to_string())
            .unwrap_or_default();
        if is_allowed_user {
            ensure!(
                !user_staking_from.staked >= amount,
                ContractError::ExceedAmount {}
            );
            restake_amount = amount;
        }
    }

    let current_time = env.block.time.seconds();
    let updated_reward_weights = calculate_updated_reward_weights(deps.as_ref(), current_time)?;
    let response = _claim_single(
        deps.branch(),
        env.clone(),
        sender.to_string(),
        from_duration,
        locked_at,
    )?;

    let mut user_staking_to = UserStaked {
        staked: Uint128::zero(),
        reward_weights: updated_reward_weights.clone(),
    };

    let mut total_staking_by_duration_from = TOTAL_STAKING_BY_DURATION
        .load(deps.storage, from_duration)
        .unwrap_or_default();
    let mut total_staking_by_duration_to = TOTAL_STAKING_BY_DURATION
        .load(deps.storage, to_duration)
        .unwrap_or_default();

    if restake_amount.eq(&user_staking_from.staked) {
        USER_STAKED.remove(
            deps.storage,
            (&sender.to_string(), from_duration, locked_at),
        );
    } else {
        USER_STAKED.save(
            deps.storage,
            (&sender.to_string(), from_duration, locked_at),
            &UserStaked {
                staked: user_staking_from.staked - restake_amount,
                reward_weights: updated_reward_weights.clone(),
            },
        )?;
    }
    user_staking_to.staked =
        user_staking_to.staked + restake_amount + add_amount.unwrap_or_default();
    if add_amount.is_some() {
        let mut total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
        total_staking += add_amount.unwrap();
        TOTAL_STAKING.save(deps.storage, &total_staking)?;
    }
    total_staking_by_duration_from -= restake_amount;
    total_staking_by_duration_to += restake_amount + add_amount.unwrap_or_default();
    USER_STAKED.save(
        deps.storage,
        (&recipient, to_duration, current_time),
        &user_staking_to,
    )?;
    TOTAL_STAKING_BY_DURATION.save(deps.storage, from_duration, &total_staking_by_duration_from)?;
    TOTAL_STAKING_BY_DURATION.save(deps.storage, to_duration, &total_staking_by_duration_to)?;
    REWARD_WEIGHTS.save(deps.storage, &updated_reward_weights)?;

    Ok(response
        .add_attribute("action", "add lock")
        .add_attribute("user", sender.to_string())
        .add_attribute("amount", add_amount.unwrap_or_default().to_string())
        .add_attribute("action", "extend duration")
        .add_attribute("from", from_duration.to_string())
        .add_attribute("user", sender.to_string())
        .add_attribute("amount", restake_amount)
        .add_attribute("to", to_duration.to_string())
        .add_attribute("receiver", recipient.to_string()))
}

pub fn _claim_single(
    deps: DepsMut,
    env: Env,
    sender: String,
    duration: u64,
    locked_at: u64,
) -> Result<Response, ContractError> {
    let current_time = env.block.time.seconds();
    let mut user_staking = USER_STAKED.load(deps.storage, (&sender, duration, locked_at))?;
    let updated_reward_weights = calculate_updated_reward_weights(deps.as_ref(), current_time)?;
    let user_reward = calculate_user_reward(
        deps.as_ref(),
        sender.clone(),
        duration,
        locked_at,
        current_time,
    )?;

    user_staking.reward_weights = updated_reward_weights.clone();
    REWARD_WEIGHTS.save(deps.storage, &updated_reward_weights)?;
    USER_STAKED.save(deps.storage, (&sender, duration, locked_at), &user_staking)?;
    LAST_CLAIM_TIME.save(deps.storage, &env.block.time.seconds())?;
    _claim(
        deps,
        env,
        sender,
        user_reward.eclipastro,
        VaultRewards {
            eclip: user_reward.eclip,
            beclip: user_reward.beclip,
        },
    )
}

pub fn _claim(
    deps: DepsMut,
    env: Env,
    sender: String,
    eclipastro: Uint128,
    vault_rewards: VaultRewards,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let pending_eclipastro_rewards =
        query_eclipastro_pending_rewards(deps.as_ref(), config.token_converter.to_string())?;

    let mut response = Response::new().add_attribute("action", "claim rewards");
    let mut msgs = vec![];

    if !pending_eclipastro_rewards.is_zero() {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.token_converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Claim {})?,
            funds: vec![],
        }));
        PENDING_ECLIPASTRO_REWARDS.save(
            deps.storage,
            env.block.time.seconds(),
            &pending_eclipastro_rewards,
        )?;
        response = response
            .add_attribute("action", "claim eclipastro rewards")
            .add_attribute("amount", pending_eclipastro_rewards.to_string());
    }

    if !eclipastro.is_zero() {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender.clone(),
                amount: eclipastro,
            })?,
            funds: vec![],
        }));
        response = response
            .add_attribute("action", "claim user eclipastro reward")
            .add_attribute("amount", eclipastro.to_string());
    }

    if !vault_rewards.beclip.is_zero() {
        match config.rewards.beclip.info {
            AssetInfo::Token { contract_addr } => {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: sender.clone(),
                        amount: vault_rewards.beclip,
                    })?,
                    funds: vec![],
                }));
            }
            AssetInfo::NativeToken { denom } => {
                msgs.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: sender.clone(),
                    amount: [coin(vault_rewards.beclip.u128(), denom)].to_vec(),
                }));
            }
        }
        response = response
            .add_attribute("action", "claim user beclip reward")
            .add_attribute("amount", vault_rewards.beclip.to_string());
    }

    if !vault_rewards.eclip.is_zero() {
        match config.rewards.eclip.info {
            AssetInfo::Token { contract_addr } => {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: sender.clone(),
                        amount: vault_rewards.eclip,
                    })?,
                    funds: vec![],
                }));
            }
            AssetInfo::NativeToken { denom } => {
                msgs.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: sender.clone(),
                    amount: [coin(vault_rewards.eclip.u128(), denom)].to_vec(),
                }));
            }
        }
        response = response
            .add_attribute("action", "claim user eclip reward")
            .add_attribute("amount", vault_rewards.eclip.to_string());
    }

    Ok(response.add_messages(msgs))
}

pub fn _claim_all(
    deps: DepsMut,
    env: Env,
    sender: String,
    with_flexible: bool,
) -> Result<Response, ContractError> {
    let current_time = env.block.time.seconds();
    let updated_reward_weights = calculate_updated_reward_weights(deps.as_ref(), current_time)?;
    let total_user_reward =
        calculate_total_user_reward(deps.as_ref(), sender.clone(), current_time)?;
    let mut total_eclipastro_reward = Uint128::zero();
    let mut total_beclip_reward = Uint128::zero();
    let mut total_eclip_reward = Uint128::zero();

    for reward_duration in total_user_reward {
        if !with_flexible {
            continue;
        }
        let duration = reward_duration.duration;
        for reward_locked_at in reward_duration.rewards {
            let locked_at = reward_locked_at.locked_at;
            total_eclipastro_reward += reward_locked_at.rewards.eclipastro;
            total_beclip_reward += reward_locked_at.rewards.beclip;
            total_eclip_reward += reward_locked_at.rewards.eclip;
            USER_STAKED.update(
                deps.storage,
                (&sender, duration, locked_at),
                |user_staking| -> StdResult<_> {
                    let mut user_staking = user_staking.unwrap();
                    user_staking.reward_weights = updated_reward_weights.clone();
                    Ok(user_staking)
                },
            )?;
        }
    }

    REWARD_WEIGHTS.save(deps.storage, &updated_reward_weights)?;
    LAST_CLAIM_TIME.save(deps.storage, &env.block.time.seconds())?;

    _claim(
        deps,
        env,
        sender,
        total_eclipastro_reward,
        VaultRewards {
            eclip: total_eclip_reward,
            beclip: total_beclip_reward,
        },
    )
}

/// Claim user rewards
pub fn claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    duration: u64,
    locked_at: Option<u64>,
) -> Result<Response, ContractError> {
    let locked_at = locked_at.unwrap_or_default();
    _claim_single(deps, env, info.sender.to_string(), duration, locked_at)
}

pub fn claim_all(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    with_flexible: bool,
) -> Result<Response, ContractError> {
    _claim_all(deps, env, info.sender.to_string(), with_flexible)
}

/// Unlock amount and claim rewards of user
pub fn unstake(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    duration: u64,
    locked_at: Option<u64>,
    amount: Option<Uint128>,
    receiver: Option<String>,
) -> Result<Response, ContractError> {
    let locked_at = locked_at.unwrap_or_default();
    let sender = info.sender.to_string();
    let mut response = _claim_single(
        deps.branch(),
        env.clone(),
        sender.clone(),
        duration,
        locked_at,
    )?;

    let config = CONFIG.load(deps.storage)?;
    let mut user_staking = USER_STAKED.load(deps.storage, (&sender, duration, locked_at))?;
    let unlock_amount = amount.unwrap_or(user_staking.staked);
    ensure!(
        user_staking.staked >= unlock_amount,
        ContractError::ExceedAmount {}
    );
    ensure!(
        unlock_amount.gt(&Uint128::zero()),
        ContractError::NoLockedAmount {}
    );
    let receiver = receiver.unwrap_or(info.sender.to_string());

    let mut total_staking = TOTAL_STAKING.load(deps.storage)?;
    let mut total_staking_by_duration = TOTAL_STAKING_BY_DURATION.load(deps.storage, duration)?;
    total_staking = total_staking.checked_sub(unlock_amount).unwrap();
    total_staking_by_duration = total_staking_by_duration
        .checked_sub(unlock_amount)
        .unwrap();

    if unlock_amount == user_staking.staked {
        USER_STAKED.remove(deps.storage, (&sender, duration, locked_at));
    } else {
        user_staking.staked -= unlock_amount;
        USER_STAKED.save(deps.storage, (&sender, duration, locked_at), &user_staking)?;
    }
    TOTAL_STAKING.save(deps.storage, &total_staking)?;
    TOTAL_STAKING_BY_DURATION.save(deps.storage, duration, &total_staking_by_duration)?;

    let penalty_amount = calculate_penalty(deps.as_ref(), env, unlock_amount, duration, locked_at)?;

    let mut msgs = vec![WasmMsg::Execute {
        contract_addr: config.token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: receiver,
            amount: unlock_amount.checked_sub(penalty_amount).unwrap(),
        })?,
        funds: vec![],
    }];
    if !penalty_amount.is_zero() {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: config.treasury.to_string(),
                amount: penalty_amount,
            })?,
            funds: vec![],
        });
    }
    response = response
        .add_attribute("action", "unstake")
        .add_attribute("amount", unlock_amount.to_string())
        .add_attribute("penalty", penalty_amount.to_string())
        .add_attribute("duration", duration.to_string());

    if duration > 0u64 {
        response = response.add_attribute("locked_at", locked_at.to_string());
    }
    Ok(response.add_messages(msgs))
}
