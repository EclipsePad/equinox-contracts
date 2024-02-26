use std::vec;

use cosmwasm_std::{
    coins, ensure, to_json_binary, BankMsg, DepsMut, Env, MessageInfo, Order, Response, StdResult,
    Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use equinox_msg::{
    reward_distributor::UpdateConfigMsg,
    token_converter::{
        ExecuteMsg as ConverterExecuteMsg, QueryMsg as ConverterQueryMsg, RewardResponse,
    },
};

use crate::{
    entry::query::{total_staking_amount_update, total_staking_reward_update, user_reward_update},
    error::ContractError,
    state::{
        CONFIG, FLEXIBLE_USER_STAKING, LAST_UPDATE_TIME, OWNER, TIMELOCK_USER_STAKING,
        TOTAL_STAKING,
    },
};

/// Update config
pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_config: UpdateConfigMsg,
) -> Result<Response, ContractError> {
    // Only owner can execute
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let mut config = CONFIG.load(deps.storage)?;
    let mut res: Response = Response::new().add_attribute("action", "update config");
    if let Some(eclipastro) = new_config.eclipastro {
        config.eclipastro = deps.api.addr_validate(&eclipastro)?;
        res = res.add_attribute("eclipastro", eclipastro);
    }
    if let Some(eclip) = new_config.eclip {
        config.eclip = eclip.clone();
        res = res.add_attribute("eclip", eclip);
    }
    if let Some(flexible_staking) = new_config.flexible_staking {
        config.flexible_staking = deps.api.addr_validate(&flexible_staking)?;
        res = res.add_attribute("flexible_staking", flexible_staking);
    }
    if let Some(timelock_staking) = new_config.timelock_staking {
        config.timelock_staking = deps.api.addr_validate(&timelock_staking)?;
        res = res.add_attribute("timelock_staking", timelock_staking);
    }
    if let Some(token_converter) = new_config.token_converter {
        config.token_converter = deps.api.addr_validate(&token_converter)?;
        res = res.add_attribute("token_converter", token_converter);
    }
    if let Some(eclip_daily_reward) = new_config.eclip_daily_reward {
        config.eclip_daily_reward = eclip_daily_reward;
        res = res.add_attribute("eclip_daily_reward", eclip_daily_reward);
    }
    if let Some(locking_reward_config) = new_config.locking_reward_config {
        config.locking_reward_config = locking_reward_config.clone();
        res = res.add_attribute(
            "locking_reward_config",
            locking_reward_config
                .into_iter()
                .map(|i| {
                    "(".to_string()
                        + &i.duration.to_string()
                        + ","
                        + &i.multiplier.to_string()
                        + ")"
                })
                .collect::<Vec<String>>()
                .join(","),
        );
    }
    CONFIG.save(deps.storage, &config)?;
    Ok(res)
}

/// Update owner
pub fn update_owner(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response, ContractError> {
    // Only owner can execute
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let new_owner_addr = deps.api.addr_validate(&new_owner)?;
    OWNER.set(deps.branch(), Some(new_owner_addr))?;
    Ok(Response::new()
        .add_attribute("action", "update owner")
        .add_attribute("to", new_owner))
}

/// flexible staking event
pub fn flexible_stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // only flexible staking contract can execute this function
    ensure!(
        info.sender == config.flexible_staking,
        ContractError::Unauthorized {}
    );
    // calculate xASTRO rewards
    let pending_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    // update total staking reward
    let mut total_staking_data =
        total_staking_reward_update(deps.as_ref(), env.clone(), &pending_reward)?;
    // update total staking balance
    total_staking_data = total_staking_amount_update(total_staking_data, 0u64, amount, true)?;
    // get user flexible staking data
    let mut user_staking = FLEXIBLE_USER_STAKING
        .load(deps.storage, &user)
        .unwrap_or_default();
    // update user staking reward
    user_staking = user_reward_update(deps.as_ref(), &total_staking_data, 0u64, &user_staking)?;
    // update user staking balance
    user_staking.amount = user_staking.amount.checked_add(amount.clone()).unwrap();
    TOTAL_STAKING.save(deps.storage, &total_staking_data)?;
    // update last update time as current block time
    LAST_UPDATE_TIME.save(deps.storage, &env.block.time.seconds())?;
    FLEXIBLE_USER_STAKING.save(deps.storage, &user, &user_staking)?;
    // claim eclipastro rewards if exists
    if pending_reward.users_reward.amount.gt(&Uint128::zero()) {
        return Ok(Response::new().add_message(WasmMsg::Execute {
            contract_addr: config.token_converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Claim {})?,
            funds: vec![],
        }));
    }
    Ok(Response::new())
}

