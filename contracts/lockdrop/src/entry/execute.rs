use astroport::{
    asset::{Asset, AssetInfo},
    pair::ExecuteMsg as PairExecuteMsg,
    staking::ExecuteMsg as AstroStakingExecuteMsg,
    token::BalanceResponse,
};
use cosmwasm_std::{
    attr, coin, ensure, ensure_eq, ensure_ne, from_json, to_json_binary, BankMsg, Coin, CosmosMsg,
    DepsMut, Env, MessageInfo, Order, Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, Cw20ReceiveMsg};
use cw_utils::one_coin;
use equinox_msg::{
    lockdrop::{CallbackMsg, Cw20HookMsg, RewardDistributionConfig, StakeType, UpdateConfigMsg},
    lp_staking::{Cw20HookMsg as LpStakingCw20HookMsg, ExecuteMsg as LpExecuteMsg},
    single_sided_staking::{
        Cw20HookMsg as SingleSidedCw20HookMsg, ExecuteMsg as SingleSidedExecuteMsg,
    },
    token_converter::ExecuteMsg as ConverterExecuteMsg,
};

use crate::{
    entry::query::{
        calculate_lp_staking_beclip_incentives, calculate_lp_staking_user_rewards,
        calculate_lp_total_rewards, calculate_single_sided_total_rewards,
        calculate_single_staking_beclip_incentives, calculate_single_staking_user_rewards,
        calculate_updated_lp_reward_weights, calculate_updated_single_staking_reward_weights,
        calculate_user_beclip_incentives_for_lp_lockup,
        calculate_user_beclip_incentives_for_single_lockup,
    },
    error::ContractError,
    math::{calculate_max_withdrawal_amount_allowed, calculate_weight},
    state::{
        CONFIG, LP_LOCKUP_INFO, LP_LOCKUP_STATE, LP_STAKING_REWARD_WEIGHTS, LP_USER_LOCKUP_INFO,
        OWNER, REWARD_DISTRIBUTION_CONFIG, SINGLE_LOCKUP_INFO, SINGLE_LOCKUP_STATE,
        SINGLE_STAKING_REWARD_WEIGHTS, SINGLE_USER_LOCKUP_INFO, TOTAL_BECLIP_INCENTIVES,
    },
};

pub fn try_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_cfg: UpdateConfigMsg,
) -> Result<Response, ContractError> {
    let mut cfg = CONFIG.load(deps.storage)?;
    let mut attributes = vec![attr("action", "update_config")];

    OWNER.assert_admin(deps.as_ref(), &info.sender)?;

    if let Some(dao_treasury_address) = new_cfg.dao_treasury_address {
        cfg.dao_treasury_address = dao_treasury_address.clone();
        attributes.push(attr("new_timelock_staking", &dao_treasury_address))
    };
    CONFIG.save(deps.storage, &cfg)?;
    Ok(Response::new().add_attributes(attributes))
}

pub fn try_update_reward_distribution_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_cfg: RewardDistributionConfig,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let cfg = CONFIG.load(deps.storage)?;

    ensure!(
        env.block.time.seconds() <= cfg.init_timestamp + cfg.deposit_window + cfg.withdrawal_window,
        ContractError::LockdropFinished {}
    );

    let attributes = vec![
        attr("action", "update_reward_distribution_config"),
        attr("instant_bps", new_cfg.instant.to_string()),
        attr("vesting period", new_cfg.vesting_period.to_string()),
    ];

    REWARD_DISTRIBUTION_CONFIG.save(deps.storage, &new_cfg)?;
    Ok(Response::new().add_attributes(attributes))
}

pub fn try_increase_lockup(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stake_type: StakeType,
    duration: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let received_token = one_coin(&info)?;
    let sender = info.sender.to_string();

    let current_time = env.block.time.seconds();
    ensure!(
        current_time >= cfg.init_timestamp,
        ContractError::DepositWindowNotStarted {}
    );
    ensure!(
        current_time < cfg.init_timestamp + cfg.deposit_window,
        ContractError::DepositWindowClosed {}
    );
    ensure_ne!(
        cfg.lock_configs.iter().find(|c| c.duration == duration),
        None,
        ContractError::InvalidDuration(duration)
    );
    ensure!(
        received_token.denom == cfg.astro_token || received_token.denom == cfg.xastro_token,
        ContractError::InvalidAsset {}
    );

    if received_token.denom == cfg.astro_token {
        let xastro_balance = deps
            .querier
            .query_balance(&env.contract.address, cfg.xastro_token)?;
        let msgs = vec![
            astro_convert_msg(cfg.astro_staking.to_string(), &received_token)?,
            CallbackMsg::IncreaseLockup {
                prev_xastro_balance: xastro_balance.amount,
                stake_type,
                duration,
                sender,
            }
            .to_cosmos_msg(&env)?,
        ];
        return Ok(Response::new()
            .add_attribute("action", "convert ASTRO to xASTRO")
            .add_attribute("amount", received_token.amount.to_string())
            .add_messages(msgs));
    }

    match stake_type {
        StakeType::SingleStaking => increase_single_lockup(
            deps,
            duration,
            sender,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: received_token.denom,
                },
                amount: received_token.amount,
            },
        ),
        StakeType::LpStaking => increase_lp_lockup(
            deps,
            duration,
            sender,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: received_token.denom,
                },
                amount: received_token.amount,
            },
        ),
    }
}

