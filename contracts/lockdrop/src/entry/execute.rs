use astroport::{
    asset::{Asset, AssetInfo},
    pair::ExecuteMsg as PairExecuteMsg,
    staking::Cw20HookMsg as AstroportStakingCw20HookMsg,
    token::BalanceResponse,
};
use cosmwasm_std::{
    attr, coins, ensure, ensure_eq, ensure_ne, from_json, to_json_binary, Addr, BankMsg, CosmosMsg,
    Decimal, Deps, DepsMut, Env, MessageInfo, Order, Response, Uint128, Uint256, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, Cw20ReceiveMsg};
use equinox_msg::{
    flexible_staking::{
        Cw20HookMsg as FlexibleStakingCw20HookMsg, ExecuteMsg as FlexibleStakingExecuteMsg,
        QueryMsg as FlexibleStakingQueryMsg,
    },
    lockdrop::{AssetRewardWeight, CallbackMsg, Cw20HookMsg, StakeType, UpdateConfigMsg},
    lp_staking::{Cw20HookMsg as LpStakingCw20HookMsg, ExecuteMsg as LpStakingExecuteMsg},
    reward_distributor::{
        Config as RewardDistributorConfig, FlexibleReward, QueryMsg as RewardDistributorQueryMsg,
        TimelockReward,
    },
    timelock_staking::{
        Cw20HookMsg as TimelockStakingCw20HookMsg, ExecuteMsg as TimelockStakingExecuteMsg,
        QueryMsg as TimelockStakingQueryMsg,
    },
    token_converter::Cw20HookMsg as ConverterCw20HookMsg,
};

use crate::{
    error::ContractError,
    math::{
        calculate_eclipastro_amount_for_lp, calculate_eclipastro_staked,
        calculate_max_withdrawal_percent_allowed, calculate_weight,
    },
    state::{
        CONFIG, LP_LOCKUP_INFO, LP_LOCKUP_STATE, LP_USER_LOCKUP_INFO, OWNER, SINGLE_LOCKUP_INFO,
        SINGLE_LOCKUP_STATE, SINGLE_USER_LOCKUP_INFO,
    },
};

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let user_address = deps.api.addr_validate(&cw20_msg.sender)?;
    let amount = cw20_msg.amount;

    // CHECK :: Tokens sent > 0
    ensure!(amount.gt(&Uint128::zero()), ContractError::ZeroAmount {});

    match from_json(&cw20_msg.msg)? {
        Cw20HookMsg::IncreaseLockup {
            stake_type,
            duration,
        } => handle_increase_lockup(deps, env, info, stake_type, user_address, duration, amount),
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
        CallbackMsg::DistributeSingleStakingAssetRewards {
            prev_eclip_balance,
            prev_eclipastro_balance,
            user_address,
            recipient,
            duration,
        } => handle_distribute_single_staking_asset_rewards(
            deps,
            env,
            prev_eclip_balance,
            prev_eclipastro_balance,
            user_address,
            recipient,
            duration,
        ),
        CallbackMsg::DistributeLpStakingAssetRewards {
            prev_eclip_balance,
            prev_astro_balance,
            user_address,
            recipient,
            duration,
        } => handle_lp_stake_distribute_asset_reward(
            deps,
            env,
            prev_eclip_balance,
            prev_astro_balance,
            user_address,
            recipient,
            duration,
        ),
        CallbackMsg::StakeToSingleVault {
            prev_eclipastro_balance,
            astro_balance_to_convert,
            xastro_balance_to_convert,
        } => handle_stake_to_single_vault(
            deps,
            env,
            prev_eclipastro_balance,
            astro_balance_to_convert,
            xastro_balance_to_convert,
        ),
        CallbackMsg::DepositIntoPool {
            prev_eclipastro_balance,
            prev_xastro_balance,
            astro_balance_for_eclipastro,
            astro_balance_for_xastro,
            xastro_balance_for_eclipastro,
        } => handle_deposit_into_pool(
            deps,
            env,
            prev_eclipastro_balance,
            prev_xastro_balance,
            astro_balance_for_eclipastro,
            astro_balance_for_xastro,
            xastro_balance_for_eclipastro,
        ),
        CallbackMsg::StakeLpToken {
            prev_lp_token_balance,
        } => handle_stake_lp_token(deps, env, prev_lp_token_balance),
        CallbackMsg::ClaimSingleStakingAssetRewards {
            user_address,
            recipient,
            duration,
        } => {
            handle_claim_single_staking_asset_rewards(deps, env, user_address, recipient, duration)
        }
        CallbackMsg::ClaimLpStakingAssetRewards {
            user_address,
            recipient,
            duration,
        } => handle_claim_lp_staking_asset_rewards(deps, env, user_address, recipient, duration),
        CallbackMsg::UnlockSingleLockup {
            user_address,
            duration,
        } => handle_unlock_single_lockup(deps, env, info, user_address, duration),
        CallbackMsg::UnlockLpLockup {
            user_address,
            duration,
        } => handle_unlock_lp_lockup(deps, env, info, user_address, duration),
    }
}

/// users lockup assets
/// lockup is only allowed in deposit window
/// ASTRO/xASTRO tokens are allowed
pub fn handle_increase_lockup(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stake_type: StakeType,
    user_address: Addr,
    duration: u64,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let staking_token = info.sender;

    let current_time = env.block.time.seconds();
    ensure!(
        current_time >= cfg.init_timestamp,
        ContractError::DepositWindowNotStarted {}
    );
    ensure!(
        current_time <= cfg.init_timestamp + cfg.deposit_window,
        ContractError::DepositWindowClosed {}
    );
    ensure_ne!(
        cfg.lock_configs.iter().find(|c| c.duration == duration),
        None,
        ContractError::InvalidDuration(duration)
    );

    match stake_type {
        StakeType::SingleStaking => {
            let mut lockup_info = SINGLE_LOCKUP_INFO
                .load(deps.storage, duration)
                .unwrap_or_default();
            let mut user_lockup_info = SINGLE_USER_LOCKUP_INFO
                .load(deps.storage, (&user_address, duration))
                .unwrap_or_default();
            if staking_token == cfg.astro_token {
                lockup_info.astro_amount_in_lockups = lockup_info
                    .astro_amount_in_lockups
                    .checked_add(amount)
                    .unwrap();
                user_lockup_info.astro_amount_in_lockups = user_lockup_info
                    .astro_amount_in_lockups
                    .checked_add(amount)
                    .unwrap();
            } else if staking_token == cfg.xastro_token {
                lockup_info.xastro_amount_in_lockups = lockup_info
                    .xastro_amount_in_lockups
                    .checked_add(amount)
                    .unwrap();
                user_lockup_info.xastro_amount_in_lockups = user_lockup_info
                    .xastro_amount_in_lockups
                    .checked_add(amount)
                    .unwrap();
            } else {
                return Err(ContractError::InvalidLockupAsset(staking_token.to_string()));
            }
            SINGLE_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
            SINGLE_USER_LOCKUP_INFO.save(
                deps.storage,
                (&user_address, duration),
                &user_lockup_info,
            )?;

            Ok(Response::new().add_attributes(vec![
                attr("action", "increase_lockup_position"),
                attr("type", "single staking"),
                attr("from", user_address),
                attr("asset", staking_token),
                attr("amount", amount),
                attr("duration", duration.to_string()),
            ]))
        }
        StakeType::LpStaking => {
            let mut lockup_info = LP_LOCKUP_INFO
                .load(deps.storage, duration)
                .unwrap_or_default();
            let mut user_lockup_info = LP_USER_LOCKUP_INFO
                .load(deps.storage, (&user_address, duration))
                .unwrap_or_default();
            if staking_token == cfg.astro_token {
                lockup_info.astro_amount_in_lockups = lockup_info
                    .astro_amount_in_lockups
                    .checked_add(amount)
                    .unwrap();
                user_lockup_info.astro_amount_in_lockups = user_lockup_info
                    .astro_amount_in_lockups
                    .checked_add(amount)
                    .unwrap();
            } else if staking_token == cfg.xastro_token {
                lockup_info.xastro_amount_in_lockups = lockup_info
                    .xastro_amount_in_lockups
                    .checked_add(amount)
                    .unwrap();
                user_lockup_info.xastro_amount_in_lockups = user_lockup_info
                    .xastro_amount_in_lockups
                    .checked_add(amount)
                    .unwrap();
            } else {
                return Err(ContractError::InvalidLockupAsset(staking_token.to_string()));
            }
            LP_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
            LP_USER_LOCKUP_INFO.save(deps.storage, (&user_address, duration), &user_lockup_info)?;

            Ok(Response::new().add_attributes(vec![
                attr("action", "increase_lockup_position"),
                attr("type", "lp staking"),
                attr("from", user_address),
                attr("asset", staking_token),
                attr("amount", amount),
                attr("duration", duration.to_string()),
            ]))
        }
    }
}

