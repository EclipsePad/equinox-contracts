use astroport::staking::ExecuteMsg;
use cosmwasm_std::{
    ensure, ensure_eq, ensure_ne, to_json_binary, Coin, CosmosMsg, DepsMut, Env, MessageInfo,
    Response, Uint128, WasmMsg,
};

use cw20::Cw20ExecuteMsg;
use cw_utils::one_coin;
use equinox_msg::token_converter::{CallbackMsg, RewardConfig, RewardResponse, UpdateConfig};
use equinox_msg::voter::ExecuteMsg as VoterExecuteMsg;

use crate::entry::query::_query_rewards;
use crate::external_queriers::query_rates_astro_staking;
use crate::math::calculate_eclipastro_amount;
use crate::{
    error::ContractError,
    state::{
        CONFIG, OWNER, REWARD_CONFIG, TOTAL_STAKE_INFO, TREASURY_REWARD, WITHDRAWABLE_BALANCE,
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
    if let Some(vxastro_holder) = new_config.vxastro_holder {
        config.vxastro_holder = Some(vxastro_holder.clone());
        res = res.add_attribute("vxastro_holder", vxastro_holder.to_string());
    }
    if let Some(treasury) = new_config.treasury {
        config.treasury = treasury.clone();
        res = res.add_attribute("treasury", treasury);
    }
    if let Some(stability_pool) = new_config.stability_pool {
        config.stability_pool = Some(stability_pool.clone());
        res = res.add_attribute("stability_pool", stability_pool.to_string());
    }
    if let Some(staking_reward_distributor) = new_config.staking_reward_distributor {
        config.staking_reward_distributor = Some(staking_reward_distributor.clone());
        res = res.add_attribute("staking_reward_distributor", staking_reward_distributor);
    }
    if let Some(ce_reward_distributor) = new_config.ce_reward_distributor {
        config.ce_reward_distributor = Some(ce_reward_distributor.clone());
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

pub fn _claim(
    deps: DepsMut,
    treasury_claim_amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let reward_config = REWARD_CONFIG.load(deps.storage)?;
    let mut total_stake_info = TOTAL_STAKE_INFO.load(deps.storage).unwrap_or_default();
    let mut withdrawable = WITHDRAWABLE_BALANCE.load(deps.storage).unwrap_or_default();
    let mut treasury_reward = TREASURY_REWARD.load(deps.storage).unwrap_or_default();
    let res: (RewardResponse, Uint128) = _query_rewards(deps.as_ref())?;
    let claimable_xastro = res.1;
    let users_reward = claimable_xastro.multiply_ratio(reward_config.users, 10000u32);
    let reward_ce_holders = res.0.ce_holders_reward.amount;
    let reward_stability_pool = res.0.stability_pool_reward.amount;
    let reward_treasury = res.0.treasury_reward.amount;
    // must exist users reward
    ensure_ne!(
        users_reward,
        Uint128::zero(),
        ContractError::NoRewardClaimable {}
    );
    // add message to mint eclipASTRO to staking_reward_distributor
    let mut msgs = vec![mint_eclipastro_msg(
        config.staking_reward_distributor.clone().unwrap().to_string(),
        res.0.users_reward.amount,
        config.eclipastro.to_string(),
    )?];
    // withdrawable is withdrawable xASTRO amount that DAO withdraws. equal to staking eclipASTRO rewards part
    withdrawable = withdrawable.checked_add(users_reward).unwrap();
    // add messsage to withdraw xASTRO DAO rewards - treasury rewards
    if reward_ce_holders.gt(&Uint128::zero()) {
        msgs.push(withdraw_xastro_msg(
            config.vxastro_holder.clone().unwrap().to_string(),
            config.ce_reward_distributor.clone().unwrap().to_string(),
            reward_ce_holders,
        )?);
    }
    if reward_stability_pool.gt(&Uint128::zero()) {
        msgs.push(withdraw_xastro_msg(
            config.vxastro_holder.clone().unwrap().to_string(),
            config.staking_reward_distributor.clone().unwrap().to_string(),
            reward_ce_holders,
        )?);
    }
    // deduct claimable
    total_stake_info.claimed_xastro += claimable_xastro;
    treasury_reward = treasury_reward
        .checked_add(
            reward_treasury
        )
        .unwrap();

    let mut response = Response::new()
        .add_attribute("action", "claim reward")
        .add_attribute("token", "eclipASTRO")
        .add_attribute(
            "recipient",
            config.staking_reward_distributor.unwrap().to_string(),
        )
        .add_attribute("amount", users_reward.to_string())
        .add_attribute("token", "xASTRO")
        .add_attribute(
            "recipient",
            config.ce_reward_distributor.unwrap().to_string(),
        )
        .add_attribute("amount", reward_ce_holders.to_string())
        .add_attribute("token", "xASTRO")
        .add_attribute("recipient", config.stability_pool.unwrap().to_string())
        .add_attribute("amount", reward_stability_pool.to_string());

    if !treasury_claim_amount.is_zero() {
        ensure!(
            treasury_claim_amount.le(&treasury_reward),
            ContractError::NotEnoughBalance {}
        );
        msgs.push(withdraw_xastro_msg(
            config.vxastro_holder.unwrap().to_string(),
            config.treasury.to_string(),
            treasury_claim_amount,
        )?);
        treasury_reward -= treasury_claim_amount;
        response = response
            .add_attribute("action", "claim treasury reward")
            .add_attribute("amount", treasury_claim_amount.to_string());
    }

    WITHDRAWABLE_BALANCE.save(deps.storage, &withdrawable)?;
    TOTAL_STAKE_INFO.save(deps.storage, &total_stake_info)?;
    TREASURY_REWARD.save(deps.storage, &treasury_reward)?;
    Ok(response.add_messages(msgs))
}

/// claim user rewards
pub fn claim(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // only staking reward distributor contract can execute this function
    ensure_eq!(
        info.sender,
        config.staking_reward_distributor.unwrap(),
        ContractError::Unauthorized {}
    );
    _claim(deps, Uint128::zero())
}

/// claim treasury rewards
pub fn claim_treasury_reward(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    _claim(deps, amount)
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
        .add_message(withdraw_xastro_msg(
            config.vxastro_holder.unwrap().to_string(),
            recipient,
            amount,
        )?)
        .add_attribute("action", "withdraw xtoken")
        .add_attribute("amount", amount.to_string()))
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
        CallbackMsg::ConvertAstro {
            prev_xastro_balance,
            astro_amount_to_convert,
            receiver,
        } => handle_convert_astro(
            deps,
            env,
            prev_xastro_balance,
            astro_amount_to_convert,
            receiver,
        ),
    }
}

fn handle_convert_astro(
    deps: DepsMut,
    env: Env,
    prev_xastro_balance: Uint128,
    astro_amount_to_convert: Uint128,
    receiver: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut total_staking = TOTAL_STAKE_INFO.load(deps.storage)?;

    let xastro_balance = deps
        .querier
        .query_balance(env.contract.address, config.xastro.clone())?;
    let converted_xastro = xastro_balance.amount - prev_xastro_balance;
    let xastro_token = Coin {
        denom: config.xastro,
        amount: converted_xastro,
    };
    let msgs = vec![
        send_xastro_msg(config.vxastro_holder.unwrap().to_string(), xastro_token.clone())?,
        mint_eclipastro_msg(
            receiver,
            astro_amount_to_convert,
            config.eclipastro.to_string(),
        )?,
    ];

    total_staking.xastro += converted_xastro;
    total_staking.astro += astro_amount_to_convert;

    TOTAL_STAKE_INFO.save(deps.storage, &total_staking)?;

    Ok(Response::new()
        .add_attribute("action", "lock xASTRO")
        .add_attribute("xASTRO", xastro_token.amount.to_string())
        .add_attribute("action", "mint eclipastro")
        .add_attribute("eclipASTRO", astro_amount_to_convert.to_string())
        .add_messages(msgs))
}

/// Stake ASTRO/xASTRO
pub fn try_convert(deps: DepsMut, env: Env, info: MessageInfo, recipient: Option<String>) -> Result<Response, ContractError> {
    let received_token = one_coin(&info)?;
    let config = CONFIG.load(deps.storage)?;
    let mut total_staking = TOTAL_STAKE_INFO.load(deps.storage).unwrap_or_default();
    let receiver = recipient.unwrap_or(info.sender.into_string());

    // only ASTRO token or xASTRO token can execute this message
    ensure!(
        received_token.denom == config.astro || received_token.denom == config.xastro,
        ContractError::UnknownToken(received_token.denom.clone())
    );
    if received_token.denom == config.astro {
        let xastro_balance = deps
            .querier
            .query_balance(&env.contract.address, config.xastro)?;
        return Ok(Response::new()
            .add_messages(vec![
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: config.staking_contract.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Enter { receiver: None })?,
                    funds: vec![received_token.clone()],
                }),
                CallbackMsg::ConvertAstro {
                    prev_xastro_balance: xastro_balance.amount,
                    astro_amount_to_convert: received_token.amount,
                    receiver,
                }
                .to_cosmos_msg(&env)?,
            ])
            .add_attribute("action", "stake ASTRO")
            .add_attribute("ASTRO", received_token.amount.to_string()));
    }
    let rate = query_rates_astro_staking(deps.as_ref(), config.staking_contract.to_string())?;
    let eclipastro_amount =
        calculate_eclipastro_amount(rate, Uint128::from(received_token.amount));
    let msgs = vec![
        send_xastro_msg(config.vxastro_holder.unwrap().to_string(), received_token.clone())?,
        mint_eclipastro_msg(
            receiver.clone(),
            eclipastro_amount,
            config.eclipastro.to_string(),
        )?,
    ];

    total_staking.xastro += received_token.amount;
    total_staking.astro += eclipastro_amount;

    TOTAL_STAKE_INFO.save(deps.storage, &total_staking)?;

    Ok(Response::new()
        .add_attribute("action", "lock xASTRO")
        .add_attribute("xASTRO", received_token.amount.to_string())
        .add_attribute("action", "mint eclipastro")
        .add_attribute("eclipASTRO", eclipastro_amount.to_string())
        .add_messages(msgs))
}

pub fn send_xastro_msg(voter: String, coin: Coin) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: voter,
        msg: to_json_binary(&VoterExecuteMsg::Stake {})?,
        funds: vec![coin],
    }))
}

pub fn withdraw_xastro_msg(
    voter: String,
    recipient: String,
    amount: Uint128,
) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: voter,
        msg: to_json_binary(&VoterExecuteMsg::Withdraw { amount, recipient })?,
        funds: vec![],
    }))
}

pub fn mint_eclipastro_msg(
    receiver: String,
    amount: Uint128,
    eclipastro: String,
) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: eclipastro,
        msg: to_json_binary(&Cw20ExecuteMsg::Mint {
            recipient: receiver,
            amount: amount,
        })?,
        funds: vec![],
    }))
}