pub fn try_extend_lockup(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stake_type: StakeType,
    from_duration: u64,
    to_duration: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let received_tokens = info.funds;
    let deposit_existing = !received_tokens.is_empty();
    ensure!(
        received_tokens.iter().len() <= 1,
        ContractError::InvalidAsset {}
    );
    let sender = info.sender.to_string();

    ensure_ne!(
        cfg.lock_configs
            .iter()
            .find(|c| c.duration == from_duration),
        None,
        ContractError::InvalidDuration(from_duration)
    );

    ensure_ne!(
        cfg.lock_configs.iter().find(|c| c.duration == to_duration),
        None,
        ContractError::InvalidDuration(to_duration)
    );

    let received_token = &received_tokens[0];

    if deposit_existing {
        ensure!(
            received_token.denom == cfg.astro_token || received_token.denom == cfg.xastro_token,
            ContractError::InvalidAsset {}
        );
    }

    let current_time = env.block.time.seconds();
    // deposit window only
    if current_time >= cfg.init_timestamp && current_time < cfg.init_timestamp + cfg.deposit_window
    {
        let mut add_amount = Uint128::zero();
        if deposit_existing {
            if received_token.denom == cfg.astro_token {
                let xastro_balance = deps
                    .querier
                    .query_balance(&env.contract.address, cfg.xastro_token)?;
                let msgs = vec![
                    astro_convert_msg(cfg.astro_staking.to_string(), received_token)?,
                    CallbackMsg::ExtendLockup {
                        prev_xastro_balance: xastro_balance.amount,
                        stake_type,
                        from_duration,
                        to_duration,
                        sender,
                    }
                    .to_cosmos_msg(&env)?,
                ];
                return Ok(Response::new()
                    .add_attribute("action", "convert ASTRO to xASTRO")
                    .add_attribute("amount", received_token.amount.to_string())
                    .add_messages(msgs));
            }
            add_amount = received_token.amount;
        }
        match stake_type {
            StakeType::SingleStaking => {
                return extend_single_lockup(deps, from_duration, to_duration, sender, add_amount);
            }
            StakeType::LpStaking => {
                return extend_lp_lockup(deps, from_duration, to_duration, sender, add_amount);
            }
        }
    } else if current_time >= cfg.init_timestamp + cfg.deposit_window + cfg.withdrawal_window
        && cfg.claims_allowed
    {
        match stake_type {
            StakeType::SingleStaking => {
                if deposit_existing {
                    let eclipastro_balance: BalanceResponse = deps.querier.query_wasm_smart(
                        &cfg.eclipastro_token,
                        &Cw20QueryMsg::Balance {
                            address: env.contract.address.to_string(),
                        },
                    )?;
                    let msgs = vec![
                        convert_eclipastro_msg(cfg.converter.to_string(), received_token)?,
                        CallbackMsg::ExtendLockupAfterLockdrop {
                            prev_eclipastro_balance: eclipastro_balance.balance,
                            from_duration,
                            to_duration,
                            sender,
                        }
                        .to_cosmos_msg(&env)?,
                    ];
                    return Ok(Response::new()
                        .add_attribute("action", "convert ASTRO to xASTRO")
                        .add_attribute("amount", received_token.amount.to_string())
                        .add_messages(msgs));
                }
                return extend_single_lockup_after_lockdrop(
                    deps,
                    env,
                    from_duration,
                    to_duration,
                    sender,
                    Uint128::zero(),
                );
            }
            StakeType::LpStaking => {
                return Err(ContractError::Std(StdError::generic_err(
                    "Extend Lockup after Lockdrop is not allowed for lp staking",
                )));
            }
        }
    }
    Err(ContractError::ExtendLockupError {})
}

pub fn try_stake_to_vaults(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // check is owner
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let mut cfg = CONFIG.load(deps.storage)?;
    let current_time = env.block.time.seconds();

    // check is already staked
    ensure!(!cfg.claims_allowed, ContractError::AlreadyStaked {});

    // check time window
    ensure!(
        current_time > (cfg.init_timestamp + cfg.deposit_window + cfg.withdrawal_window),
        ContractError::LockdropNotFinished {}
    );

    cfg.claims_allowed = true;
    cfg.countdown_start_at = current_time;

    CONFIG.save(deps.storage, &cfg)?;
    let single_msgs = handle_stake_single_vault(deps.branch(), env.clone())?;
    let lp_msgs = handle_stake_lp_vault(deps, env)?;

    Ok(Response::new()
        .add_messages(single_msgs)
        .add_messages(lp_msgs))
}

pub fn try_claim_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stake_type: StakeType,
    duration: u64,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    match stake_type {
        StakeType::SingleStaking => _claim_single_sided_rewards(deps, env, sender, duration),
        StakeType::LpStaking => _claim_lp_rewards(deps, env, sender, duration),
    }
}

pub fn try_claim_all_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stake_type: StakeType,
    with_flexible: bool,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    match stake_type {
        StakeType::SingleStaking => {
            _claim_all_single_sided_rewards(deps, env, sender, None, with_flexible)
        }
        StakeType::LpStaking => _claim_all_lp_rewards(deps, env, sender, None, with_flexible),
    }
}

pub fn increase_single_lockup(
    deps: DepsMut,
    duration: u64,
    sender: String,
    asset: Asset,
) -> Result<Response, ContractError> {
    let mut lockup_info = SINGLE_LOCKUP_INFO
        .load(deps.storage, duration)
        .unwrap_or_default();
    let mut user_lockup_info = SINGLE_USER_LOCKUP_INFO
        .load(deps.storage, (&sender, duration))
        .unwrap_or_default();
    let staking_amount = asset.amount;
    lockup_info.xastro_amount_in_lockups = lockup_info
        .xastro_amount_in_lockups
        .checked_add(staking_amount)
        .unwrap();
    user_lockup_info.xastro_amount_in_lockups = user_lockup_info
        .xastro_amount_in_lockups
        .checked_add(staking_amount)
        .unwrap();
    SINGLE_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
    SINGLE_USER_LOCKUP_INFO.save(deps.storage, (&sender, duration), &user_lockup_info)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "increase_lockup_position"),
        attr("type", "single staking"),
        attr("from", sender),
        attr("asset", asset.info.to_string()),
        attr("amount", asset.amount.to_string()),
        attr("duration", duration.to_string()),
    ]))
}

pub fn increase_lp_lockup(
    deps: DepsMut,
    duration: u64,
    sender: String,
    asset: Asset,
) -> Result<Response, ContractError> {
    let mut lockup_info = LP_LOCKUP_INFO
        .load(deps.storage, duration)
        .unwrap_or_default();
    let mut user_lockup_info = LP_USER_LOCKUP_INFO
        .load(deps.storage, (&sender, duration))
        .unwrap_or_default();
    let staking_amount = asset.amount;
    lockup_info.xastro_amount_in_lockups = lockup_info
        .xastro_amount_in_lockups
        .checked_add(staking_amount)
        .unwrap();
    user_lockup_info.xastro_amount_in_lockups = user_lockup_info
        .xastro_amount_in_lockups
        .checked_add(staking_amount)
        .unwrap();
    LP_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
    LP_USER_LOCKUP_INFO.save(deps.storage, (&sender, duration), &user_lockup_info)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "increase_lockup_position"),
        attr("type", "lp staking"),
        attr("from", sender),
        attr("asset", asset.info.to_string()),
        attr("amount", asset.amount.to_string()),
        attr("duration", duration.to_string()),
    ]))
}

pub fn extend_single_lockup(
    deps: DepsMut,
    from_duration: u64,
    to_duration: u64,
    sender: String,
    add_amount: Uint128,
) -> Result<Response, ContractError> {
    let mut lockup_info_from = SINGLE_LOCKUP_INFO.load(deps.storage, from_duration)?;
    let mut lockup_info_to = SINGLE_LOCKUP_INFO
        .load(deps.storage, to_duration)
        .unwrap_or_default();
    let user_lockup_info_from =
        SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&sender, from_duration))?;
    let mut user_lockup_info_to = SINGLE_USER_LOCKUP_INFO
        .load(deps.storage, (&sender, to_duration))
        .unwrap_or_default();

    let existing_xastro_amount = user_lockup_info_from.xastro_amount_in_lockups;
    ensure!(
        !existing_xastro_amount.is_zero(),
        ContractError::NotStaked {}
    );
    lockup_info_from.xastro_amount_in_lockups -= existing_xastro_amount;

    SINGLE_LOCKUP_INFO.save(deps.storage, from_duration, &lockup_info_from)?;
    SINGLE_USER_LOCKUP_INFO.remove(deps.storage, (&sender, from_duration));

    lockup_info_to.xastro_amount_in_lockups += existing_xastro_amount + add_amount;

    user_lockup_info_to.xastro_amount_in_lockups += existing_xastro_amount + add_amount;

    SINGLE_LOCKUP_INFO.save(deps.storage, to_duration, &lockup_info_to)?;
    SINGLE_USER_LOCKUP_INFO.save(deps.storage, (&sender, to_duration), &user_lockup_info_to)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "extend_duration"),
        attr("type", "single staking"),
        attr("from", sender),
        attr("from", from_duration.to_string()),
        attr("amount", existing_xastro_amount),
        attr("to", to_duration.to_string()),
        attr("amount", existing_xastro_amount + add_amount),
    ]))
}