/// stake all the lockup assets to single staking vault
/// staking is only allowed after withdraw window
/// only owner can do this
/// ASTRO/xASTRO will be converted to eclipASTRO and be staked to single staking vault
/// change SINGLE_STATE's is_staked to true
pub fn handle_stake_single_vault(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;

    // check is owner
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    // check time window
    ensure!(
        env.block.time.seconds()
            > (cfg.init_timestamp + cfg.deposit_window + cfg.withdrawal_window),
        ContractError::LockdropNotFinished {}
    );
    // check is already staked
    ensure_eq!(state.is_staked, false, ContractError::AlreadyStaked {});

    // get all single staking lockup assets on this contract
    let (single_lockup_astro_amount, single_lockup_xastro_amount) = SINGLE_LOCKUP_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .fold((Uint128::zero(), Uint128::zero()), |acc, cur| {
            let (_, info) = cur.unwrap();
            (
                acc.0.checked_add(info.astro_amount_in_lockups).unwrap(),
                acc.1.checked_add(info.xastro_amount_in_lockups).unwrap(),
            )
        });

    let mut msgs = vec![
        // convert all the ASTRO to eclipASTRO
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.astro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: cfg.converter.to_string(),
                amount: single_lockup_astro_amount,
                msg: to_json_binary(&ConverterCw20HookMsg::Convert {})?,
            })?,
            funds: vec![],
        }),
        // convert all the xASTRO to eclipASTRO
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.xastro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: cfg.converter.to_string(),
                amount: single_lockup_xastro_amount,
                msg: to_json_binary(&ConverterCw20HookMsg::Convert {})?,
            })?,
            funds: vec![],
        }),
    ];

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
            astro_balance_to_convert: single_lockup_astro_amount,
            xastro_balance_to_convert: single_lockup_xastro_amount,
        }
        .to_cosmos_msg(&env)?,
    );

    Ok(Response::new().add_messages(msgs))
}

/// stake all the lockup assets to lp staking vault
/// staking is only allowed after withdraw window
/// only owner can do this
/// ASTRO/xASTRO will be converted to eclipASTRO/xASTRO(50%/50%) and be deposited to liquidity pool and be staked to lp staking vault
/// change LP_STATE's is_staked to true
pub fn handle_stake_lp_vault(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    // check is owner
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    // check time window
    ensure!(
        env.block.time.seconds()
            > (cfg.init_timestamp + cfg.deposit_window + cfg.withdrawal_window),
        ContractError::LockdropNotFinished {}
    );
    // check is already staked
    let state = LP_LOCKUP_STATE.load(deps.storage)?;
    ensure_eq!(state.is_staked, false, ContractError::AlreadyStaked {});

    // get all lp staking lockup assets on this contract
    let (lp_lockup_astro_amount, lp_lockup_xastro_amount) = LP_LOCKUP_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .fold((Uint128::zero(), Uint128::zero()), |acc, cur| {
            let (_, info) = cur.unwrap();
            (
                acc.0.checked_add(info.astro_amount_in_lockups).unwrap(),
                acc.1.checked_add(info.xastro_amount_in_lockups).unwrap(),
            )
        });

    // check contract balance
    let astro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &cfg.astro_token,
        &Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;
    let xastro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &cfg.xastro_token,
        &Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;
    ensure!(
        astro_balance.balance.ge(&lp_lockup_astro_amount)
            && xastro_balance.balance.ge(&lp_lockup_xastro_amount),
        ContractError::InsufficientAmountInContract {}
    );

    let astro_balance_for_eclipastro = lp_lockup_astro_amount
        .checked_div(Uint128::from(2u128))
        .unwrap();
    let mut msgs = vec![];
    // convert half of the ASTRO to eclipASTRO
    if astro_balance_for_eclipastro.gt(&Uint128::zero()) {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.astro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: cfg.converter.to_string(),
                amount: astro_balance_for_eclipastro,
                msg: to_json_binary(&ConverterCw20HookMsg::Convert {})?,
            })?,
            funds: vec![],
        }));
    }
    let astro_balance_for_xastro = lp_lockup_astro_amount
        .checked_sub(astro_balance_for_eclipastro)
        .unwrap();
    // convert half of the ASTRO to xASTRO at astro_staking contract
    if astro_balance_for_xastro.gt(&Uint128::zero()) {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.astro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: cfg.astro_staking.to_string(),
                amount: astro_balance_for_xastro,
                msg: to_json_binary(&AstroportStakingCw20HookMsg::Enter {})?,
            })?,
            funds: vec![],
        }));
    }
    // convert half of the xASTRO to eclipASTRO
    let xastro_balance_for_eclipastro = lp_lockup_xastro_amount
        .checked_div(Uint128::from(2u128))
        .unwrap();
    if xastro_balance_for_eclipastro.gt(&Uint128::zero()) {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.xastro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: cfg.converter.to_string(),
                amount: xastro_balance_for_eclipastro,
                msg: to_json_binary(&ConverterCw20HookMsg::Convert {})?,
            })?,
            funds: vec![],
        }));
    }

    // callback function to stake eclipASTRO to single staking vaults
    let prev_eclipastro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &cfg.eclipastro_token,
        &Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;
    let prev_xastro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &cfg.xastro_token,
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
            prev_xastro_balance: prev_xastro_balance.balance,
            astro_balance_for_eclipastro,
            astro_balance_for_xastro,
            xastro_balance_for_eclipastro,
        }
        .to_cosmos_msg(&env)?,
    );
    msgs.push(
        CallbackMsg::StakeLpToken {
            prev_lp_token_balance: prev_lp_token_balance.balance,
        }
        .to_cosmos_msg(&env)?,
    );

    Ok(Response::new().add_messages(msgs))
}