/// timelock staking event
pub fn timelock_stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    amount: Uint128,
    duration: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // only timelock staking contract can execute this function
    ensure!(
        info.sender == config.timelock_staking,
        ContractError::Unauthorized {}
    );
    // calculate xASTRO rewards
    let pending_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    let current_time = env.block.time.seconds();
    // update total staking reward
    let mut total_staking_data =
        total_staking_reward_update(deps.as_ref(), env.clone(), &pending_reward)?;
    // update total staking balance
    total_staking_data = total_staking_amount_update(total_staking_data, duration, amount, true)?;
    // get user staking data
    let mut user_staking = TIMELOCK_USER_STAKING
        .load(deps.storage, (&user, duration, current_time))
        .unwrap_or_default();
    // update user staking reward
    user_staking = user_reward_update(deps.as_ref(), &total_staking_data, duration, &user_staking)?;
    // update user staking balance
    user_staking.amount = user_staking.amount.checked_add(amount).unwrap();
    TOTAL_STAKING.save(deps.storage, &total_staking_data)?;
    // update last update time as current block time
    LAST_UPDATE_TIME.save(deps.storage, &current_time)?;
    TIMELOCK_USER_STAKING.save(
        deps.storage,
        (&user, duration, env.block.time.seconds()),
        &user_staking,
    )?;
    // claim eclipastro rewards if exists
    if pending_reward.users_reward.amount.gt(&Uint128::zero()) {
        return Ok(Response::new().add_message(WasmMsg::Execute {
            contract_addr: config.token_converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Claim {})?,
            funds: vec![],
        }));
    }
    Ok(Response::new())
}

/// claim rewards
pub fn flexible_stake_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // only flexible staking contract can execute this function
    ensure!(
        info.sender == config.flexible_staking,
        ContractError::Unauthorized {}
    );
    // calculate xASTRO rewards
    let pending_eclipastro_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    // update total staking rewards
    let total_staking_data =
        total_staking_reward_update(deps.as_ref(), env.clone(), &pending_eclipastro_reward)?;
    // update user staking rewards
    let mut user_staking = FLEXIBLE_USER_STAKING.load(deps.storage, &user)?;
    user_staking = user_reward_update(deps.as_ref(), &total_staking_data, 0u64, &user_staking)?;
    let mut msgs = vec![];
    // if there is pending eclipASTRO rewards from xASTRO rewards, claim it
    if pending_eclipastro_reward
        .users_reward
        .amount
        .gt(&Uint128::zero())
    {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.token_converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Claim {})?,
            funds: vec![],
        });
    }
    let mut res = Response::new();
    // if there is user's eclipASTRO rewards, send it to user
    if user_staking
        .rewards
        .eclipastro
        .pending_reward
        .gt(&Uint128::zero())
    {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.eclipastro.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: user.clone(),
                amount: user_staking.rewards.eclipastro.pending_reward,
            })?,
            funds: vec![],
        });
        res = res
            .add_attribute("recipient", user.clone())
            .add_attribute("asset", config.eclipastro.to_string())
            .add_attribute(
                "amount",
                user_staking.rewards.eclipastro.pending_reward.to_string(),
            );
    }
    // if there is user's ECLIP rewards, send it to user
    let mut bankmsgs = vec![];
    if user_staking
        .rewards
        .eclip
        .pending_reward
        .gt(&Uint128::zero())
    {
        bankmsgs.push(BankMsg::Send {
            to_address: user.clone(),
            amount: coins(
                user_staking.rewards.eclip.pending_reward.u128(),
                config.eclip.clone(),
            ),
        });
        res = res
            .add_attribute("recipient", user.clone())
            .add_attribute("asset", config.eclip.clone().to_string())
            .add_attribute(
                "amount",
                user_staking.rewards.eclip.pending_reward.to_string(),
            );
    }
    // update user's rewards details
    user_staking.rewards.eclip.pending_reward = Uint128::zero();
    user_staking.rewards.eclipastro.pending_reward = Uint128::zero();
    TOTAL_STAKING.save(deps.storage, &total_staking_data)?;
    LAST_UPDATE_TIME.save(deps.storage, &env.block.time.seconds())?;
    FLEXIBLE_USER_STAKING.save(deps.storage, &user, &user_staking)?;
    // claim rewards
    Ok(res.add_messages(msgs).add_messages(bankmsgs))
}