pub fn extend_lp_lockup(
    deps: DepsMut,
    from_duration: u64,
    to_duration: u64,
    sender: String,
    add_amount: Uint128,
) -> Result<Response, ContractError> {
    let mut lockup_info_from = LP_LOCKUP_INFO.load(deps.storage, from_duration)?;
    let mut lockup_info_to = LP_LOCKUP_INFO
        .load(deps.storage, to_duration)
        .unwrap_or_default();
    let user_lockup_info_from = LP_USER_LOCKUP_INFO.load(deps.storage, (&sender, from_duration))?;
    let mut user_lockup_info_to = LP_USER_LOCKUP_INFO
        .load(deps.storage, (&sender, to_duration))
        .unwrap_or_default();

    let existing_xastro_amount = user_lockup_info_from.xastro_amount_in_lockups;
    ensure!(
        !existing_xastro_amount.is_zero(),
        ContractError::NotStaked {}
    );
    lockup_info_from.xastro_amount_in_lockups -= existing_xastro_amount;

    LP_LOCKUP_INFO.save(deps.storage, from_duration, &lockup_info_from)?;
    LP_USER_LOCKUP_INFO.remove(deps.storage, (&sender, from_duration));

    lockup_info_to.xastro_amount_in_lockups += existing_xastro_amount + add_amount;

    user_lockup_info_to.xastro_amount_in_lockups += existing_xastro_amount + add_amount;

    LP_LOCKUP_INFO.save(deps.storage, to_duration, &lockup_info_to)?;
    LP_USER_LOCKUP_INFO.save(deps.storage, (&sender, to_duration), &user_lockup_info_to)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "extend_duration"),
        attr("type", "single staking"),
        attr("from", sender),
        attr("from", from_duration.to_string()),
        attr("amount", existing_xastro_amount),
        attr("to", to_duration.to_string()),
        attr("amount", existing_xastro_amount + add_amount),
    ]))
}

pub fn extend_single_lockup_after_lockdrop(
    mut deps: DepsMut,
    env: Env,
    from_duration: u64,
    to_duration: u64,
    sender: String,
    add_amount: Uint128,
) -> Result<Response, ContractError> {
    let response = _claim_all_single_sided_rewards(
        deps.branch(),
        env,
        sender.clone(),
        Some(vec![from_duration, to_duration]),
        true,
    )?;
    let cfg = CONFIG.load(deps.storage)?;
    let mut lockup_info_from = SINGLE_LOCKUP_INFO.load(deps.storage, from_duration)?;
    let mut user_lockup_info_from =
        SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&sender, from_duration))?;

    let existing_eclipastro_amount = user_lockup_info_from.total_eclipastro_staked
        - user_lockup_info_from.total_eclipastro_withdrawed;
    ensure!(
        !existing_eclipastro_amount.is_zero(),
        ContractError::NotStaked {}
    );
    lockup_info_from.total_withdrawed += existing_eclipastro_amount;
    user_lockup_info_from.total_eclipastro_withdrawed += existing_eclipastro_amount;

    SINGLE_LOCKUP_INFO.save(deps.storage, from_duration, &lockup_info_from)?;
    SINGLE_USER_LOCKUP_INFO.save(
        deps.storage,
        (&sender, from_duration),
        &user_lockup_info_from,
    )?;
    let mut msgs = vec![];

    if add_amount.is_zero() {
        if from_duration == 0 {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.single_sided_staking.to_string(),
                msg: to_json_binary(&SingleSidedExecuteMsg::Restake {
                    from_duration,
                    locked_at: None,
                    amount: Some(existing_eclipastro_amount),
                    to_duration,
                    recipient: Some(sender.clone()),
                })?,
                funds: vec![],
            }))
        } else {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.single_sided_staking.to_string(),
                msg: to_json_binary(&SingleSidedExecuteMsg::Restake {
                    from_duration,
                    locked_at: Some(cfg.countdown_start_at),
                    amount: Some(existing_eclipastro_amount),
                    to_duration,
                    recipient: Some(sender.clone()),
                })?,
                funds: vec![],
            }))
        }
    } else if from_duration == 0 {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.eclipastro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: cfg.single_sided_staking.to_string(),
                amount: add_amount,
                msg: to_json_binary(&SingleSidedCw20HookMsg::Restake {
                    from_duration,
                    locked_at: None,
                    amount: Some(existing_eclipastro_amount),
                    to_duration,
                    recipient: Some(sender.clone()),
                })?,
            })?,
            funds: vec![],
        }))
    } else {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.eclipastro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: cfg.single_sided_staking.to_string(),
                amount: add_amount,
                msg: to_json_binary(&SingleSidedCw20HookMsg::Restake {
                    from_duration,
                    locked_at: Some(cfg.countdown_start_at),
                    amount: Some(existing_eclipastro_amount),
                    to_duration,
                    recipient: Some(sender.clone()),
                })?,
            })?,
            funds: vec![],
        }))
    }

    Ok(response
        .add_attributes(vec![
            attr("action", "extend_duration after lockdrop"),
            attr("type", "single staking"),
            attr("from", sender),
            attr("from", from_duration.to_string()),
            attr("amount", existing_eclipastro_amount),
            attr("to", to_duration.to_string()),
            attr("amount", existing_eclipastro_amount + add_amount),
        ])
        .add_messages(msgs))
}

pub fn handle_increase_lockup_callback(
    deps: DepsMut,
    env: Env,
    prev_xastro_balance: Uint128,
    stake_type: StakeType,
    duration: u64,
    sender: String,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let xastro_balance = deps
        .querier
        .query_balance(env.contract.address, cfg.xastro_token.clone())?;
    let xastro_deposit = xastro_balance.amount - prev_xastro_balance;
    ensure!(
        !xastro_deposit.is_zero(),
        ContractError::InvalidDepositAmounts {}
    );

    match stake_type {
        StakeType::SingleStaking => increase_single_lockup(
            deps,
            duration,
            sender,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: cfg.xastro_token,
                },
                amount: xastro_deposit,
            },
        ),
        StakeType::LpStaking => increase_lp_lockup(
            deps,
            duration,
            sender,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: cfg.xastro_token,
                },
                amount: xastro_deposit,
            },
        ),
    }
}