pub fn handle_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_cfg: UpdateConfigMsg,
) -> Result<Response, ContractError> {
    let mut cfg = CONFIG.load(deps.storage)?;
    let mut attributes = vec![attr("action", "update_config")];

    OWNER.assert_admin(deps.as_ref(), &info.sender)?;

    if let Some(flexible_staking) = new_cfg.flexible_staking {
        if cfg.flexible_staking.is_some() {
            return Err(ContractError::AlreadySet(flexible_staking));
        }

        cfg.flexible_staking = Some(deps.api.addr_validate(&flexible_staking)?);
        attributes.push(attr("new_flexible_staking", &flexible_staking))
    };

    if let Some(timelock_staking) = new_cfg.timelock_staking {
        if cfg.timelock_staking.is_some() {
            return Err(ContractError::AlreadySet(timelock_staking));
        }

        cfg.timelock_staking = Some(deps.api.addr_validate(&timelock_staking)?);
        attributes.push(attr("new_timelock_staking", &timelock_staking))
    };

    if let Some(lp_staking) = new_cfg.lp_staking {
        if cfg.lp_staking.is_some() {
            return Err(ContractError::AlreadySet(lp_staking));
        }

        cfg.lp_staking = Some(deps.api.addr_validate(&lp_staking)?);
        attributes.push(attr("new_timelock_staking", &lp_staking))
    };

    if let Some(reward_distributor) = new_cfg.reward_distributor {
        if cfg.reward_distributor.is_some() {
            return Err(ContractError::AlreadySet(reward_distributor));
        }

        cfg.reward_distributor = Some(deps.api.addr_validate(&reward_distributor)?);
        attributes.push(attr("new_reward_distributor", &reward_distributor))
    };

    CONFIG.save(deps.storage, &cfg)?;
    Ok(Response::new().add_attributes(attributes))
}

pub fn handle_enable_claims(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut single_state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let mut lp_state = LP_LOCKUP_STATE.load(deps.storage)?;

    OWNER.assert_admin(deps.as_ref(), &info.sender)?;

    // CHECK :: Have the deposit / withdraw windows concluded
    ensure!(
        env.block.time.seconds()
            > (cfg.init_timestamp + cfg.deposit_window + cfg.withdrawal_window),
        ContractError::LockdropNotFinished {}
    );

    ensure!(
        single_state.is_staked && lp_state.is_staked,
        ContractError::NotStaked {}
    );

    // CHECK ::: Claims are only enabled once
    ensure!(
        !single_state.are_claims_allowed || !lp_state.are_claims_allowed,
        ContractError::AlreadyAllowed {}
    );
    single_state.are_claims_allowed = true;
    single_state.countdown_start_at = env.block.time.seconds();
    lp_state.are_claims_allowed = true;
    lp_state.countdown_start_at = env.block.time.seconds();

    SINGLE_LOCKUP_STATE.save(deps.storage, &single_state)?;
    LP_LOCKUP_STATE.save(deps.storage, &lp_state)?;
    Ok(Response::new().add_attribute("action", "allow_claims"))
}

// can withdraw freely at deposit window
// can withdraw only once at withdraw window
// withdraw amount is limited at withdraw window
pub fn handle_single_locking_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    assets: Option<Vec<Asset>>,
    duration: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    // Check :: Amount should be within the allowed withdrawal limit bounds
    let max_withdrawal_percent =
        calculate_max_withdrawal_percent_allowed(env.block.time.seconds(), &cfg);
    let user_address = info.sender;
    let mut user_lockup_info =
        SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&user_address, duration))?;
    let mut lockup_info = SINGLE_LOCKUP_INFO.load(deps.storage, duration)?;

    ensure_eq!(
        user_lockup_info.withdrawal_flag,
        false,
        ContractError::AlreadyWithdrawed {}
    );

    if let Some(assets) = assets.clone() {
        for asset in assets {
            ensure!(
                asset.amount.gt(&Uint128::zero()),
                ContractError::ZeroAmount {}
            );
        }
    }

    let mut msgs = vec![];
    let mut response = Response::new().add_attribute("action", "withdraw_from_lockup");
    let assets = assets.unwrap_or(vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: cfg.astro_token.clone(),
            },
            amount: user_lockup_info.astro_amount_in_lockups * max_withdrawal_percent,
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: cfg.xastro_token.clone(),
            },
            amount: user_lockup_info.xastro_amount_in_lockups * max_withdrawal_percent,
        },
    ]);

    for asset in assets {
        if asset.amount.eq(&Uint128::zero()) {
            continue;
        }
        ensure!(
            asset.info.equal(&AssetInfo::Token {
                contract_addr: cfg.astro_token.clone()
            }) || asset.info.equal(&AssetInfo::Token {
                contract_addr: cfg.xastro_token.clone()
            }),
            ContractError::InvalidLockupAsset(asset.to_string())
        );

        let max_withdrawal_allowed = if asset.info.equal(&AssetInfo::Token {
            contract_addr: cfg.astro_token.clone(),
        }) {
            user_lockup_info.astro_amount_in_lockups * max_withdrawal_percent
        } else {
            user_lockup_info.xastro_amount_in_lockups * max_withdrawal_percent
        };
        ensure!(
            asset.amount.le(&max_withdrawal_allowed),
            ContractError::WithdrawLimitExceed(max_withdrawal_allowed.to_string())
        );
        if asset.info.equal(&AssetInfo::Token {
            contract_addr: cfg.astro_token.clone(),
        }) {
            user_lockup_info.astro_amount_in_lockups = user_lockup_info
                .astro_amount_in_lockups
                .checked_sub(asset.amount)
                .unwrap();
            lockup_info.astro_amount_in_lockups = lockup_info
                .astro_amount_in_lockups
                .checked_sub(asset.amount)
                .unwrap();
        } else {
            user_lockup_info.xastro_amount_in_lockups = user_lockup_info
                .xastro_amount_in_lockups
                .checked_sub(asset.amount)
                .unwrap();
            lockup_info.xastro_amount_in_lockups = lockup_info
                .xastro_amount_in_lockups
                .checked_sub(asset.amount)
                .unwrap();
        }

        // COSMOS_MSG ::TRANSFER WITHDRAWN tokens
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: asset.info.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: user_address.to_string(),
                amount: asset.amount,
            })?,
            funds: vec![],
        }));

        response = response
            .add_attribute("token", asset.info.to_string())
            .add_attribute("user_address", user_address.to_string())
            .add_attribute("duration", duration.to_string())
            .add_attribute("amount", asset.amount);
    }

    // Update withdrawal flag after the deposit window
    if env.block.time.seconds() >= cfg.init_timestamp + cfg.deposit_window {
        user_lockup_info.withdrawal_flag = true;
    }

    SINGLE_USER_LOCKUP_INFO.save(deps.storage, (&user_address, duration), &user_lockup_info)?;
    SINGLE_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;

    Ok(response.add_messages(msgs))
}