/// claim timelock rewards
pub fn timelock_stake_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    duration: u64,
    locked_at: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // only timelock staking contracts can execute this function
    ensure!(
        info.sender == config.timelock_staking,
        ContractError::Unauthorized {}
    );
    // calculate xASTRO rewards
    let pending_eclipastro_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    // update total staking rewards
    let total_staking_data =
        total_staking_reward_update(deps.as_ref(), env.clone(), &pending_eclipastro_reward)?;
    let mut user_staking =
        TIMELOCK_USER_STAKING.load(deps.storage, (&user, duration, locked_at))?;
    // update user staking rewards
    user_staking = user_reward_update(deps.as_ref(), &total_staking_data, duration, &user_staking)?;
    let mut msgs = vec![];
    // if there is pending eclipASTRO rewards from xASTRO rewards, claim it
    if pending_eclipastro_reward
        .users_reward
        .amount
        .gt(&Uint128::zero())
    {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.token_converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Claim {})?,
            funds: vec![],
        });
    }
    let mut res = Response::new();
    // if there is user's eclipASTRO rewards, send it to user
    if user_staking
        .rewards
        .eclipastro
        .pending_reward
        .gt(&Uint128::zero())
    {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.eclipastro.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: user.clone(),
                amount: user_staking.rewards.eclipastro.pending_reward,
            })?,
            funds: vec![],
        });
        res = res
            .add_attribute("recipient", user.clone())
            .add_attribute("asset", config.eclipastro.to_string())
            .add_attribute(
                "amount",
                user_staking.rewards.eclipastro.pending_reward.to_string(),
            );
    }
    // if there is user's ECLIP rewards, send it to user
    let mut bankmsgs = vec![];
    if user_staking
        .rewards
        .eclip
        .pending_reward
        .gt(&Uint128::zero())
    {
        bankmsgs.push(BankMsg::Send {
            to_address: user.clone(),
            amount: coins(
                user_staking.rewards.eclip.pending_reward.u128(),
                config.eclip.clone(),
            ),
        });
        res = res
            .add_attribute("recipient", user.clone())
            .add_attribute("asset", config.eclip.clone().to_string())
            .add_attribute(
                "amount",
                user_staking.rewards.eclip.pending_reward.to_string(),
            );
    }
    // update user's rewards details
    user_staking.rewards.eclip.pending_reward = Uint128::zero();
    user_staking.rewards.eclipastro.pending_reward = Uint128::zero();
    TOTAL_STAKING.save(deps.storage, &total_staking_data)?;
    LAST_UPDATE_TIME.save(deps.storage, &env.block.time.seconds())?;
    TIMELOCK_USER_STAKING.save(deps.storage, (&user, duration, locked_at), &user_staking)?;
    // claim rewards
    Ok(res.add_messages(msgs).add_messages(bankmsgs))
}