pub fn handle_extend_lockup_callback(
    deps: DepsMut,
    env: Env,
    prev_xastro_balance: Uint128,
    stake_type: StakeType,
    from_duration: u64,
    to_duration: u64,
    sender: String,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let xastro_balance = deps
        .querier
        .query_balance(env.contract.address, cfg.xastro_token)?;
    let xastro_deposit = xastro_balance.amount - prev_xastro_balance;
    ensure!(
        !xastro_deposit.is_zero(),
        ContractError::InvalidDepositAmounts {}
    );

    match stake_type {
        StakeType::SingleStaking => {
            extend_single_lockup(deps, from_duration, to_duration, sender, xastro_deposit)
        }
        StakeType::LpStaking => {
            extend_lp_lockup(deps, from_duration, to_duration, sender, xastro_deposit)
        }
    }
}

pub fn handle_extend_lockup_after_lockdrop_callback(
    deps: DepsMut,
    env: Env,
    prev_eclipastro_balance: Uint128,
    from_duration: u64,
    to_duration: u64,
    sender: String,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let eclipastro_balance = deps
        .querier
        .query_balance(&env.contract.address, cfg.eclipastro_token)?;
    let eclipastro_deposit = eclipastro_balance.amount - prev_eclipastro_balance;
    ensure!(
        !eclipastro_deposit.is_zero(),
        ContractError::InvalidDepositAmounts {}
    );

    extend_single_lockup_after_lockdrop(
        deps,
        env,
        from_duration,
        to_duration,
        sender,
        eclipastro_deposit,
    )
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let sender = deps.api.addr_validate(&cw20_msg.sender)?;
    let amount = cw20_msg.amount;

    // CHECK :: Tokens sent > 0
    ensure!(amount.gt(&Uint128::zero()), ContractError::ZeroAmount {});

    match from_json(&cw20_msg.msg)? {
        Cw20HookMsg::ExtendLockup {
            stake_type,
            from,
            to,
        } => {
            ensure!(
                info.sender.clone() == cfg.eclipastro_token,
                ContractError::InvalidAsset {}
            );

            ensure_ne!(
                cfg.lock_configs.iter().find(|c| c.duration == from),
                None,
                ContractError::InvalidDuration(from)
            );

            ensure_ne!(
                cfg.lock_configs.iter().find(|c| c.duration == to),
                None,
                ContractError::InvalidDuration(to)
            );

            ensure!(cfg.claims_allowed, ContractError::LockdropNotFinished {});
            match stake_type {
                StakeType::SingleStaking => extend_single_lockup_after_lockdrop(
                    deps,
                    env,
                    from,
                    to,
                    sender.to_string(),
                    amount,
                ),
                StakeType::LpStaking => Err(ContractError::Std(StdError::generic_err(
                    "Extend Lockup after Lockdrop is not allowed for lp staking",
                ))),
            }
        }
        Cw20HookMsg::IncreasebEclipIncentives {} => {
            ensure!(
                info.sender == cfg.beclip.to_string(),
                ContractError::InvalidAsset {}
            );
            OWNER.assert_admin(deps.as_ref(), &sender)?;

            let mut total_beclip_incentives = TOTAL_BECLIP_INCENTIVES
                .load(deps.storage)
                .unwrap_or_default();
            total_beclip_incentives += amount;
            TOTAL_BECLIP_INCENTIVES.save(deps.storage, &total_beclip_incentives)?;
            Ok(Response::new()
                .add_attribute("action", "increase bECLIP incentives")
                .add_attribute("amount", amount.to_string()))
        }
    }
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
        CallbackMsg::IncreaseLockup {
            prev_xastro_balance,
            stake_type,
            duration,
            sender,
        } => handle_increase_lockup_callback(
            deps,
            env,
            prev_xastro_balance,
            stake_type,
            duration,
            sender,
        ),
        CallbackMsg::ExtendLockup {
            prev_xastro_balance,
            stake_type,
            from_duration,
            to_duration,
            sender,
        } => handle_extend_lockup_callback(
            deps,
            env,
            prev_xastro_balance,
            stake_type,
            from_duration,
            to_duration,
            sender,
        ),
        CallbackMsg::ExtendLockupAfterLockdrop {
            prev_eclipastro_balance,
            from_duration,
            to_duration,
            sender,
        } => handle_extend_lockup_after_lockdrop_callback(
            deps,
            env,
            prev_eclipastro_balance,
            from_duration,
            to_duration,
            sender,
        ),
        CallbackMsg::StakeToSingleVault {
            prev_eclipastro_balance,
            xastro_amount_to_convert,
            weighted_amount,
        } => handle_stake_to_single_vault(
            deps,
            env,
            prev_eclipastro_balance,
            xastro_amount_to_convert,
            weighted_amount,
        ),
        CallbackMsg::DepositIntoPool {
            prev_eclipastro_balance,
            xastro_amount,
            weighted_amount,
        } => handle_deposit_into_pool(
            deps,
            env,
            prev_eclipastro_balance,
            xastro_amount,
            weighted_amount,
        ),
        CallbackMsg::StakeLpToken {
            prev_lp_token_balance,
        } => handle_stake_lp_token(deps, env, prev_lp_token_balance),
    }
}

/// stake all the lockup assets to single staking vault
/// staking is only allowed after withdraw window
/// only owner can do this
/// ASTRO/xASTRO will be converted to eclipASTRO and be staked to single staking vault
/// change SINGLE_STATE's is_staked to true
pub fn handle_stake_single_vault(deps: DepsMut, env: Env) -> Result<Vec<CosmosMsg>, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let lock_configs = cfg.lock_configs.clone();

    // get all single staking lockup assets on this contract
    let total_xastro_amount_to_staking = SINGLE_LOCKUP_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .fold(Uint128::zero(), |acc, cur| {
            let (_, info) = cur.unwrap();
            acc.checked_add(info.xastro_amount_in_lockups).unwrap()
        });

    if total_xastro_amount_to_staking.is_zero() {
        return Ok(vec![]);
    }

    let total_weighted_xastro_amount_to_staking = SINGLE_LOCKUP_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .fold(Uint128::zero(), |acc, cur| {
            let (duration, info) = cur.unwrap();
            let duration_multiplier = lock_configs
                .clone()
                .into_iter()
                .find(|c| c.duration == duration)
                .unwrap_or_default()
                .multiplier;
            acc.checked_add(
                info.xastro_amount_in_lockups
                    .checked_mul(Uint128::from(duration_multiplier))
                    .unwrap(),
            )
            .unwrap()
        });

    let mut msgs = vec![];

    if total_xastro_amount_to_staking.gt(&Uint128::zero()) {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Convert { recipient: None })?,
            funds: vec![coin(
                total_xastro_amount_to_staking.u128(),
                cfg.xastro_token,
            )],
        }));
    }

    // callback function to stake eclipASTRO to single staking vaults
    let prev_eclipastro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &cfg.eclipastro_token,
        &Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    msgs.push(
        CallbackMsg::StakeToSingleVault {
            prev_eclipastro_balance: prev_eclipastro_balance.balance,
            xastro_amount_to_convert: total_xastro_amount_to_staking,
            weighted_amount: total_weighted_xastro_amount_to_staking,
        }
        .to_cosmos_msg(&env)?,
    );

    Ok(msgs)
}