// can withdraw freely at deposit window
// can withdraw only once at withdraw window
// withdraw amount is limited at withdraw window
pub fn handle_lp_locking_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    assets: Option<Vec<Asset>>,
    duration: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    // Check :: Amount should be within the allowed withdrawal limit bounds
    let max_withdrawal_percent =
        calculate_max_withdrawal_percent_allowed(env.block.time.seconds(), &cfg);
    let user_address = info.sender;
    let mut user_lockup_info = LP_USER_LOCKUP_INFO.load(deps.storage, (&user_address, duration))?;
    let mut lockup_info = LP_LOCKUP_INFO.load(deps.storage, duration)?;

    ensure_eq!(
        user_lockup_info.withdrawal_flag,
        false,
        ContractError::AlreadyWithdrawed {}
    );

    let mut msgs = vec![];
    let mut response = Response::new().add_attribute("action", "withdraw_from_lockup");
    let assets = assets.unwrap_or(vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: cfg.astro_token.clone(),
            },
            amount: user_lockup_info.astro_amount_in_lockups * max_withdrawal_percent,
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: cfg.xastro_token.clone(),
            },
            amount: user_lockup_info.xastro_amount_in_lockups * max_withdrawal_percent,
        },
    ]);
    for asset in assets {
        ensure!(
            asset.amount.gt(&Uint128::zero()),
            ContractError::ZeroAmount {}
        );
        ensure!(
            asset.info.equal(&AssetInfo::Token {
                contract_addr: cfg.astro_token.clone()
            }) || asset.info.equal(&AssetInfo::Token {
                contract_addr: cfg.xastro_token.clone()
            }),
            ContractError::InvalidLockupAsset(asset.to_string())
        );
        let max_withdrawal_allowed = if asset.info.equal(&AssetInfo::Token {
            contract_addr: cfg.astro_token.clone(),
        }) {
            user_lockup_info.astro_amount_in_lockups * max_withdrawal_percent
        } else {
            user_lockup_info.xastro_amount_in_lockups * max_withdrawal_percent
        };
        ensure!(
            asset.amount.le(&max_withdrawal_allowed),
            ContractError::WithdrawLimitExceed(max_withdrawal_allowed.to_string())
        );
        if asset.info.equal(&AssetInfo::Token {
            contract_addr: cfg.astro_token.clone(),
        }) {
            user_lockup_info.astro_amount_in_lockups = user_lockup_info
                .astro_amount_in_lockups
                .checked_sub(asset.amount)
                .unwrap();
            lockup_info.astro_amount_in_lockups = lockup_info
                .astro_amount_in_lockups
                .checked_sub(asset.amount)
                .unwrap();
        } else {
            user_lockup_info.xastro_amount_in_lockups = user_lockup_info
                .xastro_amount_in_lockups
                .checked_sub(asset.amount)
                .unwrap();
            lockup_info.xastro_amount_in_lockups = lockup_info
                .xastro_amount_in_lockups
                .checked_sub(asset.amount)
                .unwrap();
        }

        // COSMOS_MSG ::TRANSFER WITHDRAWN tokens
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: asset.info.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: user_address.to_string(),
                amount: asset.amount,
            })?,
            funds: vec![],
        }));

        response = response
            .add_attribute("token", asset.info.to_string())
            .add_attribute("user_address", user_address.to_string())
            .add_attribute("duration", duration.to_string())
            .add_attribute("amount", asset.amount);
    }

    // Update withdrawal flag after the deposit window
    if env.block.time.seconds() >= cfg.init_timestamp + cfg.deposit_window {
        user_lockup_info.withdrawal_flag = true;
    }

    LP_USER_LOCKUP_INFO.save(deps.storage, (&user_address, duration), &user_lockup_info)?;
    LP_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;

    Ok(response.add_messages(msgs))
}

pub fn handle_increase_eclip_incentives(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    stake_type: StakeType,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let coins = info.funds;
    let mut response = Response::new().add_attribute("action", "increase ECLIP incentives");
    for coin in coins {
        ensure!(
            coin.denom == cfg.eclip,
            ContractError::OnlyEclipAllowed {
                expected: cfg.eclip,
                got: coin.denom
            }
        );
        ensure!(
            coin.amount.gt(&Uint128::zero()),
            ContractError::InvalidTokenBalance {}
        );
        match stake_type {
            StakeType::SingleStaking => {
                let mut single_state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
                single_state.total_eclip_incentives = single_state
                    .total_eclip_incentives
                    .checked_add(coin.amount)
                    .unwrap();
                SINGLE_LOCKUP_STATE.save(deps.storage, &single_state)?;
                response = response
                    .add_attribute("type", "single staking")
                    .add_attribute("amount", coin.amount);
            }
            StakeType::LpStaking => {
                let mut lp_state = LP_LOCKUP_STATE.load(deps.storage)?;
                lp_state.total_eclip_incentives = lp_state
                    .total_eclip_incentives
                    .checked_add(coin.amount)
                    .unwrap();
                LP_LOCKUP_STATE.save(deps.storage, &lp_state)?;
                response = response
                    .add_attribute("type", "lp staking")
                    .add_attribute("amount", coin.amount);
            }
        }
    }
    Ok(response)
}

pub fn handle_claim_single_staking_asset_rewards(
    deps: DepsMut,
    env: Env,
    user_address: Addr,
    recipient: Addr,
    duration: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    ensure_eq!(
        state.are_claims_allowed,
        true,
        ContractError::ClaimRewardNotAllowed {}
    );
    let mut msgs = vec![];

    let flexible_staking = cfg.flexible_staking.unwrap().to_string();
    let timelock_staking = cfg.timelock_staking.unwrap().to_string();

    let flexible_staking_reward_response: FlexibleReward = deps.querier.query_wasm_smart(
        &flexible_staking,
        &FlexibleStakingQueryMsg::Reward {
            user: env.contract.address.to_string(),
        },
    )?;
    if flexible_staking_reward_response.eclip.gt(&Uint128::zero())
        || flexible_staking_reward_response
            .eclipastro
            .gt(&Uint128::zero())
    {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: flexible_staking.clone(),
            msg: to_json_binary(&FlexibleStakingExecuteMsg::Claim {})?,
            funds: vec![],
        }));
    }

    let timelock_staking_reward_response: Vec<TimelockReward> = deps.querier.query_wasm_smart(
        &timelock_staking,
        &TimelockStakingQueryMsg::Reward {
            user: env.contract.address.to_string(),
        },
    )?;

    let timelock_claimable = timelock_staking_reward_response
        .into_iter()
        .any(|r| r.eclip.gt(&Uint128::zero()) || r.eclipastro.gt(&Uint128::zero()));

    if timelock_claimable {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: timelock_staking,
            msg: to_json_binary(&TimelockStakingExecuteMsg::ClaimAll {})?,
            funds: vec![],
        }));
    }

    let eclip_balance = deps
        .querier
        .query_balance(env.contract.address.to_string(), cfg.eclip)
        .unwrap();
    // current eclipASTRO balance of this contract
    let eclipastro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &cfg.eclipastro_token,
        &Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    msgs.push(
        CallbackMsg::DistributeSingleStakingAssetRewards {
            prev_eclip_balance: eclip_balance.amount,
            prev_eclipastro_balance: eclipastro_balance.balance,
            user_address,
            recipient,
            duration,
        }
        .to_cosmos_msg(&env)?,
    );

    Ok(Response::default().add_messages(msgs))
}

pub fn handle_claim_lp_staking_asset_rewards(
    deps: DepsMut,
    env: Env,
    user_address: Addr,
    recipient: Addr,
    duration: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let state = LP_LOCKUP_STATE.load(deps.storage)?;
    ensure_eq!(
        state.are_claims_allowed,
        true,
        ContractError::ClaimRewardNotAllowed {}
    );
    let claim_reward_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.lp_staking.unwrap().to_string(),
        msg: to_json_binary(&LpStakingExecuteMsg::Claim {})?,
        funds: vec![],
    });

    let eclip_balance = deps
        .querier
        .query_balance(env.contract.address.to_string(), cfg.eclip)
        .unwrap();
    // current eclipASTRO balance of this contract
    let astro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &cfg.astro_token,
        &Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    let distribute_callback_msg = CallbackMsg::DistributeLpStakingAssetRewards {
        prev_eclip_balance: eclip_balance.amount,
        prev_astro_balance: astro_balance.balance,
        user_address,
        recipient,
        duration,
    }
    .to_cosmos_msg(&env)?;

    Ok(Response::default().add_messages(vec![claim_reward_msg, distribute_callback_msg]))
}