/// claim all timelock rewards
pub fn timelock_stake_claim_all(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // only timelock staking contract can execute this function
    ensure!(
        info.sender == config.timelock_staking,
        ContractError::Unauthorized {}
    );
    // calculate xASTRO rewards
    let pending_eclipastro_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    // update total staking rewards
    let total_staking_data =
        total_staking_reward_update(deps.as_ref(), env.clone(), &pending_eclipastro_reward)?;
    let mut msgs = vec![];
    // if there is pending eclipASTRO rewards from xASTRO rewards, claim it
    if pending_eclipastro_reward
        .users_reward
        .amount
        .gt(&Uint128::zero())
    {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.token_converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Claim {})?,
            funds: vec![],
        });
    }
    let mut res = Response::new();
    let mut flexible_reward = Uint128::zero();
    let mut timelock_reward = Uint128::zero();
    let user_staking_prefixes = TIMELOCK_USER_STAKING
        .sub_prefix(&user)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|s| {
            let ((duration, locked_at), mut staking_data) = s.unwrap();
            staking_data =
                user_reward_update(deps.as_ref(), &total_staking_data, duration, &staking_data)
                    .unwrap();
            flexible_reward = flexible_reward
                .checked_add(staking_data.rewards.eclipastro.pending_reward)
                .unwrap();
            timelock_reward = timelock_reward
                .checked_add(staking_data.rewards.eclip.pending_reward)
                .unwrap();
            Ok((duration, locked_at))
        })
        .collect::<StdResult<Vec<(u64, u64)>>>()
        .unwrap();
    // if there is user's eclipASTRO rewards, send it to user
    if flexible_reward.gt(&Uint128::zero()) {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.eclipastro.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: user.clone(),
                amount: flexible_reward,
            })?,
            funds: vec![],
        });
        res = res
            .add_attribute("recipient", user.clone())
            .add_attribute("asset", config.eclipastro.to_string())
            .add_attribute("amount", flexible_reward.to_string());
    }
    // if there is user's ECLIP rewards, send it to user
    let mut bankmsgs = vec![];
    if timelock_reward.gt(&Uint128::zero()) {
        bankmsgs.push(BankMsg::Send {
            to_address: user.clone(),
            amount: coins(timelock_reward.u128(), config.eclip.clone()),
        });
        res = res
            .add_attribute("recipient", user.clone())
            .add_attribute("asset", config.eclip.clone().to_string())
            .add_attribute("amount", timelock_reward.to_string());
    }
    // update user's rewards details
    TOTAL_STAKING.save(deps.storage, &total_staking_data)?;
    LAST_UPDATE_TIME.save(deps.storage, &env.block.time.seconds())?;
    for (duration, locked_at) in user_staking_prefixes {
        let mut staking_data = TIMELOCK_USER_STAKING
            .load(deps.storage, (&user, duration, locked_at))
            .unwrap();
        staking_data.rewards.eclip.reward_weight = total_staking_data.reward_weight_eclip;
        staking_data.rewards.eclip.pending_reward = Uint128::zero();
        staking_data.rewards.eclipastro.reward_weight = total_staking_data.reward_weight_eclipastro;
        staking_data.rewards.eclipastro.pending_reward = Uint128::zero();
        TIMELOCK_USER_STAKING
            .save(deps.storage, (&user, duration, locked_at), &staking_data)
            .unwrap();
    }
    Ok(res.add_messages(msgs).add_messages(bankmsgs))
}