/// stake all the lockup assets to lp staking vault
/// staking is only allowed after withdraw window
/// only owner can do this
/// ASTRO/xASTRO will be converted to eclipASTRO/xASTRO(50%/50%) and be deposited to liquidity pool and be staked to lp staking vault
/// change LP_STATE's is_staked to true
pub fn handle_stake_lp_vault(deps: DepsMut, env: Env) -> Result<Vec<CosmosMsg>, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let lock_configs = cfg.lock_configs.clone();

    // get all lp staking lockup assets on this contract
    let xastro_amount_to_stake = LP_LOCKUP_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .fold(Uint128::zero(), |acc, cur| {
            let (_, info) = cur.unwrap();
            acc.checked_add(info.xastro_amount_in_lockups).unwrap()
        });

    if xastro_amount_to_stake.is_zero() {
        return Ok(vec![]);
    }

    let total_weighted_xastro_amount_to_staking = LP_LOCKUP_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .fold(Uint128::zero(), |acc, cur| {
            let (duration, info) = cur.unwrap();
            let duration_multiplier = lock_configs
                .clone()
                .into_iter()
                .find(|c| c.duration == duration)
                .unwrap_or_default()
                .multiplier;
            acc.checked_add(
                info.xastro_amount_in_lockups
                    .checked_mul(Uint128::from(duration_multiplier))
                    .unwrap(),
            )
            .unwrap()
        });

    let half_xastro_amount = xastro_amount_to_stake / Uint128::from(2u128);

    let mut msgs = vec![];
    if half_xastro_amount.gt(&Uint128::zero()) {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.converter.to_string(),
            msg: to_json_binary(&ConverterExecuteMsg::Convert { recipient: None })?,
            funds: vec![coin(half_xastro_amount.u128(), cfg.xastro_token)],
        }));
    }

    // callback function to stake eclipASTRO to single staking vaults
    let prev_eclipastro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &cfg.eclipastro_token,
        &Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;
    let prev_lp_token_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &cfg.lp_token,
        &Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;
    msgs.push(
        CallbackMsg::DepositIntoPool {
            prev_eclipastro_balance: prev_eclipastro_balance.balance,
            xastro_amount: xastro_amount_to_stake,
            weighted_amount: total_weighted_xastro_amount_to_staking,
        }
        .to_cosmos_msg(&env)?,
    );
    msgs.push(
        CallbackMsg::StakeLpToken {
            prev_lp_token_balance: prev_lp_token_balance.balance,
        }
        .to_cosmos_msg(&env)?,
    );

    Ok(msgs)
}

// stake eclipASTRO to single staking vault
// save xASTRO/eclipASTRO rate
// is_staked = true
fn handle_stake_to_single_vault(
    deps: DepsMut,
    env: Env,
    prev_eclipastro_balance: Uint128,
    xastro_amount_to_convert: Uint128,
    weighted_amount: Uint128,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let eclipastro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &cfg.eclipastro_token,
        &Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;
    let eclipastro_balance_to_lockup = eclipastro_balance
        .balance
        .checked_sub(prev_eclipastro_balance)
        .unwrap();
    state.total_xastro = xastro_amount_to_convert;
    state.weighted_total_xastro = weighted_amount;
    state.total_eclipastro_lockup = eclipastro_balance_to_lockup;

    let mut response = Response::new()
        .add_attribute("action", "convert xASTRO to eclipASTRO")
        .add_attribute("from", cfg.xastro_token.to_string())
        .add_attribute("amount", xastro_amount_to_convert)
        .add_attribute("to", cfg.eclipastro_token.to_string())
        .add_attribute("amount", eclipastro_balance_to_lockup);
    let mut msgs = vec![];
    for c in &cfg.lock_configs {
        let mut lockup_info = SINGLE_LOCKUP_INFO
            .load(deps.storage, c.duration)
            .unwrap_or_default();
        let eclipastro_amount_to_stake = lockup_info
            .xastro_amount_in_lockups
            .multiply_ratio(eclipastro_balance_to_lockup, xastro_amount_to_convert);
        if eclipastro_amount_to_stake.is_zero() {
            continue;
        }
        lockup_info.total_staked = eclipastro_amount_to_stake;
        SINGLE_LOCKUP_INFO.save(deps.storage, c.duration, &lockup_info)?;
        state.weighted_total_eclipastro_lockup +=
            calculate_weight(eclipastro_amount_to_stake, c.duration, &cfg).unwrap();
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.eclipastro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: cfg.single_sided_staking.to_string(),
                amount: eclipastro_amount_to_stake,
                msg: to_json_binary(&SingleSidedCw20HookMsg::Stake {
                    lock_duration: c.duration,
                    recipient: None,
                })?,
            })?,
            funds: vec![],
        }));
        response = response
            .add_attribute("action", "lock to single sided staking vault")
            .add_attribute("token", cfg.eclipastro_token.to_string())
            .add_attribute("amount", eclipastro_amount_to_stake)
            .add_attribute("duration", c.duration.to_string());
    }
    SINGLE_LOCKUP_STATE.save(deps.storage, &state)?;
    Ok(response.add_messages(msgs))
}

// deposit eclipASTRO/xASTRO to Astroport pool
// save xASTRO/eclipASTRO rate
fn handle_deposit_into_pool(
    deps: DepsMut,
    env: Env,
    prev_eclipastro_balance: Uint128,
    xastro_amount: Uint128,
    weighted_amount: Uint128,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut state = LP_LOCKUP_STATE.load(deps.storage)?;
    let current_eclipastro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &cfg.eclipastro_token,
        &Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;
    let eclipastro_amount_to_deposit = current_eclipastro_balance
        .balance
        .checked_sub(prev_eclipastro_balance)
        .unwrap();
    let xastro_amount_to_deposit = xastro_amount / Uint128::from(2u128);
    ensure!(
        eclipastro_amount_to_deposit.gt(&Uint128::zero())
            && xastro_amount_to_deposit.gt(&Uint128::zero()),
        ContractError::InvalidTokenBalance {}
    );
    state.total_xastro = xastro_amount;
    state.weighted_total_xastro = weighted_amount;
    LP_LOCKUP_STATE.save(deps.storage, &state)?;
    let msgs = vec![
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.eclipastro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                spender: cfg.liquidity_pool.to_string(),
                amount: eclipastro_amount_to_deposit,
                expires: None,
            })?,
            funds: vec![],
        }),
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.liquidity_pool.to_string(),
            msg: to_json_binary(&PairExecuteMsg::ProvideLiquidity {
                assets: vec![
                    Asset {
                        info: AssetInfo::Token {
                            contract_addr: cfg.eclipastro_token.clone(),
                        },
                        amount: eclipastro_amount_to_deposit,
                    },
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: cfg.xastro_token.clone(),
                        },
                        amount: xastro_amount_to_deposit,
                    },
                ],
                slippage_tolerance: None,
                auto_stake: Some(false),
                receiver: None,
            })?,
            funds: vec![coin(
                xastro_amount_to_deposit.u128(),
                cfg.xastro_token.clone(),
            )],
        }),
    ];
    Ok(Response::new()
        .add_attribute("action", "convert xASTRO to eclipASTRO")
        .add_attribute("from", cfg.xastro_token.to_string())
        .add_attribute("amount", eclipastro_amount_to_deposit)
        .add_attribute("to", cfg.eclipastro_token.to_string())
        .add_attribute("amount", eclipastro_amount_to_deposit)
        .add_attribute(
            "action",
            "deposit eclipASTRO/xASTRO token pair to liquidity pool",
        )
        .add_attribute("token1", cfg.eclipastro_token.to_string())
        .add_attribute("amount", eclipastro_amount_to_deposit)
        .add_attribute("token2", cfg.xastro_token.to_string())
        .add_attribute("amount", eclipastro_amount_to_deposit)
        .add_messages(msgs))
}