pub fn handle_claim_rewards_and_unlock_for_single_lockup(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    duration: u64,
    withdraw_lockup: bool,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;

    ensure!(
        state.are_claims_allowed,
        ContractError::ClaimRewardNotAllowed {}
    );

    let user_address = info.sender;

    let mut user_lockup_info =
        SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&user_address.clone(), duration))?;

    if user_lockup_info.total_eclip_incentives == Uint128::zero() {
        user_lockup_info.total_eclip_incentives =
            calculate_user_eclip_incentives_for_single_lockup(
                deps.as_ref(),
                user_address.clone(),
                duration,
            )?;
    }
    let half_amount = user_lockup_info
        .total_eclip_incentives
        .checked_div(Uint128::from(2u128))
        .unwrap();
    let max_allowed_to_claim = half_amount
        .checked_add(half_amount.multiply_ratio(
            env.block.time.seconds() - state.countdown_start_at,
            duration,
        ))
        .unwrap();
    let claimable_amount = max_allowed_to_claim
        .checked_sub(user_lockup_info.claimed_eclip_incentives)
        .unwrap_or_default();
    user_lockup_info.claimed_eclip_incentives = max_allowed_to_claim;

    let mut msgs = vec![];
    let mut response =
        Response::new().add_attribute("action", "claim rewards and unlock single lockup");
    msgs.push(
        CallbackMsg::ClaimSingleStakingAssetRewards {
            user_address: user_address.clone(),
            recipient: user_address.clone(),
            duration,
        }
        .to_cosmos_msg(&env)?,
    );
    if claimable_amount.gt(&Uint128::zero()) {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: user_address.to_string(),
            amount: coins(claimable_amount.into(), cfg.eclip.clone()),
        }));
    }
    response = response
        .add_attribute("asset", cfg.eclip)
        .add_attribute("amount", claimable_amount)
        .add_attribute("to", user_address.clone());

    if withdraw_lockup {
        ensure!(
            !user_lockup_info.unlock_flag,
            ContractError::AlreadyUnlocked {}
        );
        ensure!(
            env.block.time.seconds() - state.countdown_start_at > duration,
            ContractError::WaitToUnlock(
                state.countdown_start_at + duration - env.block.time.seconds()
            )
        );
        msgs.push(
            CallbackMsg::UnlockSingleLockup {
                user_address: user_address.clone(),
                duration,
            }
            .to_cosmos_msg(&env)?,
        );
    }

    SINGLE_USER_LOCKUP_INFO.save(deps.storage, (&user_address, duration), &user_lockup_info)?;
    Ok(response.add_messages(msgs))
}

pub fn handle_unlock_single_lockup(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    user_address: Addr,
    duration: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut user_lockup_info =
        SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&user_address.clone(), duration))?;
    let mut lockup_info = SINGLE_LOCKUP_INFO.load(deps.storage, duration)?;
    let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let withdraw_amount = calculate_eclipastro_staked(
        user_lockup_info.astro_amount_in_lockups,
        user_lockup_info.xastro_amount_in_lockups,
        state.conversion_rate,
    )
    .unwrap();
    let mut msgs = vec![];
    if duration == 0 {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.flexible_staking.unwrap().to_string(),
            msg: to_json_binary(&FlexibleStakingExecuteMsg::Unstake {
                amount: withdraw_amount,
            })?,
            funds: vec![],
        }));
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.eclipastro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: user_address.to_string(),
                amount: withdraw_amount,
            })?,
            funds: vec![],
        }));
    } else {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.timelock_staking.unwrap().to_string(),
            msg: to_json_binary(&TimelockStakingExecuteMsg::Unstake {
                duration,
                locked_at: state.countdown_start_at,
                amount: Some(withdraw_amount),
            })?,
            funds: vec![],
        }));
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.eclipastro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: user_address.to_string(),
                amount: withdraw_amount,
            })?,
            funds: vec![],
        }));
    }
    user_lockup_info.unlock_flag = true;
    lockup_info.total_withdrawed = lockup_info
        .total_withdrawed
        .checked_add(withdraw_amount)
        .unwrap();
    SINGLE_USER_LOCKUP_INFO.save(deps.storage, (&user_address, duration), &user_lockup_info)?;
    SINGLE_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
    Ok(Response::new()
        .add_attribute("asset", cfg.eclipastro_token.to_string())
        .add_attribute("amount", withdraw_amount)
        .add_attribute("to", user_address.to_string())
        .add_messages(msgs))
}

pub fn handle_claim_rewards_and_unlock_for_lp_lockup(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    duration: u64,
    withdraw_lockup: bool,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let state = LP_LOCKUP_STATE.load(deps.storage)?;

    ensure!(
        state.are_claims_allowed,
        ContractError::ClaimRewardNotAllowed {}
    );

    let user_address = info.sender;

    let mut user_lockup_info = LP_USER_LOCKUP_INFO.load(deps.storage, (&user_address, duration))?;

    if user_lockup_info.total_eclip_incentives == Uint128::zero() {
        user_lockup_info.total_eclip_incentives = calculate_user_eclip_incentives_for_lp_lockup(
            deps.as_ref(),
            user_address.clone(),
            duration,
        )?;
    }
    let half_amount = user_lockup_info
        .total_eclip_incentives
        .checked_div(Uint128::from(2u128))
        .unwrap();
    let max_allowed_to_claim = half_amount
        .checked_add(half_amount.multiply_ratio(
            env.block.time.seconds() - state.countdown_start_at,
            duration,
        ))
        .unwrap();
    let claimable_amount = max_allowed_to_claim
        .checked_sub(user_lockup_info.claimed_eclip_incentives)
        .unwrap_or_default();
    user_lockup_info.claimed_eclip_incentives = max_allowed_to_claim;

    let mut msgs = vec![];
    let mut response =
        Response::new().add_attribute("action", "claim rewards and unlock lp lockup");
    if claimable_amount.gt(&Uint128::zero()) {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: user_address.to_string(),
            amount: coins(claimable_amount.into(), cfg.eclip.clone()),
        }));
        response = response
            .add_attribute("asset", cfg.eclip)
            .add_attribute("amount", claimable_amount)
            .add_attribute("to", user_address.to_string());
    }
    msgs.push(
        CallbackMsg::ClaimLpStakingAssetRewards {
            user_address: user_address.clone(),
            recipient: user_address.clone(),
            duration,
        }
        .to_cosmos_msg(&env)?,
    );

    if withdraw_lockup {
        ensure!(
            !user_lockup_info.unlock_flag,
            ContractError::AlreadyUnlocked {}
        );
        ensure!(
            env.block.time.seconds() - state.countdown_start_at > duration,
            ContractError::WaitToUnlock(
                state.countdown_start_at + duration - env.block.time.seconds()
            )
        );
        msgs.push(
            CallbackMsg::UnlockLpLockup {
                user_address: user_address.clone(),
                duration,
            }
            .to_cosmos_msg(&env)?,
        )
    }

    LP_USER_LOCKUP_INFO.save(deps.storage, (&user_address, duration), &user_lockup_info)?;
    Ok(response.add_messages(msgs))
}