/// flexible unstaking event
pub fn flexible_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // only flexible staking contract can call this function
    ensure!(
        info.sender == config.flexible_staking,
        ContractError::Unauthorized {}
    );
    // calculate xASTRO reward
    let pending_eclipastro_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    // update total staking reward
    let mut total_staking_data =
        total_staking_reward_update(deps.as_ref(), env.clone(), &pending_eclipastro_reward)?;
    // update total staking balance
    total_staking_data = total_staking_amount_update(total_staking_data, 0u64, amount, false)?;
    let mut user_staking = FLEXIBLE_USER_STAKING.load(deps.storage, &user)?;
    user_staking = user_reward_update(deps.as_ref(), &total_staking_data, 0u64, &user_staking)?;
    user_staking.amount = user_staking.amount.checked_sub(amount).unwrap();
    let mut msgs = vec![];
    // if there is xASTRO rewards, claim it
    if pending_eclipastro_reward
        .users_reward
        .amount
        .gt(&Uint128::zero())
    {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.token_converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Claim {})?,
            funds: vec![],
        });
    }
    // if there is user's eclipASTRO rewards, send it
    if user_staking
        .rewards
        .eclipastro
        .pending_reward
        .gt(&Uint128::zero())
    {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.eclipastro.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: user.clone(),
                amount: user_staking.rewards.eclipastro.pending_reward,
            })?,
            funds: vec![],
        });
    }
    // if there is user's eclip rewards, send it
    let mut bankmsgs = vec![];
    if user_staking
        .rewards
        .eclip
        .pending_reward
        .gt(&Uint128::zero())
    {
        bankmsgs.push(BankMsg::Send {
            to_address: user.clone(),
            amount: coins(
                user_staking.rewards.eclip.pending_reward.u128(),
                config.eclip,
            ),
        })
    }
    // update user's rewards data
    user_staking.rewards.eclip.pending_reward = Uint128::zero();
    user_staking.rewards.eclipastro.pending_reward = Uint128::zero();
    TOTAL_STAKING.save(deps.storage, &total_staking_data)?;
    // update last update time to current block time
    LAST_UPDATE_TIME.save(deps.storage, &env.block.time.seconds())?;
    FLEXIBLE_USER_STAKING.save(deps.storage, &user, &user_staking)?;
    Ok(Response::new().add_messages(msgs).add_messages(bankmsgs))
}

/// timelock unstaking event
pub fn timelock_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    duration: u64,
    locked_at: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // only timelock staking contract can call this function
    ensure!(
        info.sender == config.timelock_staking,
        ContractError::Unauthorized {}
    );
    // calculate xASTRO reward
    let pending_eclipastro_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    // update total staking reward
    let mut total_staking_data =
        total_staking_reward_update(deps.as_ref(), env.clone(), &pending_eclipastro_reward)?;
    let mut user_staking =
        TIMELOCK_USER_STAKING.load(deps.storage, (&user, duration, locked_at))?;
    // update total staking balance
    total_staking_data = total_staking_amount_update(
        total_staking_data,
        duration,
        user_staking.amount.clone(),
        false,
    )?;
    // update user reward data
    user_staking = user_reward_update(deps.as_ref(), &total_staking_data, duration, &user_staking)?;
    let mut msgs = vec![];
    // if there is xASTRO rewards, claim it
    if pending_eclipastro_reward
        .users_reward
        .amount
        .gt(&Uint128::zero())
    {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.token_converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Claim {})?,
            funds: vec![],
        });
    }
    // if there is user's eclipASTRO rewards, send it
    if user_staking
        .rewards
        .eclipastro
        .pending_reward
        .gt(&Uint128::zero())
    {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.eclipastro.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: user.clone(),
                amount: user_staking.rewards.eclipastro.pending_reward,
            })?,
            funds: vec![],
        });
    }
    // if there is user's eclip rewards, send it
    let mut bankmsgs = vec![];
    if user_staking
        .rewards
        .eclip
        .pending_reward
        .gt(&Uint128::zero())
    {
        bankmsgs.push(BankMsg::Send {
            to_address: user.clone(),
            amount: coins(
                user_staking.rewards.eclip.pending_reward.u128(),
                config.eclip,
            ),
        })
    }
    TOTAL_STAKING.save(deps.storage, &total_staking_data)?;
    // update last update time to current block time
    LAST_UPDATE_TIME.save(deps.storage, &env.block.time.seconds())?;
    TIMELOCK_USER_STAKING.remove(deps.storage, (&user, duration, locked_at));
    Ok(Response::new().add_messages(msgs).add_messages(bankmsgs))
}

