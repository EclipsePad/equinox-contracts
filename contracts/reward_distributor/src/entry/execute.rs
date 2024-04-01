use std::vec;

use cosmwasm_std::{
    coins, ensure, to_json_binary, BankMsg, DepsMut, Env, MessageInfo, Order, Response, StdResult,
    Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use equinox_msg::{
    reward_distributor::UpdateConfigMsg,
    timelock_staking::RestakingDetail,
    token_converter::{
        ExecuteMsg as ConverterExecuteMsg, QueryMsg as ConverterQueryMsg, RewardResponse,
    },
};

use crate::{
    entry::query::{total_staking_amount_update, total_staking_reward_update, user_reward_update},
    error::ContractError,
    state::{
        CONFIG, FLEXIBLE_USER_STAKING, LAST_UPDATE_TIME, OWNER, PENDING_REWARDS,
        REWARD_DISTRIBUTION_PERIOD, REWARD_DISTRIBUTION_TIME_DIFF, TIMELOCK_USER_STAKING,
        TOTAL_STAKING,
    },
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

/// flexible staking event
/// Only flexible staking contract
/// Non zero amount
pub fn flexible_stake(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure!(
        info.sender == config.flexible_staking,
        ContractError::Unauthorized {}
    );
    ensure!(amount.gt(&Uint128::zero()), ContractError::ZeroAmount {});
    // update total staking reward
    let mut total_staking_data = total_staking_reward_update(deps.as_ref(), env.clone())?;
    // update total staking balance
    total_staking_data = total_staking_amount_update(total_staking_data, 0u64, amount, true)?;
    // get user flexible staking data
    let mut user_staking = FLEXIBLE_USER_STAKING
        .load(deps.storage, &user)
        .unwrap_or_default();
    // update user staking reward
    user_staking = user_reward_update(deps.as_ref(), &total_staking_data, 0u64, &user_staking)?;
    // update user staking balance
    user_staking.amount = user_staking.amount.checked_add(amount).unwrap();
    TOTAL_STAKING.save(deps.storage, &total_staking_data)?;
    // update last update time as current block time
    LAST_UPDATE_TIME.save(deps.storage, &env.block.time.seconds())?;
    FLEXIBLE_USER_STAKING.save(deps.storage, &user, &user_staking)?;
    // calculate xASTRO rewards
    let pending_eclipastro_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    // claim eclipastro rewards if exists
    if pending_eclipastro_reward
        .users_reward
        .amount
        .gt(&Uint128::zero())
    {
        update_pending_rewards(
            deps.branch(),
            pending_eclipastro_reward.users_reward.amount,
            env.block.time.seconds(),
        )?;
        return Ok(Response::new().add_message(WasmMsg::Execute {
            contract_addr: config.token_converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Claim {})?,
            funds: vec![],
        }));
    }
    Ok(Response::new())
}

/// timelock staking event
/// Only timelock staking contract
/// Non zero amount
pub fn timelock_stake(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    amount: Uint128,
    duration: u64,
) -> Result<Response, ContractError> {
    let current_time = env.block.time.seconds();
    let config = CONFIG.load(deps.storage)?;
    ensure!(
        info.sender == config.timelock_staking,
        ContractError::Unauthorized {}
    );
    ensure!(amount.gt(&Uint128::zero()), ContractError::ZeroAmount {});

    // update total staking reward
    let mut total_staking_data = total_staking_reward_update(deps.as_ref(), env.clone())?;
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

    // calculate xASTRO rewards
    let pending_eclipastro_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    // claim eclipastro rewards if exists
    if pending_eclipastro_reward
        .users_reward
        .amount
        .gt(&Uint128::zero())
    {
        update_pending_rewards(
            deps.branch(),
            pending_eclipastro_reward.users_reward.amount,
            env.block.time.seconds(),
        )?;
        return Ok(Response::new().add_message(WasmMsg::Execute {
            contract_addr: config.token_converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Claim {})?,
            funds: vec![],
        }));
    }
    Ok(Response::new())
}