pub fn handle_unlock_lp_lockup(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    user_address: Addr,
    duration: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut user_lockup_info =
        LP_USER_LOCKUP_INFO.load(deps.storage, (&user_address.clone(), duration))?;
    let mut lockup_info = LP_LOCKUP_INFO.load(deps.storage, duration)?;
    let state = LP_LOCKUP_STATE.load(deps.storage)?;
    let converted_eclipastro_amount = calculate_eclipastro_amount_for_lp(
        user_lockup_info.astro_amount_in_lockups,
        user_lockup_info.xastro_amount_in_lockups,
        state.conversion_rate,
    )
    .unwrap();
    let lp_amount = state
        .total_lp_lockdrop
        .multiply_ratio(state.total_eclipastro, converted_eclipastro_amount);
    let msgs = vec![
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.lp_staking.unwrap().to_string(),
            msg: to_json_binary(&LpStakingExecuteMsg::Unstake { amount: lp_amount })?,
            funds: vec![],
        }),
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.lp_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: user_address.to_string(),
                amount: lp_amount,
            })?,
            funds: vec![],
        }),
    ];
    user_lockup_info.unlock_flag = true;
    lockup_info.total_withdrawed = lockup_info.total_withdrawed.checked_add(lp_amount).unwrap();
    LP_USER_LOCKUP_INFO.save(deps.storage, (&user_address, duration), &user_lockup_info)?;
    LP_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
    Ok(Response::new()
        .add_attribute("asset", cfg.eclipastro_token.to_string())
        .add_attribute("amount", lp_amount)
        .add_attribute("to", user_address.to_string())
        .add_messages(msgs))
}