fn handle_stake_lp_token(
    deps: DepsMut,
    env: Env,
    prev_lp_token_balance: Uint128,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut state = LP_LOCKUP_STATE.load(deps.storage)?;
    let lp_token_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &cfg.lp_token,
        &Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;
    let lp_token_to_stake = lp_token_balance
        .balance
        .checked_sub(prev_lp_token_balance)
        .unwrap();
    ensure!(
        lp_token_to_stake.gt(&Uint128::zero()),
        ContractError::InvalidLpTokenBalance {}
    );
    state.total_lp_lockdrop = lp_token_to_stake;
    for c in &cfg.lock_configs {
        let mut lockup_info = LP_LOCKUP_INFO
            .load(deps.storage, c.duration)
            .unwrap_or_default();
        lockup_info.total_staked = state
            .total_lp_lockdrop
            .multiply_ratio(lockup_info.xastro_amount_in_lockups, state.total_xastro);
        LP_LOCKUP_INFO.save(deps.storage, c.duration, &lockup_info)?;
        state.weighted_total_lp_lockdrop = state
            .weighted_total_lp_lockdrop
            .checked_add(calculate_weight(
                lockup_info.total_staked,
                c.duration,
                &cfg,
            )?)
            .unwrap();
    }
    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.lp_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Send {
            contract: cfg.lp_staking.to_string(),
            amount: lp_token_to_stake,
            msg: to_json_binary(&LpStakingCw20HookMsg::Stake {})?,
        })?,
        funds: vec![],
    });
    LP_LOCKUP_STATE.save(deps.storage, &state)?;
    Ok(Response::new()
        .add_attribute("action", "stake lp token")
        .add_attribute("token", cfg.lp_token.to_string())
        .add_attribute("amount", lp_token_to_stake)
        .add_message(msg))
}

pub fn _claim_single_sided_rewards(
    deps: DepsMut,
    env: Env,
    sender: String,
    duration: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    ensure_eq!(
        cfg.claims_allowed,
        true,
        ContractError::ClaimRewardNotAllowed {}
    );

    let mut user_lockup_info = SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&sender, duration))?;
    if user_lockup_info.total_beclip_incentives == Uint128::zero() {
        user_lockup_info.total_beclip_incentives =
            calculate_user_beclip_incentives_for_single_lockup(
                deps.as_ref(),
                sender.clone(),
                duration,
            )?;
    }

    let pending_beclip_incentives = calculate_single_staking_beclip_incentives(
        deps.as_ref(),
        env.block.time.seconds(),
        user_lockup_info.clone(),
    )?;
    user_lockup_info.claimed_beclip_incentives += pending_beclip_incentives;

    let single_staking_rewards =
        calculate_single_sided_total_rewards(deps.as_ref(), env.contract.address.to_string())?;

    let mut msgs = vec![];

    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.single_sided_staking.clone().to_string(),
        msg: to_json_binary(&SingleSidedExecuteMsg::ClaimAll {
            with_flexible: true,
        })?,
        funds: vec![],
    }));

    for rewards_by_duration in single_staking_rewards {
        let updated_reward_weights =
            calculate_updated_single_staking_reward_weights(deps.as_ref(), &rewards_by_duration)?;
        SINGLE_STAKING_REWARD_WEIGHTS.save(
            deps.storage,
            rewards_by_duration.duration,
            &updated_reward_weights,
        )?;

        if rewards_by_duration.duration == duration {
            let user_rewards = calculate_single_staking_user_rewards(
                deps.as_ref(),
                updated_reward_weights.clone(),
                pending_beclip_incentives,
                user_lockup_info.clone(),
            )?;
            user_lockup_info.reward_weights = updated_reward_weights;
            if !user_rewards.eclipastro.is_zero() {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.eclipastro_token.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: sender.clone(),
                        amount: user_rewards.eclipastro,
                    })?,
                    funds: vec![],
                }))
            }
            if !user_rewards.beclip.is_zero() {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.beclip.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: sender.clone(),
                        amount: user_rewards.beclip,
                    })?,
                    funds: vec![],
                }))
            }
        }
    }
    SINGLE_USER_LOCKUP_INFO.save(deps.storage, (&sender, duration), &user_lockup_info)?;

    Ok(Response::new().add_messages(msgs))
}

pub fn _claim_lp_rewards(
    deps: DepsMut,
    env: Env,
    sender: String,
    duration: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    ensure_eq!(
        cfg.claims_allowed,
        true,
        ContractError::ClaimRewardNotAllowed {}
    );

    let mut user_lockup_info = LP_USER_LOCKUP_INFO.load(deps.storage, (&sender, duration))?;
    if user_lockup_info.total_beclip_incentives == Uint128::zero() {
        user_lockup_info.total_beclip_incentives = calculate_user_beclip_incentives_for_lp_lockup(
            deps.as_ref(),
            sender.clone(),
            duration,
        )?;
    }

    let pending_beclip_incentives = calculate_lp_staking_beclip_incentives(
        deps.as_ref(),
        env.block.time.seconds(),
        user_lockup_info.clone(),
    )?;

    user_lockup_info.claimed_beclip_incentives += pending_beclip_incentives;

    let lp_staking_rewards =
        calculate_lp_total_rewards(deps.as_ref(), env.contract.address.to_string())?;

    let mut msgs = vec![];

    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.lp_staking.clone().to_string(),
        msg: to_json_binary(&LpExecuteMsg::Claim {})?,
        funds: vec![],
    }));

    let updated_lp_reward_weights =
        calculate_updated_lp_reward_weights(deps.as_ref(), &lp_staking_rewards)?;
    LP_STAKING_REWARD_WEIGHTS.save(deps.storage, &updated_lp_reward_weights)?;
    let user_rewards = calculate_lp_staking_user_rewards(
        deps.as_ref(),
        updated_lp_reward_weights.clone(),
        pending_beclip_incentives,
        user_lockup_info.clone(),
    )?;
    user_lockup_info.reward_weights = updated_lp_reward_weights;

    if !user_rewards.astro.is_zero() {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: sender.clone(),
            amount: vec![coin(user_rewards.astro.u128(), cfg.astro_token)],
        }));
    }
    if !user_rewards.beclip.is_zero() {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.beclip.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender.clone(),
                amount: user_rewards.beclip,
            })?,
            funds: vec![],
        }))
    }

    LP_USER_LOCKUP_INFO.save(deps.storage, (&sender, duration), &user_lockup_info)?;

    Ok(Response::new().add_messages(msgs))
}