/// claim rewards
/// only flexible staking contract
pub fn flexible_stake_claim(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    ensure!(
        info.sender == config.flexible_staking,
        ContractError::Unauthorized {}
    );

    // update total staking rewards
    let total_staking_data = total_staking_reward_update(deps.as_ref(), env.clone())?;
    // update user staking rewards
    let mut user_staking = FLEXIBLE_USER_STAKING.load(deps.storage, &user)?;
    user_staking = user_reward_update(deps.as_ref(), &total_staking_data, 0u64, &user_staking)?;

    ensure!(
        user_staking
            .rewards
            .eclipastro
            .pending_reward
            .gt(&Uint128::zero())
            || user_staking
                .rewards
                .eclip
                .pending_reward
                .gt(&Uint128::zero()),
        ContractError::NoReward {}
    );

    let mut msgs = vec![];

    // calculate xASTRO rewards
    let pending_eclipastro_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    // if there is pending eclipASTRO rewards from xASTRO rewards, claim it
    if pending_eclipastro_reward
        .users_reward
        .amount
        .gt(&Uint128::zero())
    {
        update_pending_rewards(
            deps.branch(),
            pending_eclipastro_reward.users_reward.amount,
            env.block.time.seconds(),
        )?;
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
/// only timelock staking contract
pub fn timelock_stake_claim(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    duration: u64,
    locked_at: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    ensure!(
        info.sender == config.timelock_staking,
        ContractError::Unauthorized {}
    );

    // update total staking rewards
    let total_staking_data = total_staking_reward_update(deps.as_ref(), env.clone())?;
    let mut user_staking =
        TIMELOCK_USER_STAKING.load(deps.storage, (&user, duration, locked_at))?;
    // update user staking rewards
    user_staking = user_reward_update(deps.as_ref(), &total_staking_data, duration, &user_staking)?;

    ensure!(
        user_staking
            .rewards
            .eclipastro
            .pending_reward
            .gt(&Uint128::zero())
            || user_staking
                .rewards
                .eclip
                .pending_reward
                .gt(&Uint128::zero()),
        ContractError::NoReward {}
    );

    let mut msgs = vec![];

    // calculate xASTRO rewards
    let pending_eclipastro_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    // if there is pending eclipASTRO rewards from xASTRO rewards, claim it
    if pending_eclipastro_reward
        .users_reward
        .amount
        .gt(&Uint128::zero())
    {
        update_pending_rewards(
            deps.branch(),
            pending_eclipastro_reward.users_reward.amount,
            env.block.time.seconds(),
        )?;
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
/// only timelock staking contract
pub fn timelock_stake_claim_all(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    ensure!(
        info.sender == config.timelock_staking,
        ContractError::Unauthorized {}
    );

    // update total staking rewards
    let total_staking_data = total_staking_reward_update(deps.as_ref(), env.clone())?;

    let mut msgs = vec![];

    // calculate xASTRO rewards
    let pending_eclipastro_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    // if there is pending eclipASTRO rewards from xASTRO rewards, claim it
    if pending_eclipastro_reward
        .users_reward
        .amount
        .gt(&Uint128::zero())
    {
        update_pending_rewards(
            deps.branch(),
            pending_eclipastro_reward.users_reward.amount,
            env.block.time.seconds(),
        )?;
        msgs.push(WasmMsg::Execute {
            contract_addr: config.token_converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Claim {})?,
            funds: vec![],
        });
    }

    let mut res = Response::new();

    let mut eclipastro_reward = Uint128::zero();
    let mut eclip_reward = Uint128::zero();
    let user_staking_prefixes = TIMELOCK_USER_STAKING
        .sub_prefix(&user)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|s| {
            let ((duration, locked_at), mut staking_data) = s.unwrap();
            staking_data =
                user_reward_update(deps.as_ref(), &total_staking_data, duration, &staking_data)
                    .unwrap();
            eclipastro_reward = eclipastro_reward
                .checked_add(staking_data.rewards.eclipastro.pending_reward)
                .unwrap();
            eclip_reward = eclip_reward
                .checked_add(staking_data.rewards.eclip.pending_reward)
                .unwrap();
            Ok((duration, locked_at))
        })
        .collect::<StdResult<Vec<(u64, u64)>>>()
        .unwrap();

    ensure!(
        eclipastro_reward.gt(&Uint128::zero()) || eclip_reward.gt(&Uint128::zero()),
        ContractError::NoReward {}
    );

    // if there is user's eclipASTRO rewards, send it to user
    if eclipastro_reward.gt(&Uint128::zero()) {
        msgs.push(WasmMsg::Execute {
            contract_addr: config.eclipastro.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: user.clone(),
                amount: eclipastro_reward,
            })?,
            funds: vec![],
        });
        res = res
            .add_attribute("recipient", user.clone())
            .add_attribute("asset", config.eclipastro.to_string())
            .add_attribute("amount", eclipastro_reward.to_string());
    }
    // if there is user's ECLIP rewards, send it to user
    let mut bankmsgs = vec![];
    if eclip_reward.gt(&Uint128::zero()) {
        bankmsgs.push(BankMsg::Send {
            to_address: user.clone(),
            amount: coins(eclip_reward.u128(), config.eclip.clone()),
        });
        res = res
            .add_attribute("recipient", user.clone())
            .add_attribute("asset", config.eclip.clone().to_string())
            .add_attribute("amount", eclip_reward.to_string());
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
/// only flexible staking contract
pub fn flexible_unstake(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    ensure!(
        info.sender == config.flexible_staking,
        ContractError::Unauthorized {}
    );

    // update total staking reward
    let mut total_staking_data = total_staking_reward_update(deps.as_ref(), env.clone())?;
    // update total staking balance
    total_staking_data = total_staking_amount_update(total_staking_data, 0u64, amount, false)?;
    let mut user_staking = FLEXIBLE_USER_STAKING.load(deps.storage, &user)?;
    user_staking = user_reward_update(deps.as_ref(), &total_staking_data, 0u64, &user_staking)?;
    user_staking.amount = user_staking.amount.checked_sub(amount).unwrap();
    let mut msgs = vec![];

    // calculate xASTRO reward
    let pending_eclipastro_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    // if there is xASTRO rewards, claim it
    if pending_eclipastro_reward
        .users_reward
        .amount
        .gt(&Uint128::zero())
    {
        update_pending_rewards(
            deps.branch(),
            pending_eclipastro_reward.users_reward.amount,
            env.block.time.seconds(),
        )?;
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
/// only timelock staking contract
pub fn timelock_unstake(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    duration: u64,
    locked_at: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    ensure!(
        info.sender == config.timelock_staking,
        ContractError::Unauthorized {}
    );

    // update total staking reward
    let mut total_staking_data = total_staking_reward_update(deps.as_ref(), env.clone())?;
    let mut user_staking =
        TIMELOCK_USER_STAKING.load(deps.storage, (&user, duration, locked_at))?;
    // update total staking balance
    total_staking_data =
        total_staking_amount_update(total_staking_data, duration, user_staking.amount, false)?;
    // update user reward data
    user_staking = user_reward_update(deps.as_ref(), &total_staking_data, duration, &user_staking)?;
    let mut msgs = vec![];

    // calculate xASTRO reward
    let pending_eclipastro_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    // if there is xASTRO rewards, claim it
    if pending_eclipastro_reward
        .users_reward
        .amount
        .gt(&Uint128::zero())
    {
        update_pending_rewards(
            deps.branch(),
            pending_eclipastro_reward.users_reward.amount,
            env.block.time.seconds(),
        )?;
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
/// only timelock staking contract
pub fn restake(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    restaking_detail: RestakingDetail,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let receiver = restaking_detail.receiver;
    let user = restaking_detail.sender;
    let locked_at = restaking_detail.locked_at;
    let from_duration = restaking_detail.from_duration;
    let to_duration = restaking_detail.to_duration;
    let amount = restaking_detail.amount.unwrap();

    ensure!(
        info.sender == config.timelock_staking,
        ContractError::Unauthorized {}
    );

    let new_owner = receiver.unwrap_or(info.sender);

    // get user staking data
    let mut old_user_staking =
        TIMELOCK_USER_STAKING.load(deps.storage, (&user.to_string(), from_duration, locked_at))?;
    let mut new_user_staking = TIMELOCK_USER_STAKING
        .load(
            deps.storage,
            (
                &new_owner.to_string(),
                to_duration,
                env.block.time.seconds(),
            ),
        )
        .unwrap_or_default();
    // update total staking rewards info
    let mut total_staking_data = total_staking_reward_update(deps.as_ref(), env.clone())?;
    // update total staking balance from duration
    total_staking_data =
        total_staking_amount_update(total_staking_data, from_duration, amount, false)?;
    // update total staking balance to duration
    total_staking_data =
        total_staking_amount_update(total_staking_data, to_duration, amount, true)?;
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

    // calculate pending xASTRO rewards
    let pending_eclipastro_reward: RewardResponse = deps.querier.query_wasm_smart(
        config.token_converter.to_string(),
        &ConverterQueryMsg::Rewards {},
    )?;
    // claim pending xASTRO rewards
    if pending_eclipastro_reward
        .users_reward
        .amount
        .gt(&Uint128::zero())
    {
        update_pending_rewards(
            deps.branch(),
            pending_eclipastro_reward.users_reward.amount,
            env.block.time.seconds(),
        )?;
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
                    recipient: user.to_string(),
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
                to_address: user.to_string(),
                amount: coins(
                    old_user_staking.rewards.eclip.pending_reward.u128(),
                    config.eclip,
                ),
            })
        }
    }
    if amount == old_user_staking.amount {
        TIMELOCK_USER_STAKING.remove(deps.storage, (&user.to_string(), from_duration, locked_at));
    } else {
        old_user_staking.amount -= amount;
        TIMELOCK_USER_STAKING.save(
            deps.storage,
            (&user.to_string(), from_duration, locked_at),
            &old_user_staking,
        )?;
    }

    new_user_staking.amount = new_user_staking.amount.checked_add(amount).unwrap();
    TIMELOCK_USER_STAKING.save(
        deps.storage,
        (
            &new_owner.to_string(),
            to_duration,
            env.block.time.seconds(),
        ),
        &new_user_staking,
    )?;
    TOTAL_STAKING.save(deps.storage, &total_staking_data)?;
    // update last update time to current time
    LAST_UPDATE_TIME.save(deps.storage, &env.block.time.seconds())?;
    Ok(Response::new().add_messages(msgs).add_messages(bankmsgs))
}

pub fn update_pending_rewards(
    deps: DepsMut,
    pending_eclipastro_reward: Uint128,
    current_time: u64,
) -> Result<(), ContractError> {
    let pending_rewards = PENDING_REWARDS.last(deps.storage);
    match pending_rewards {
        Ok(d) => {
            if let Some(data) = d {
                if current_time - data.0 > REWARD_DISTRIBUTION_TIME_DIFF {
                    PENDING_REWARDS.save(deps.storage, current_time, &pending_eclipastro_reward)?;
                } else {
                    PENDING_REWARDS.save(
                        deps.storage,
                        data.0,
                        &data
                            .1
                            .checked_add(pending_eclipastro_reward.multiply_ratio(
                                REWARD_DISTRIBUTION_PERIOD,
                                data.0 + REWARD_DISTRIBUTION_TIME_DIFF - current_time,
                            ))
                            .unwrap(),
                    )?;
                }
            } else {
                PENDING_REWARDS.save(deps.storage, current_time, &pending_eclipastro_reward)?;
            }
        }
        Err(_) => {
            PENDING_REWARDS.save(deps.storage, current_time, &pending_eclipastro_reward)?;
        }
    }
    Ok(())
}