// stake eclipASTRO to single staking vault
// save xASTRO/eclipASTRO rate
// is_staked = true
fn handle_stake_to_single_vault(
    deps: DepsMut,
    env: Env,
    prev_eclipastro_balance: Uint128,
    astro_balance_to_convert: Uint128,
    xastro_balance_to_convert: Uint128,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let eclipastro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &cfg.eclipastro_token,
        &Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;
    let eclipastro_balance_from_xastro = eclipastro_balance
        .balance
        .checked_sub(prev_eclipastro_balance)
        .unwrap()
        .checked_sub(astro_balance_to_convert)
        .unwrap();
    state.conversion_rate =
        Decimal::from_ratio(eclipastro_balance_from_xastro, xastro_balance_to_convert);
    state.total_eclipastro_lockup = eclipastro_balance
        .balance
        .checked_sub(prev_eclipastro_balance)
        .unwrap();
    state.is_staked = true;
    let flexible_staking = cfg.flexible_staking.clone().unwrap();
    let timelock_staking = cfg.timelock_staking.clone().unwrap();

    let mut response = Response::new()
        .add_attribute("action", "convert ASTRO to eclipASTRO")
        .add_attribute("from", cfg.astro_token.to_string())
        .add_attribute("amount", astro_balance_to_convert)
        .add_attribute("to", cfg.eclipastro_token.to_string())
        .add_attribute("amount", astro_balance_to_convert)
        .add_attribute("action", "convert xASTRO to eclipASTRO")
        .add_attribute("from", cfg.xastro_token.to_string())
        .add_attribute("amount", xastro_balance_to_convert)
        .add_attribute("to", cfg.eclipastro_token.to_string())
        .add_attribute("amount", eclipastro_balance_from_xastro);
    let mut msgs = vec![];
    for c in &cfg.lock_configs {
        let mut lockup_info = SINGLE_LOCKUP_INFO
            .load(deps.storage, c.duration)
            .unwrap_or_default();
        let eclipastro_amount_to_stake = lockup_info
            .astro_amount_in_lockups
            .checked_add(
                lockup_info
                    .xastro_amount_in_lockups
                    .multiply_ratio(eclipastro_balance_from_xastro, xastro_balance_to_convert),
            )
            .unwrap();
        lockup_info.total_staked = eclipastro_amount_to_stake;
        SINGLE_LOCKUP_INFO.save(deps.storage, c.duration, &lockup_info)?;
        state.weighted_total_eclipastro_lockup = state
            .weighted_total_eclipastro_lockup
            .checked_add(calculate_weight(
                eclipastro_amount_to_stake,
                c.duration,
                &cfg,
            )?)
            .unwrap();
        if c.duration == 0u64 {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.eclipastro_token.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Send {
                    contract: flexible_staking.to_string(),
                    amount: eclipastro_amount_to_stake,
                    msg: to_json_binary(&FlexibleStakingCw20HookMsg::Stake {})?,
                })?,
                funds: vec![],
            }));
            response = response
                .add_attribute("action", "stake to flexible staking vault")
                .add_attribute("token", cfg.eclipastro_token.to_string())
                .add_attribute("amount", eclipastro_amount_to_stake);
        } else {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.eclipastro_token.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Send {
                    contract: timelock_staking.to_string(),
                    amount: eclipastro_amount_to_stake,
                    msg: to_json_binary(&TimelockStakingCw20HookMsg::Lock {
                        duration: c.duration,
                    })?,
                })?,
                funds: vec![],
            }));
            response = response
                .add_attribute("action", "lock to timelock staking vault")
                .add_attribute("token", cfg.eclipastro_token.to_string())
                .add_attribute("amount", eclipastro_amount_to_stake)
                .add_attribute("duration", c.duration.to_string());
        }
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
    prev_xastro_balance: Uint128,
    astro_balance_for_eclipastro: Uint128,
    astro_balance_for_xastro: Uint128,
    xastro_balance_for_eclipastro: Uint128,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut state = LP_LOCKUP_STATE.load(deps.storage)?;
    let current_eclipastro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &cfg.eclipastro_token,
        &Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;
    let current_xastro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &cfg.xastro_token,
        &Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;
    let eclipastro_balance_from_xastro = current_eclipastro_balance
        .balance
        .checked_sub(prev_eclipastro_balance)
        .unwrap()
        .checked_sub(astro_balance_for_eclipastro)
        .unwrap();
    state.conversion_rate = Decimal::from_ratio(
        eclipastro_balance_from_xastro,
        xastro_balance_for_eclipastro,
    );
    let eclipastro_amount_for_deposit = current_eclipastro_balance
        .balance
        .checked_sub(prev_eclipastro_balance)
        .unwrap();
    let xastro_amount_for_deposit = current_xastro_balance
        .balance
        .checked_sub(prev_xastro_balance)
        .unwrap();
    ensure!(
        eclipastro_amount_for_deposit.gt(&Uint128::zero())
            && xastro_amount_for_deposit.gt(&Uint128::zero()),
        ContractError::InvalidTokenBalance {}
    );
    state.total_xastro = xastro_amount_for_deposit;
    state.total_eclipastro = eclipastro_amount_for_deposit;
    LP_LOCKUP_STATE.save(deps.storage, &state)?;
    let msgs = vec![
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.eclipastro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                spender: cfg.liquidity_pool.to_string(),
                amount: eclipastro_amount_for_deposit,
                expires: None,
            })?,
            funds: vec![],
        }),
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.xastro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                spender: cfg.liquidity_pool.to_string(),
                amount: xastro_amount_for_deposit,
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
                        amount: eclipastro_amount_for_deposit,
                    },
                    Asset {
                        info: AssetInfo::Token {
                            contract_addr: cfg.xastro_token.clone(),
                        },
                        amount: xastro_amount_for_deposit,
                    },
                ],
                slippage_tolerance: None,
                auto_stake: Some(false),
                receiver: None,
            })?,
            funds: vec![],
        }),
    ];
    Ok(Response::new()
        .add_attribute("action", "convert ASTRO to eclipASTRO")
        .add_attribute("from", cfg.astro_token.to_string())
        .add_attribute("amount", astro_balance_for_eclipastro)
        .add_attribute("to", cfg.eclipastro_token.to_string())
        .add_attribute("amount", astro_balance_for_eclipastro)
        .add_attribute("action", "convert ASTRO to xASTRO")
        .add_attribute("from", cfg.astro_token.to_string())
        .add_attribute("amount", astro_balance_for_xastro)
        .add_attribute("to", cfg.xastro_token.to_string())
        .add_attribute("amount", astro_balance_for_eclipastro)
        .add_attribute("action", "convert xASTRO to eclipASTRO")
        .add_attribute("from", cfg.xastro_token.to_string())
        .add_attribute("amount", xastro_balance_for_eclipastro)
        .add_attribute("to", cfg.eclipastro_token.to_string())
        .add_attribute("amount", eclipastro_balance_from_xastro)
        .add_attribute(
            "action",
            "deposit eclipASTRO/xASTRO token pair to liquidity pool",
        )
        .add_attribute("token1", cfg.eclipastro_token.to_string())
        .add_attribute("amount", eclipastro_amount_for_deposit)
        .add_attribute("token2", cfg.xastro_token.to_string())
        .add_attribute("amount", xastro_amount_for_deposit)
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
    state.is_staked = true;
    state.total_lp_lockdrop = lp_token_to_stake;
    for c in &cfg.lock_configs {
        let mut lockup_info = LP_LOCKUP_INFO
            .load(deps.storage, c.duration)
            .unwrap_or_default();
        let converted_eclipastro_amount = calculate_eclipastro_amount_for_lp(
            lockup_info.astro_amount_in_lockups,
            lockup_info.xastro_amount_in_lockups,
            state.conversion_rate,
        )
        .unwrap();
        lockup_info.total_staked = state
            .total_lp_lockdrop
            .multiply_ratio(converted_eclipastro_amount, state.total_eclipastro);
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
            contract: cfg.lp_staking.unwrap().to_string(),
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

fn handle_distribute_single_staking_asset_rewards(
    deps: DepsMut,
    env: Env,
    prev_eclip_balance: Uint128,
    prev_eclipastro_balance: Uint128,
    user_address: Addr,
    recipient: Addr,
    duration: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let self_address = env.contract.address;
    let mut user_lockup_info =
        SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&user_address, duration))?;
    ensure!(
        !user_lockup_info.unlock_flag,
        ContractError::AlreadyUnlocked {}
    );
    let mut single_state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let total_eclip_stake = SINGLE_LOCKUP_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .fold(Uint128::zero(), |acc, i| {
            let (_, lockup_info) = i.unwrap();
            acc + lockup_info.total_staked - lockup_info.total_withdrawed
        });
    let eclip_balance = deps
        .querier
        .query_balance(self_address.clone(), cfg.eclip.clone())?;
    let eclipastro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &cfg.eclipastro_token,
        &Cw20QueryMsg::Balance {
            address: self_address.to_string(),
        },
    )?;
    let eclip_reward_amount = eclip_balance
        .amount
        .checked_sub(prev_eclip_balance)
        .unwrap();
    let eclipastro_reward_amount = eclipastro_balance
        .balance
        .checked_sub(prev_eclipastro_balance)
        .unwrap();
    if single_state.reward_weights.is_empty() {
        single_state.reward_weights = vec![
            AssetRewardWeight {
                asset: AssetInfo::Token {
                    contract_addr: cfg.eclipastro_token.clone(),
                },
                weight: Decimal::zero(),
            },
            AssetRewardWeight {
                asset: AssetInfo::NativeToken {
                    denom: cfg.eclip.clone(),
                },
                weight: Decimal::zero(),
            },
        ];
    }
    let reward_distributor_config: RewardDistributorConfig = deps.querier.query_wasm_smart(
        cfg.reward_distributor.unwrap().to_string(),
        &RewardDistributorQueryMsg::Config {},
    )?;
    let locking_reward_config = reward_distributor_config.locking_reward_config;
    let mut current_duration_multiplier = 0u64;
    single_state.reward_weights = single_state
        .reward_weights
        .into_iter()
        .map(|mut r| {
            if r.asset.equal(&AssetInfo::Token {
                contract_addr: cfg.eclipastro_token.clone(),
            }) {
                r.weight = r
                    .weight
                    .checked_add(Decimal::from_ratio(
                        eclipastro_reward_amount,
                        total_eclip_stake,
                    ))
                    .unwrap();
            }
            if r.asset.equal(&AssetInfo::NativeToken {
                denom: cfg.eclip.clone(),
            }) {
                let total_eclip_power_with_multiplier = SINGLE_LOCKUP_INFO
                    .range(deps.storage, None, None, Order::Ascending)
                    .fold(Uint128::zero(), |acc, cur| {
                        let (d, lockup_info) = cur.unwrap();
                        let reward_config = locking_reward_config.iter().find(|c| c.duration == d);
                        let reward_multiplier = match reward_config {
                            Some(c) => c.multiplier,
                            None => 0u64,
                        };
                        if d == duration {
                            current_duration_multiplier = reward_multiplier;
                        }
                        acc + (lockup_info.total_staked - lockup_info.total_withdrawed)
                            .checked_mul(Uint128::from(reward_multiplier))
                            .unwrap()
                    });
                r.weight = r
                    .weight
                    .checked_add(Decimal::from_ratio(
                        eclip_reward_amount,
                        total_eclip_power_with_multiplier,
                    ))
                    .unwrap();
            }
            r
        })
        .collect::<Vec<AssetRewardWeight>>();

    let user_eclipastro_staked = calculate_eclipastro_staked(
        user_lockup_info.astro_amount_in_lockups,
        user_lockup_info.xastro_amount_in_lockups,
        single_state.conversion_rate,
    )
    .unwrap();

    let mut msgs = vec![];
    let mut response = Response::new()
        .add_attribute("action", "claim asset rewards")
        .add_attribute("to", user_address.clone().to_string());
    for reward_weight in &single_state.reward_weights {
        let default_reward_weight = AssetRewardWeight {
            asset: reward_weight.clone().asset,
            weight: Decimal::zero(),
        };
        let user_reward_weight = user_lockup_info
            .reward_weights
            .iter()
            .find(|r| r.asset.equal(&reward_weight.asset))
            .unwrap_or(&default_reward_weight);
        let mut reward_amount = reward_weight
            .weight
            .checked_sub(user_reward_weight.weight)
            .unwrap()
            .checked_mul(Decimal::from_ratio(user_eclipastro_staked, 0u128))
            .unwrap()
            .to_uint_floor();
        if reward_weight.asset.equal(&AssetInfo::NativeToken {
            denom: cfg.eclip.clone(),
        }) {
            reward_amount = reward_amount
                .checked_mul(Uint128::from(current_duration_multiplier))
                .unwrap();
        }
        if reward_amount.gt(&Uint128::zero()) {
            if reward_weight.asset.is_native_token() {
                msgs.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: recipient.to_string(),
                    amount: coins(reward_amount.into(), reward_weight.asset.to_string()),
                }));
            } else {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: reward_weight.asset.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: recipient.to_string(),
                        amount: reward_amount,
                    })?,
                    funds: vec![],
                }));
            }
            response = response
                .add_attribute("asset", reward_weight.asset.to_string())
                .add_attribute("amount", reward_amount);
        }
    }
    user_lockup_info.reward_weights = single_state.reward_weights.clone();
    SINGLE_LOCKUP_STATE.save(deps.storage, &single_state)?;
    SINGLE_USER_LOCKUP_INFO.save(deps.storage, (&user_address, duration), &user_lockup_info)?;
    Ok(response.add_messages(msgs))
}