pub fn _claim_all_single_sided_rewards(
    deps: DepsMut,
    env: Env,
    sender: String,
    durations: Option<Vec<u64>>,
    with_flexible: bool,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    ensure_eq!(
        cfg.claims_allowed,
        true,
        ContractError::ClaimRewardNotAllowed {}
    );

    let single_staking_rewards =
        calculate_single_sided_total_rewards(deps.as_ref(), env.contract.address.to_string())?;

    let mut msgs = vec![];

    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.single_sided_staking.clone().to_string(),
        msg: to_json_binary(&SingleSidedExecuteMsg::ClaimAll {
            with_flexible: true,
        })?,
        funds: vec![],
    }));

    let mut eclipastro_rewards = Uint128::zero();
    let mut beclip_rewards = Uint128::zero();

    for rewards_by_duration in single_staking_rewards {
        let duration = rewards_by_duration.duration;
        if !with_flexible && duration == 0 {
            continue;
        }
        if let Some(ref durations) = durations {
            if !durations.iter().any(|d| d == &duration) {
                continue;
            }
        }
        let updated_reward_weights =
            calculate_updated_single_staking_reward_weights(deps.as_ref(), &rewards_by_duration)?;
        SINGLE_STAKING_REWARD_WEIGHTS.save(
            deps.storage,
            rewards_by_duration.duration,
            &updated_reward_weights,
        )?;

        let mut user_lockup_info = SINGLE_USER_LOCKUP_INFO
            .load(deps.storage, (&sender, duration))
            .unwrap_or_default();
        if user_lockup_info.total_eclipastro_staked - user_lockup_info.total_eclipastro_withdrawed
            == Uint128::zero()
        {
            continue;
        }
        if user_lockup_info.total_beclip_incentives == Uint128::zero() {
            user_lockup_info.total_beclip_incentives =
                calculate_user_beclip_incentives_for_single_lockup(
                    deps.as_ref(),
                    sender.clone(),
                    duration,
                )?;
        }
        let pending_beclip_incentives = calculate_single_staking_beclip_incentives(
            deps.as_ref(),
            env.block.time.seconds(),
            user_lockup_info.clone(),
        )?;
        user_lockup_info.claimed_beclip_incentives += pending_beclip_incentives;

        let user_rewards = calculate_single_staking_user_rewards(
            deps.as_ref(),
            updated_reward_weights.clone(),
            pending_beclip_incentives,
            user_lockup_info.clone(),
        )?;
        eclipastro_rewards += user_rewards.eclipastro;
        beclip_rewards += user_rewards.beclip;
        user_lockup_info.reward_weights = updated_reward_weights;
        SINGLE_USER_LOCKUP_INFO.save(deps.storage, (&sender, duration), &user_lockup_info)?;
    }
    if !eclipastro_rewards.is_zero() {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.eclipastro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender.clone(),
                amount: eclipastro_rewards,
            })?,
            funds: vec![],
        }))
    }
    if !beclip_rewards.is_zero() {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.beclip.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender,
                amount: beclip_rewards,
            })?,
            funds: vec![],
        }))
    }

    Ok(Response::new().add_messages(msgs))
}

pub fn _claim_all_lp_rewards(
    deps: DepsMut,
    env: Env,
    sender: String,
    durations: Option<Vec<u64>>,
    with_flexible: bool,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    ensure_eq!(
        cfg.claims_allowed,
        true,
        ContractError::ClaimRewardNotAllowed {}
    );

    let lp_staking_rewards =
        calculate_lp_total_rewards(deps.as_ref(), env.contract.address.to_string())?;

    let updated_lp_reward_weights =
        calculate_updated_lp_reward_weights(deps.as_ref(), &lp_staking_rewards)?;
    LP_STAKING_REWARD_WEIGHTS.save(deps.storage, &updated_lp_reward_weights)?;

    let mut msgs = vec![];

    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.lp_staking.clone().to_string(),
        msg: to_json_binary(&LpExecuteMsg::Claim {})?,
        funds: vec![],
    }));

    let mut astro_rewards = Uint128::zero();
    let mut beclip_rewards = Uint128::zero();

    for lock_config in cfg.lock_configs {
        let duration = lock_config.duration;
        if !with_flexible && duration == 0 {
            continue;
        }
        if let Some(ref durations) = durations {
            if !durations.iter().any(|d| d == &duration) {
                continue;
            }
        }
        let mut user_lockup_info = LP_USER_LOCKUP_INFO
            .load(deps.storage, (&sender, duration))
            .unwrap_or_default();
        if user_lockup_info.total_lp_staked - user_lockup_info.total_lp_withdrawed
            == Uint128::zero()
        {
            continue;
        }
        if user_lockup_info.total_beclip_incentives == Uint128::zero() {
            user_lockup_info.total_beclip_incentives =
                calculate_user_beclip_incentives_for_lp_lockup(
                    deps.as_ref(),
                    sender.clone(),
                    duration,
                )?;
        }

        let pending_beclip_incentives = calculate_lp_staking_beclip_incentives(
            deps.as_ref(),
            env.block.time.seconds(),
            user_lockup_info.clone(),
        )?;
        user_lockup_info.claimed_beclip_incentives += pending_beclip_incentives;
        let user_rewards = calculate_lp_staking_user_rewards(
            deps.as_ref(),
            updated_lp_reward_weights.clone(),
            pending_beclip_incentives,
            user_lockup_info.clone(),
        )?;
        user_lockup_info.reward_weights = updated_lp_reward_weights.clone();
        astro_rewards += user_rewards.astro;
        beclip_rewards += user_rewards.beclip;

        LP_USER_LOCKUP_INFO.save(deps.storage, (&sender, duration), &user_lockup_info)?;
    }
    if !astro_rewards.is_zero() {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: sender.clone(),
            amount: vec![coin(astro_rewards.u128(), cfg.astro_token)],
        }));
    }
    if !beclip_rewards.is_zero() {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.beclip.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender,
                amount: beclip_rewards,
            })?,
            funds: vec![],
        }))
    }
    Ok(Response::new().add_messages(msgs))
}

pub fn try_unlock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stake_type: StakeType,
    duration: u64,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    match stake_type {
        StakeType::SingleStaking => {
            _unlock_single_lockup(deps, env, info, sender, duration, amount)
        }
        StakeType::LpStaking => _unlock_lp_lockup(deps, env, info, sender, duration, amount),
    }
}