/// restaking event
pub fn restake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    from_duration: u64,
    mut locked_at: u64,
    to_duration: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // only timelock staking contract can execute it
    ensure!(
        info.sender == config.timelock_staking,
        ContractError::Unauthorized {}
    );
    // calculate pending xASTRO rewards
    let pending_eclipastro_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    // get user staking data
    let mut old_user_staking =
        TIMELOCK_USER_STAKING.load(deps.storage, (&user, from_duration, locked_at))?;
    let mut new_user_staking = TIMELOCK_USER_STAKING
        .load(deps.storage, (&user, to_duration, locked_at))
        .unwrap_or_default();
    // update total staking rewards info
    let mut total_staking_data =
        total_staking_reward_update(deps.as_ref(), env.clone(), &pending_eclipastro_reward)?;
    // update total staking balance from duration
    total_staking_data = total_staking_amount_update(
        total_staking_data,
        from_duration,
        old_user_staking.amount,
        false,
    )?;
    // update total staking balance to duration
    total_staking_data = total_staking_amount_update(
        total_staking_data,
        to_duration,
        old_user_staking.amount,
        true,
    )?;
    // update user rewards
    old_user_staking = user_reward_update(
        deps.as_ref(),
        &total_staking_data,
        from_duration,
        &old_user_staking,
    )?;
    new_user_staking = user_reward_update(
        deps.as_ref(),
        &total_staking_data,
        to_duration,
        &new_user_staking,
    )?;
    let mut msgs = vec![];
    let mut bankmsgs = vec![];
    // claim pending xASTRO rewards
    if pending_eclipastro_reward
        .users_reward
        .amount
        .gt(&Uint128::zero())
    {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.token_converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Claim {})?,
            funds: vec![],
        });
        // send user eclipASTRO rewards
        if old_user_staking
            .rewards
            .eclipastro
            .pending_reward
            .gt(&Uint128::zero())
        {
            msgs.push(WasmMsg::Execute {
                contract_addr: config.eclipastro.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: user.clone(),
                    amount: old_user_staking.rewards.eclipastro.pending_reward,
                })?,
                funds: vec![],
            });
        }
        // send user ECLIP rewards
        if old_user_staking
            .rewards
            .eclip
            .pending_reward
            .gt(&Uint128::zero())
        {
            bankmsgs.push(BankMsg::Send {
                to_address: user.clone(),
                amount: coins(
                    old_user_staking.rewards.eclip.pending_reward.u128(),
                    config.eclip,
                ),
            })
        }
    }
    TIMELOCK_USER_STAKING.remove(deps.storage, (&user, from_duration, locked_at));

    new_user_staking.amount = new_user_staking
        .amount
        .checked_add(old_user_staking.amount)
        .unwrap();
    if env.block.time.seconds() - locked_at > from_duration {
        locked_at = env.block.time.seconds() - from_duration;
    }
    TIMELOCK_USER_STAKING.save(
        deps.storage,
        (&user, to_duration, locked_at),
        &new_user_staking,
    )?;
    TOTAL_STAKING.save(deps.storage, &total_staking_data)?;
    // update last update time to current time
    LAST_UPDATE_TIME.save(deps.storage, &env.block.time.seconds())?;
    Ok(Response::new().add_messages(msgs).add_messages(bankmsgs))
}