fn handle_lp_stake_distribute_asset_reward(
    deps: DepsMut,
    env: Env,
    prev_eclip_balance: Uint128,
    prev_astro_balance: Uint128,
    user_address: Addr,
    recipient: Addr,
    duration: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let self_address = env.contract.address;
    let mut user_lockup_info = LP_USER_LOCKUP_INFO.load(deps.storage, (&user_address, duration))?;
    ensure!(
        !user_lockup_info.unlock_flag,
        ContractError::AlreadyUnlocked {}
    );
    let mut lp_state = LP_LOCKUP_STATE.load(deps.storage)?;
    let eclip_balance = deps
        .querier
        .query_balance(self_address.clone(), cfg.eclip.clone())?;
    let astro_balance: BalanceResponse = deps.querier.query_wasm_smart(
        &cfg.astro_token,
        &Cw20QueryMsg::Balance {
            address: self_address.to_string(),
        },
    )?;
    let eclip_reward_amount = eclip_balance
        .amount
        .checked_sub(prev_eclip_balance)
        .unwrap();
    let astro_reward_amount = astro_balance
        .balance
        .checked_sub(prev_astro_balance)
        .unwrap();
    if lp_state.reward_weights.is_empty() {
        lp_state.reward_weights = vec![
            AssetRewardWeight {
                asset: AssetInfo::Token {
                    contract_addr: cfg.astro_token.clone(),
                },
                weight: Decimal::zero(),
            },
            AssetRewardWeight {
                asset: AssetInfo::NativeToken {
                    denom: cfg.eclip.clone(),
                },
                weight: Decimal::zero(),
            },
        ];
    }
    let total_lp_lockup = LP_LOCKUP_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .fold(Uint128::zero(), |acc, i| {
            let (_, lockup_info) = i.unwrap();
            acc + lockup_info.total_staked - lockup_info.total_withdrawed
        });
    lp_state.reward_weights = lp_state
        .reward_weights
        .into_iter()
        .map(|mut r| {
            if r.asset.equal(&AssetInfo::Token {
                contract_addr: cfg.astro_token.clone(),
            }) {
                r.weight = r
                    .weight
                    .checked_add(Decimal::from_ratio(astro_reward_amount, total_lp_lockup))
                    .unwrap();
            }
            if r.asset.equal(&AssetInfo::NativeToken {
                denom: cfg.eclip.clone(),
            }) {
                r.weight = r
                    .weight
                    .checked_add(Decimal::from_ratio(eclip_reward_amount, total_lp_lockup))
                    .unwrap();
            }
            r
        })
        .collect::<Vec<AssetRewardWeight>>();

    let user_lp_staked = calculate_eclipastro_amount_for_lp(
        user_lockup_info.astro_amount_in_lockups,
        user_lockup_info.xastro_amount_in_lockups,
        lp_state.conversion_rate,
    )
    .unwrap()
    .multiply_ratio(lp_state.total_lp_lockdrop, lp_state.total_eclipastro);

    let mut msgs = vec![];
    let mut response = Response::new()
        .add_attribute("action", "claim asset rewards")
        .add_attribute("to", user_address.clone().to_string());
    for reward_weight in &lp_state.reward_weights {
        let default_reward_weight = AssetRewardWeight {
            asset: reward_weight.asset.clone(),
            weight: Decimal::zero(),
        };
        let user_reward_weight = user_lockup_info
            .reward_weights
            .iter()
            .find(|r| r.asset.equal(&reward_weight.asset))
            .unwrap_or(&default_reward_weight);
        let reward_amount = reward_weight
            .weight
            .checked_sub(user_reward_weight.weight)
            .unwrap()
            .checked_mul(Decimal::from_ratio(user_lp_staked, 0u128))
            .unwrap()
            .to_uint_floor();
        if reward_amount.gt(&Uint128::zero()) {
            if reward_weight.asset.is_native_token() {
                msgs.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: recipient.to_string(),
                    amount: coins(reward_amount.into(), reward_weight.asset.to_string()),
                }));
            } else {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: reward_weight.asset.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: recipient.to_string(),
                        amount: reward_amount,
                    })?,
                    funds: vec![],
                }));
            }
            response = response
                .add_attribute("asset", reward_weight.asset.to_string())
                .add_attribute("amount", reward_amount);
        }
    }
    user_lockup_info.reward_weights = lp_state.reward_weights.clone();
    LP_LOCKUP_STATE.save(deps.storage, &lp_state)?;
    LP_USER_LOCKUP_INFO.save(deps.storage, (&user_address, duration), &user_lockup_info)?;
    Ok(response.add_messages(msgs))
}

fn calculate_user_eclip_incentives_for_single_lockup(
    deps: Deps,
    user_address: Addr,
    duration: u64,
) -> Result<Uint128, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let user_lockup_info = SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&user_address, duration))?;
    let user_eclipastro_staked = calculate_eclipastro_staked(
        user_lockup_info.astro_amount_in_lockups,
        user_lockup_info.xastro_amount_in_lockups,
        state.conversion_rate,
    )
    .unwrap();
    let duration_multiplier = cfg
        .lock_configs
        .into_iter()
        .find(|c| c.duration == duration)
        .unwrap_or_default()
        .multiplier;
    let amount = Uint256::from(user_eclipastro_staked)
        .checked_mul(Uint256::from(duration_multiplier))
        .unwrap()
        .multiply_ratio(
            state.total_eclip_incentives,
            state.weighted_total_eclipastro_lockup,
        )
        .try_into()
        .unwrap();
    Ok(amount)
}

fn calculate_user_eclip_incentives_for_lp_lockup(
    deps: Deps,
    user_address: Addr,
    duration: u64,
) -> Result<Uint128, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let state = LP_LOCKUP_STATE.load(deps.storage)?;
    let user_lockup_info = LP_USER_LOCKUP_INFO.load(deps.storage, (&user_address, duration))?;
    let converted_eclipastro_amount = calculate_eclipastro_amount_for_lp(
        user_lockup_info.astro_amount_in_lockups,
        user_lockup_info.xastro_amount_in_lockups,
        state.conversion_rate,
    )
    .unwrap();
    let user_lp_token_staked = state
        .total_lp_lockdrop
        .multiply_ratio(state.total_eclipastro, converted_eclipastro_amount);
    let duration_multiplier = cfg
        .lock_configs
        .into_iter()
        .find(|c| c.duration == duration)
        .unwrap_or_default()
        .multiplier;
    let amount = Uint256::from(user_lp_token_staked)
        .checked_mul(Uint256::from(duration_multiplier))
        .unwrap()
        .multiply_ratio(
            state.total_eclip_incentives,
            state.weighted_total_lp_lockdrop,
        )
        .try_into()
        .unwrap();
    Ok(amount)
}
