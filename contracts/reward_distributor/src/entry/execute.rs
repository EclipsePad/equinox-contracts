use std::vec;

use cosmwasm_std::{
    coins, ensure, to_json_binary, BankMsg, DepsMut, Env, MessageInfo,
    Response, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use equinox_msg::{
    reward_distributor::UpdateConfigMsg,
    token_converter::ExecuteMsg as ConverterExecuteMsg,
};

use crate::{
    entry::query::{total_staking_amount_update, total_staking_reward_update, user_reward_update, user_staking_amount_update}, error::ContractError, state::{
        CONFIG, LAST_UPDATE_TIME, OWNER, TOTAL_STAKING,
        USER_REWARDS, USER_STAKING,
    }
};

/// Update config
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

/// staking event
pub fn stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    amount: Uint128,
    duration: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure!(
        info.sender == config.flexible_staking || info.sender == config.timelock_staking,
        ContractError::Unauthorized {}
    );
    let mut total_staking_data = total_staking_reward_update(deps.as_ref(), env.clone())?;
    total_staking_data = total_staking_amount_update(total_staking_data, duration, amount, true)?;
    let user_rewards = user_reward_update(deps.as_ref(), &total_staking_data, &user)?;
    let mut user_staking = USER_STAKING.load(deps.storage, &user).unwrap_or(vec![]);
    user_staking = user_staking_amount_update(user_staking, duration, amount, true)?;
    TOTAL_STAKING.save(deps.storage, &total_staking_data)?;
    LAST_UPDATE_TIME.save(deps.storage, &env.block.time.seconds())?;
    USER_REWARDS.save(deps.storage, &user, &user_rewards)?;
    USER_STAKING.save(deps.storage, &user, &user_staking)?;
    // claim eclipastro rewards
    Ok(Response::new().add_message(WasmMsg::Execute {
        contract_addr: config.token_converter.to_string(),
        msg: to_json_binary(&ConverterExecuteMsg::Claim {})?,
        funds: vec![],
    }))
}

/// claim rewards
pub fn claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure!(
        info.sender == config.flexible_staking || info.sender == config.timelock_staking,
        ContractError::Unauthorized {}
    );
    let total_staking_data = total_staking_reward_update(deps.as_ref(), env.clone())?;
    let mut user_rewards = user_reward_update(deps.as_ref(), &total_staking_data, &user)?;
    let msgs = vec![
        WasmMsg::Execute {
            contract_addr: config.token_converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Claim {})?,
            funds: vec![],
        },
        WasmMsg::Execute {
            contract_addr: config.eclipastro.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: user.clone(),
                amount: user_rewards.eclipastro.pending_reward,
            })?,
            funds: vec![],
        },
    ];
    let eclip_msg = BankMsg::Send {
        to_address: user.clone(),
        amount: coins(user_rewards.eclip.pending_reward.u128(), config.eclip),
    };
    user_rewards.eclip.pending_reward = Uint128::zero();
    user_rewards.eclipastro.pending_reward = Uint128::zero();
    TOTAL_STAKING.save(deps.storage, &total_staking_data)?;
    LAST_UPDATE_TIME.save(deps.storage, &env.block.time.seconds())?;
    USER_REWARDS.save(deps.storage, &user, &user_rewards)?;
    // claim rewards
    Ok(Response::new().add_messages(msgs).add_message(eclip_msg))
}

/// unstaking event
pub fn unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    amount: Uint128,
    duration: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure!(
        info.sender == config.flexible_staking || info.sender == config.timelock_staking,
        ContractError::Unauthorized {}
    );
    let mut total_staking_data = total_staking_reward_update(deps.as_ref(), env.clone())?;
    total_staking_data = total_staking_amount_update(total_staking_data, duration, amount, false)?;
    let mut user_rewards = user_reward_update(deps.as_ref(), &total_staking_data, &user)?;
    let mut user_staking = USER_STAKING.load(deps.storage, &user).unwrap_or(vec![]);
    user_staking = user_staking_amount_update(user_staking, duration, amount, false)?;
    let msgs = vec![
        WasmMsg::Execute {
            contract_addr: config.token_converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Claim {})?,
            funds: vec![],
        },
        WasmMsg::Execute {
            contract_addr: config.eclipastro.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: user.clone(),
                amount: user_rewards.eclipastro.pending_reward,
            })?,
            funds: vec![],
        },
    ];
    let eclip_msg = BankMsg::Send {
        to_address: user.clone(),
        amount: coins(user_rewards.eclip.pending_reward.u128(), config.eclip),
    };
    user_rewards.eclip.pending_reward = Uint128::zero();
    user_rewards.eclipastro.pending_reward = Uint128::zero();
    TOTAL_STAKING.save(deps.storage, &total_staking_data)?;
    LAST_UPDATE_TIME.save(deps.storage, &env.block.time.seconds())?;
    USER_REWARDS.save(deps.storage, &user, &user_rewards)?;
    USER_STAKING.save(deps.storage, &user, &user_staking)?;
    // claim rewards
    Ok(Response::new().add_messages(msgs).add_message(eclip_msg))
}

/// restaking event
pub fn restake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    amount: Uint128,
    from_duration: u64,
    to_duration: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure!(
        info.sender == config.timelock_staking,
        ContractError::Unauthorized {}
    );
    let mut total_staking_data = total_staking_reward_update(deps.as_ref(), env.clone())?;
    total_staking_data =
        total_staking_amount_update(total_staking_data, from_duration, amount, false)?;
    total_staking_data =
        total_staking_amount_update(total_staking_data, to_duration, amount, true)?;
    let mut user_rewards = user_reward_update(deps.as_ref(), &total_staking_data, &user)?;
    let mut user_staking = USER_STAKING.load(deps.storage, &user).unwrap_or(vec![]);
    user_staking = user_staking_amount_update(user_staking, from_duration, amount, false)?;
    user_staking = user_staking_amount_update(user_staking, to_duration, amount, true)?;
    let msgs = vec![
        WasmMsg::Execute {
            contract_addr: config.token_converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Claim {})?,
            funds: vec![],
        },
        WasmMsg::Execute {
            contract_addr: config.eclipastro.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: user.clone(),
                amount: user_rewards.eclipastro.pending_reward,
            })?,
            funds: vec![],
        },
    ];
    let eclip_msg = BankMsg::Send {
        to_address: user.clone(),
        amount: coins(user_rewards.eclip.pending_reward.u128(), config.eclip),
    };
    user_rewards.eclip.pending_reward = Uint128::zero();
    user_rewards.eclipastro.pending_reward = Uint128::zero();
    TOTAL_STAKING.save(deps.storage, &total_staking_data)?;
    LAST_UPDATE_TIME.save(deps.storage, &env.block.time.seconds())?;
    USER_REWARDS.save(deps.storage, &user, &user_rewards)?;
    USER_STAKING.save(deps.storage, &user, &user_staking)?;
    // claim rewards
    Ok(Response::new().add_messages(msgs).add_message(eclip_msg))
}