pub fn _unlock_single_lockup(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    sender: String,
    duration: u64,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let current_time = env.block.time.seconds();
    let cfg = CONFIG.load(deps.storage)?;
    let mut lockup_info = SINGLE_LOCKUP_INFO.load(deps.storage, duration)?;
    let mut user_lockup_info = SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&sender, duration))?;
    if current_time < cfg.init_timestamp + cfg.deposit_window + cfg.withdrawal_window {
        let mut withdraw_amount = calculate_max_withdrawal_amount_allowed(
            current_time,
            &cfg,
            user_lockup_info.xastro_amount_in_lockups,
        );
        if let Some(amount) = amount {
            ensure!(
                withdraw_amount.ge(&amount),
                ContractError::WithdrawLimitExceed(withdraw_amount.to_string())
            );
            withdraw_amount = amount;
        }
        if current_time > cfg.init_timestamp + cfg.deposit_window {
            ensure!(
                !user_lockup_info.withdrawal_flag,
                ContractError::AlreadyWithdrawed {}
            );
            user_lockup_info.withdrawal_flag = true;
        }
        user_lockup_info.xastro_amount_in_lockups -= withdraw_amount;
        lockup_info.xastro_amount_in_lockups -= withdraw_amount;

        // COSMOS_MSG ::TRANSFER WITHDRAWN tokens
        let msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: sender.clone(),
            amount: vec![coin(withdraw_amount.u128(), cfg.xastro_token)],
        });

        SINGLE_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
        SINGLE_USER_LOCKUP_INFO.save(deps.storage, (&sender, duration), &user_lockup_info)?;

        Ok(Response::new().add_message(msg))
    } else {
        ensure!(cfg.claims_allowed, ContractError::ClaimRewardNotAllowed {});
        let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
        if user_lockup_info.total_eclipastro_staked.is_zero() && !user_lockup_info.withdrawal_flag {
            user_lockup_info.total_eclipastro_staked = user_lockup_info
                .xastro_amount_in_lockups
                .multiply_ratio(state.total_eclipastro_lockup, state.total_xastro);
        }
        let mut withdraw_amount =
            user_lockup_info.total_eclipastro_staked - user_lockup_info.total_eclipastro_withdrawed;
        if let Some(amount) = amount {
            ensure!(
                withdraw_amount.ge(&amount),
                ContractError::WithdrawLimitExceed(withdraw_amount.to_string())
            );
            withdraw_amount = amount;
        }
        user_lockup_info.total_eclipastro_withdrawed += withdraw_amount;
        lockup_info.total_withdrawed += withdraw_amount;

        let mut msgs = vec![];

        if duration == 0 {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.single_sided_staking.to_string(),
                msg: to_json_binary(&SingleSidedExecuteMsg::Unstake {
                    duration,
                    locked_at: None,
                    amount: Some(withdraw_amount),
                    recipient: Some(sender.clone()),
                })?,
                funds: vec![],
            }));
        } else {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.single_sided_staking.to_string(),
                msg: to_json_binary(&SingleSidedExecuteMsg::Unstake {
                    duration,
                    locked_at: Some(cfg.countdown_start_at),
                    amount: Some(withdraw_amount),
                    recipient: Some(sender.clone().clone()),
                })?,
                funds: vec![],
            }));
        }

        SINGLE_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
        SINGLE_USER_LOCKUP_INFO.save(deps.storage, (&sender, duration), &user_lockup_info)?;

        Ok(Response::new().add_messages(msgs))
    }
}

pub fn _unlock_lp_lockup(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    sender: String,
    duration: u64,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let current_time = env.block.time.seconds();
    let cfg = CONFIG.load(deps.storage)?;
    let mut lockup_info = LP_LOCKUP_INFO.load(deps.storage, duration)?;
    let mut user_lockup_info = LP_USER_LOCKUP_INFO.load(deps.storage, (&sender, duration))?;
    if current_time < cfg.init_timestamp + cfg.deposit_window + cfg.withdrawal_window {
        let mut withdraw_amount = calculate_max_withdrawal_amount_allowed(
            current_time,
            &cfg,
            user_lockup_info.xastro_amount_in_lockups,
        );
        if let Some(amount) = amount {
            ensure!(
                withdraw_amount.ge(&amount),
                ContractError::WithdrawLimitExceed(withdraw_amount.to_string())
            );
            withdraw_amount = amount;
        }
        if current_time > cfg.init_timestamp + cfg.deposit_window {
            ensure!(
                !user_lockup_info.withdrawal_flag,
                ContractError::AlreadyWithdrawed {}
            );
            user_lockup_info.withdrawal_flag = true;
        }
        user_lockup_info.xastro_amount_in_lockups -= withdraw_amount;
        lockup_info.xastro_amount_in_lockups -= withdraw_amount;

        // COSMOS_MSG ::TRANSFER WITHDRAWN tokens
        let msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: sender.clone(),
            amount: vec![coin(withdraw_amount.u128(), cfg.xastro_token)],
        });

        LP_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
        LP_USER_LOCKUP_INFO.save(deps.storage, (&sender, duration), &user_lockup_info)?;

        Ok(Response::new().add_message(msg))
    } else {
        ensure!(cfg.claims_allowed, ContractError::ClaimRewardNotAllowed {});
        let state = LP_LOCKUP_STATE.load(deps.storage)?;
        if user_lockup_info.total_lp_staked.is_zero() && !user_lockup_info.withdrawal_flag {
            user_lockup_info.total_lp_staked = user_lockup_info
                .xastro_amount_in_lockups
                .multiply_ratio(state.total_lp_lockdrop, state.total_xastro);
        }
        let mut withdraw_amount =
            user_lockup_info.total_lp_staked - user_lockup_info.total_lp_withdrawed;
        if let Some(amount) = amount {
            ensure!(
                withdraw_amount.ge(&amount),
                ContractError::WithdrawLimitExceed(withdraw_amount.to_string())
            );
            withdraw_amount = amount;
        }
        user_lockup_info.total_lp_withdrawed += withdraw_amount;
        lockup_info.total_withdrawed += withdraw_amount;

        let mut penalty_amount = Uint128::zero();

        if current_time < cfg.countdown_start_at + duration {
            penalty_amount = withdraw_amount.checked_div_ceil((2u128, 1u128)).unwrap();
        }
        let mut msgs = vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.lp_staking.to_string(),
                msg: to_json_binary(&LpExecuteMsg::Unstake {
                    amount: withdraw_amount,
                    recipient: None,
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.lp_token.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: sender.clone(),
                    amount: withdraw_amount - penalty_amount,
                })?,
                funds: vec![],
            }),
        ];
        if !penalty_amount.is_zero() {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.lp_token.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: cfg.dao_treasury_address.to_string(),
                    amount: penalty_amount,
                })?,
                funds: vec![],
            }));
        }

        LP_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
        LP_USER_LOCKUP_INFO.save(deps.storage, (&sender, duration), &user_lockup_info)?;

        Ok(Response::new().add_messages(msgs))
    }
}

pub fn astro_convert_msg(astro_staking: String, coin: &Coin) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: astro_staking,
        msg: to_json_binary(&AstroStakingExecuteMsg::Enter { receiver: None })?,
        funds: vec![coin.clone()],
    }))
}

pub fn convert_eclipastro_msg(token_converter: String, coin: &Coin) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_converter,
        msg: to_json_binary(&ConverterExecuteMsg::Convert { recipient: None })?,
        funds: vec![coin.clone()],
    }))
}
