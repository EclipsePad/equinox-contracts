use astroport::{
    asset::{Asset, AssetInfo},
    pair::ExecuteMsg as PairExecuteMsg,
    staking::Cw20HookMsg as AstroStakingCw20HookMsg,
    token::BalanceResponse,
};
use cosmwasm_std::{
    attr, coins, ensure, ensure_eq, ensure_ne, from_json, to_json_binary, Addr, BankMsg, CosmosMsg,
    Decimal, DepsMut, Env, MessageInfo, Order, Response, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, Cw20ReceiveMsg};
use equinox_msg::{
    flexible_staking::{
        Cw20HookMsg as FlexibleStakingCw20HookMsg, ExecuteMsg as FlexibleStakingExecuteMsg,
        QueryMsg as FlexibleStakingQueryMsg,
    },
    lockdrop::{
        AssetRewardWeight, CallbackMsg, Cw20HookMsg, RewardDistributionConfig, StakeType,
        UpdateConfigMsg,
    },
    lp_staking::{
        Cw20HookMsg as LpStakingCw20HookMsg, ExecuteMsg as LpStakingExecuteMsg,
        QueryMsg as LpStakingQueryMsg, UserRewardResponse as LpStakingUserRewardResponse,
    },
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
    entry::query::{
        calculate_user_eclip_incentives_for_lp_lockup,
        calculate_user_eclip_incentives_for_single_lockup,
    },
    error::ContractError,
    math::{calculate_max_withdrawal_percent_allowed, calculate_weight},
    querier::{query_total_deposit_astro_staking, query_total_shares_astro_staking},
    state::{
        CONFIG, LP_LOCKUP_INFO, LP_LOCKUP_STATE, LP_USER_LOCKUP_INFO, OWNER,
        REWARD_DISTRIBUTION_CONFIG, SINGLE_LOCKUP_INFO, SINGLE_LOCKUP_STATE,
        SINGLE_USER_LOCKUP_INFO, TOTAL_ECLIP_INCENTIVES,
    },
};

pub fn receive_cw20(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let user_address = deps.api.addr_validate(&cw20_msg.sender)?;
    let amount = cw20_msg.amount;

    // CHECK :: Tokens sent > 0
    ensure!(amount.gt(&Uint128::zero()), ContractError::ZeroAmount {});

    match from_json(&cw20_msg.msg)? {
        Cw20HookMsg::IncreaseLockup {
            stake_type,
            duration,
        } => handle_increase_lockup(deps, env, info, stake_type, user_address, duration, amount),
        Cw20HookMsg::ExtendDuration {
            stake_type,
            from,
            to,
        } => {
            let current_time = env.block.time.seconds();
            ensure!(
                current_time >= cfg.init_timestamp,
                ContractError::DepositWindowNotStarted {}
            );
            ensure!(
                current_time <= cfg.init_timestamp + cfg.deposit_window,
                ContractError::DepositWindowClosed {}
            );
            ensure!(from < to, ContractError::ExtendDurationErr(from, to));
            ensure_ne!(
                cfg.lock_configs.iter().find(|c| c.duration == to),
                None,
                ContractError::InvalidDuration(to)
            );
            match stake_type {
                StakeType::SingleStaking => handle_single_sided_extend_duration(
                    deps,
                    env,
                    info,
                    user_address,
                    amount,
                    from,
                    to,
                ),
                StakeType::LpStaking => {
                    handle_lp_extend_duration(deps, env, info, user_address, amount, from, to)
                }
            }
        }
        Cw20HookMsg::Relock { from, to } => {
            ensure!(
                info.sender.clone() == cfg.eclipastro_token,
                ContractError::InvalidAsset {}
            );
            let cfg = CONFIG.load(deps.storage)?;
            let timelock_staking = cfg.timelock_staking.unwrap();
            let flexible_staking = cfg.flexible_staking.unwrap();
            let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
            let existing_amount = _unlock(deps.branch(), info, from, user_address.clone())?;

            ensure!(
                amount.gt(&Uint128::zero()),
                ContractError::InvalidTokenBalance {}
            );

            let mut msgs = vec![CallbackMsg::ClaimSingleStakingRewards {
                user_address: deps.api.addr_validate(&cw20_msg.sender)?,
                duration: from,
            }
            .to_cosmos_msg(&env)?];

            if from == 0u64 {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.eclipastro_token.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Send {
                        contract: flexible_staking.to_string(),
                        amount,
                        msg: to_json_binary(&FlexibleStakingCw20HookMsg::Relock {
                            duration: to,
                            amount: Some(existing_amount),
                            recipient: Some(user_address.to_string()),
                        })?,
                    })?,
                    funds: vec![],
                }));
            } else {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.eclipastro_token.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Send {
                        contract: timelock_staking.to_string(),
                        amount,
                        msg: to_json_binary(&TimelockStakingCw20HookMsg::Relock {
                            from_duration: from,
                            to_duration: to,
                            relocks: vec![(state.countdown_start_at, Some(existing_amount))],
                            recipient: Some(user_address.to_string()),
                        })?,
                    })?,
                    funds: vec![],
                }));
            }

            Ok(Response::new().add_messages(msgs))
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
            amount,
        } => handle_unlock_single_lockup(deps, env, info, user_address, duration, amount),
        CallbackMsg::UnlockLpLockup {
            user_address,
            duration,
            amount,
        } => handle_unlock_lp_lockup(deps, env, info, user_address, duration, amount),
        CallbackMsg::ClaimSingleStakingRewards {
            user_address,
            duration,
        } => handle_claim_rewards_and_unlock_for_single_lockup(
            deps,
            env,
            duration,
            user_address,
            None,
        ),
        CallbackMsg::StakeSingleVault {} => handle_stake_single_vault(deps, env, info),
        CallbackMsg::StakeLpVault {} => handle_stake_lp_vault(deps, env, info),
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
            let mut staking_amount = amount;
            let mut msgs = vec![];
            if staking_token == cfg.astro_token {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.astro_token.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Send {
                        contract: cfg.astro_staking.to_string(),
                        amount,
                        msg: to_json_binary(&AstroStakingCw20HookMsg::Enter {})?,
                    })?,
                    funds: vec![],
                }));
                let total_shares =
                    query_total_shares_astro_staking(deps.as_ref(), cfg.astro_staking.to_string())?;
                let total_deposit = query_total_deposit_astro_staking(
                    deps.as_ref(),
                    cfg.astro_staking.to_string(),
                )?;
                staking_amount = amount.multiply_ratio(total_shares, total_deposit);
            } else if staking_token != cfg.xastro_token {
                return Err(ContractError::InvalidLockupAsset(staking_token.to_string()));
            }
            lockup_info.xastro_amount_in_lockups = lockup_info
                .xastro_amount_in_lockups
                .checked_add(staking_amount)
                .unwrap();
            user_lockup_info.xastro_amount_in_lockups = user_lockup_info
                .xastro_amount_in_lockups
                .checked_add(staking_amount)
                .unwrap();
            SINGLE_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
            SINGLE_USER_LOCKUP_INFO.save(
                deps.storage,
                (&user_address, duration),
                &user_lockup_info,
            )?;

            Ok(Response::new()
                .add_attributes(vec![
                    attr("action", "increase_lockup_position"),
                    attr("type", "single staking"),
                    attr("from", user_address),
                    attr("asset", staking_token),
                    attr("amount", amount),
                    attr("duration", duration.to_string()),
                ])
                .add_messages(msgs))
        }
        StakeType::LpStaking => {
            let mut lockup_info = LP_LOCKUP_INFO
                .load(deps.storage, duration)
                .unwrap_or_default();
            let mut user_lockup_info = LP_USER_LOCKUP_INFO
                .load(deps.storage, (&user_address, duration))
                .unwrap_or_default();
            let mut staking_amount = amount;
            let mut msgs = vec![];
            if staking_token == cfg.astro_token {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.astro_token.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Send {
                        contract: cfg.astro_staking.to_string(),
                        amount,
                        msg: to_json_binary(&AstroStakingCw20HookMsg::Enter {})?,
                    })?,
                    funds: vec![],
                }));
                let total_shares =
                    query_total_shares_astro_staking(deps.as_ref(), cfg.astro_staking.to_string())?;
                let total_deposit = query_total_deposit_astro_staking(
                    deps.as_ref(),
                    cfg.astro_staking.to_string(),
                )?;
                staking_amount = amount.multiply_ratio(total_shares, total_deposit);
            } else if staking_token != cfg.xastro_token {
                return Err(ContractError::InvalidLockupAsset(staking_token.to_string()));
            }
            lockup_info.xastro_amount_in_lockups = lockup_info
                .xastro_amount_in_lockups
                .checked_add(staking_amount)
                .unwrap();
            user_lockup_info.xastro_amount_in_lockups = user_lockup_info
                .xastro_amount_in_lockups
                .checked_add(staking_amount)
                .unwrap();
            LP_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
            LP_USER_LOCKUP_INFO.save(deps.storage, (&user_address, duration), &user_lockup_info)?;

            Ok(Response::new()
                .add_attributes(vec![
                    attr("action", "increase_lockup_position"),
                    attr("type", "lp staking"),
                    attr("from", user_address),
                    attr("asset", staking_token),
                    attr("amount", amount),
                    attr("duration", duration.to_string()),
                ])
                .add_messages(msgs))
        }
    }
}

pub fn handle_single_sided_extend_duration(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    user_address: Addr,
    amount: Uint128,
    from_duration: u64,
    to_duration: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let staking_token = info.sender;

    let mut lockup_info = SINGLE_LOCKUP_INFO.load(deps.storage, from_duration)?;
    let user_lockup_info =
        SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&user_address, from_duration))?;

    let existing_xastro_amount = user_lockup_info.xastro_amount_in_lockups;
    lockup_info.xastro_amount_in_lockups = lockup_info
        .xastro_amount_in_lockups
        .checked_sub(existing_xastro_amount)
        .unwrap();

    SINGLE_LOCKUP_INFO.save(deps.storage, from_duration, &lockup_info)?;
    SINGLE_USER_LOCKUP_INFO.remove(deps.storage, (&user_address, from_duration));

    let mut lockup_info = SINGLE_LOCKUP_INFO
        .load(deps.storage, to_duration)
        .unwrap_or_default();
    let mut user_lockup_info = SINGLE_USER_LOCKUP_INFO
        .load(deps.storage, (&user_address, to_duration))
        .unwrap_or_default();

    let mut staking_amount = amount;
    let mut msgs = vec![];
    if staking_token == cfg.astro_token {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.astro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: cfg.astro_staking.to_string(),
                amount,
                msg: to_json_binary(&AstroStakingCw20HookMsg::Enter {})?,
            })?,
            funds: vec![],
        }));
        let total_shares =
            query_total_shares_astro_staking(deps.as_ref(), cfg.astro_staking.to_string())?;
        let total_deposit =
            query_total_deposit_astro_staking(deps.as_ref(), cfg.astro_staking.to_string())?;
        staking_amount = amount.multiply_ratio(total_shares, total_deposit);
    } else if staking_token != cfg.xastro_token {
        return Err(ContractError::InvalidLockupAsset(staking_token.to_string()));
    }

    lockup_info.xastro_amount_in_lockups += existing_xastro_amount + staking_amount;

    user_lockup_info.xastro_amount_in_lockups += existing_xastro_amount + staking_amount;

    SINGLE_LOCKUP_INFO.save(deps.storage, to_duration, &lockup_info)?;
    SINGLE_USER_LOCKUP_INFO.save(
        deps.storage,
        (&user_address, to_duration),
        &user_lockup_info,
    )?;

    Ok(Response::new()
        .add_attributes(vec![
            attr("action", "increase_lockup_position"),
            attr("type", "single staking"),
            attr("from", user_address),
            attr("asset", staking_token),
            attr("amount", amount),
            attr("action", "extend_duration"),
            attr("amount", existing_xastro_amount + staking_amount),
            attr("from", from_duration.to_string()),
            attr("to", to_duration.to_string()),
        ])
        .add_messages(msgs))
}

pub fn handle_lp_extend_duration(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    user_address: Addr,
    amount: Uint128,
    from_duration: u64,
    to_duration: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let staking_token = info.sender;

    let mut lockup_info = LP_LOCKUP_INFO.load(deps.storage, from_duration)?;
    let user_lockup_info =
        LP_USER_LOCKUP_INFO.load(deps.storage, (&user_address, from_duration))?;

    let existing_xastro_amount = user_lockup_info.xastro_amount_in_lockups;
    lockup_info.xastro_amount_in_lockups = lockup_info
        .xastro_amount_in_lockups
        .checked_sub(existing_xastro_amount)
        .unwrap();
    LP_LOCKUP_INFO.save(deps.storage, from_duration, &lockup_info)?;
    LP_USER_LOCKUP_INFO.remove(deps.storage, (&user_address, from_duration));

    let mut lockup_info = LP_LOCKUP_INFO
        .load(deps.storage, to_duration)
        .unwrap_or_default();
    let mut user_lockup_info = LP_USER_LOCKUP_INFO
        .load(deps.storage, (&user_address, to_duration))
        .unwrap_or_default();

    let mut staking_amount = amount;
    let mut msgs = vec![];
    if staking_token == cfg.astro_token {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.astro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: cfg.astro_staking.to_string(),
                amount,
                msg: to_json_binary(&AstroStakingCw20HookMsg::Enter {})?,
            })?,
            funds: vec![],
        }));
        let total_shares =
            query_total_shares_astro_staking(deps.as_ref(), cfg.astro_staking.to_string())?;
        let total_deposit =
            query_total_deposit_astro_staking(deps.as_ref(), cfg.astro_staking.to_string())?;
        staking_amount = amount.multiply_ratio(total_shares, total_deposit);
    } else if staking_token != cfg.xastro_token {
        return Err(ContractError::InvalidLockupAsset(staking_token.to_string()));
    }

    lockup_info.xastro_amount_in_lockups += existing_xastro_amount + staking_amount;
    user_lockup_info.xastro_amount_in_lockups += existing_xastro_amount + staking_amount;
    LP_LOCKUP_INFO.save(deps.storage, to_duration, &lockup_info)?;
    LP_USER_LOCKUP_INFO.save(
        deps.storage,
        (&user_address, to_duration),
        &user_lockup_info,
    )?;

    Ok(Response::new()
        .add_attributes(vec![
            attr("action", "increase_lockup_position"),
            attr("type", "lp staking"),
            attr("from", user_address),
            attr("asset", staking_token),
            attr("amount", amount),
            attr("action", "extend_duration"),
            attr("amount", existing_xastro_amount + staking_amount),
            attr("from", from_duration.to_string()),
            attr("to", to_duration.to_string()),
        ])
        .add_messages(msgs))
}

pub fn handle_extend_lock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stake_type: StakeType,
    from: u64,
    to: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    ensure!(
        cfg.init_timestamp + cfg.deposit_window > env.block.time.seconds(),
        ContractError::DepositWindowClosed {}
    );
    ensure_ne!(
        cfg.lock_configs.iter().find(|c| c.duration == to),
        None,
        ContractError::InvalidDuration(to)
    );
    ensure!(from < to, ContractError::ExtendDurationErr(from, to));
    let sender = info.sender;
    match stake_type {
        StakeType::SingleStaking => {
            let mut from_lockup_info = SINGLE_LOCKUP_INFO.load(deps.storage, from)?;
            let mut to_lockup_info = SINGLE_LOCKUP_INFO
                .load(deps.storage, to)
                .unwrap_or_default();
            let mut from_user_lockup_info =
                SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&sender, from))?;
            let mut to_user_lockup_info = SINGLE_USER_LOCKUP_INFO
                .load(deps.storage, (&sender, to))
                .unwrap_or_default();
            let amount_to_extend = from_user_lockup_info.xastro_amount_in_lockups;
            from_lockup_info.xastro_amount_in_lockups -= amount_to_extend;
            to_lockup_info.total_staked += amount_to_extend;
            from_user_lockup_info.xastro_amount_in_lockups -= amount_to_extend;
            to_user_lockup_info.xastro_amount_in_lockups += amount_to_extend;
            SINGLE_LOCKUP_INFO.save(deps.storage, from, &from_lockup_info)?;
            SINGLE_LOCKUP_INFO.save(deps.storage, to, &to_lockup_info)?;
            SINGLE_USER_LOCKUP_INFO.remove(deps.storage, (&sender, from));
            SINGLE_USER_LOCKUP_INFO.save(deps.storage, (&sender, to), &to_user_lockup_info)?;
        }
        StakeType::LpStaking => {
            let mut from_lockup_info = LP_LOCKUP_INFO.load(deps.storage, from)?;
            let mut to_lockup_info = LP_LOCKUP_INFO.load(deps.storage, to).unwrap_or_default();
            let mut from_user_lockup_info =
                LP_USER_LOCKUP_INFO.load(deps.storage, (&sender, from))?;
            let mut to_user_lockup_info = LP_USER_LOCKUP_INFO
                .load(deps.storage, (&sender, to))
                .unwrap_or_default();
            let amount_to_extend = from_user_lockup_info.xastro_amount_in_lockups;
            from_lockup_info.xastro_amount_in_lockups -= amount_to_extend;
            to_lockup_info.total_staked += amount_to_extend;
            from_user_lockup_info.xastro_amount_in_lockups -= amount_to_extend;
            to_user_lockup_info.xastro_amount_in_lockups += amount_to_extend;
            LP_LOCKUP_INFO.save(deps.storage, from, &from_lockup_info)?;
            LP_LOCKUP_INFO.save(deps.storage, to, &to_lockup_info)?;
            LP_USER_LOCKUP_INFO.remove(deps.storage, (&sender, from));
            LP_USER_LOCKUP_INFO.save(deps.storage, (&sender, to), &to_user_lockup_info)?;
        }
    }
    Ok(Response::new())
}

pub fn handle_stake_to_vaults(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // check is owner
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;

    let cfg = CONFIG.load(deps.storage)?;
    let single_state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let lp_state = LP_LOCKUP_STATE.load(deps.storage)?;

    // check is already staked
    ensure_eq!(
        single_state.is_staked || lp_state.is_staked,
        false,
        ContractError::AlreadyStaked {}
    );

    // check time window
    ensure!(
        env.block.time.seconds()
            > (cfg.init_timestamp + cfg.deposit_window + cfg.withdrawal_window),
        ContractError::LockdropNotFinished {}
    );

    let msgs = vec![
        CallbackMsg::StakeSingleVault {}.to_cosmos_msg(&env)?,
        CallbackMsg::StakeLpVault {}.to_cosmos_msg(&env)?,
    ];
    Ok(Response::new().add_messages(msgs))
}

/// stake all the lockup assets to single staking vault
/// staking is only allowed after withdraw window
/// only owner can do this
/// ASTRO/xASTRO will be converted to eclipASTRO and be staked to single staking vault
/// change SINGLE_STATE's is_staked to true
pub fn handle_stake_single_vault(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
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
        return Ok(Response::new());
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
            contract_addr: cfg.xastro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: cfg.converter.to_string(),
                amount: total_xastro_amount_to_staking,
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

    msgs.push(
        CallbackMsg::StakeToSingleVault {
            prev_eclipastro_balance: prev_eclipastro_balance.balance,
            xastro_amount_to_convert: total_xastro_amount_to_staking,
            weighted_amount: total_weighted_xastro_amount_to_staking,
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
    _info: MessageInfo,
) -> Result<Response, ContractError> {
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
        return Ok(Response::new());
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
            contract_addr: cfg.xastro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: cfg.converter.to_string(),
                amount: half_xastro_amount,
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

    Ok(Response::new().add_messages(msgs))
}

fn _unlock(
    deps: DepsMut,
    _info: MessageInfo,
    from: u64,
    user: Addr,
) -> Result<Uint128, ContractError> {
    let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    ensure!(
        state.is_staked && state.are_claims_allowed,
        ContractError::RelockNotAllowed {}
    );
    let mut lockup_info = SINGLE_LOCKUP_INFO.load(deps.storage, from)?;
    let mut user_lockup_info = SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&user, from))?;
    if user_lockup_info.total_eclipastro_staked.is_zero() && !user_lockup_info.withdrawal_flag {
        user_lockup_info.total_eclipastro_staked = user_lockup_info
            .xastro_amount_in_lockups
            .multiply_ratio(state.total_eclipastro_lockup, state.total_xastro);
    }
    let amount =
        user_lockup_info.total_eclipastro_staked - user_lockup_info.total_eclipastro_withdrawed;
    user_lockup_info.total_eclipastro_withdrawed += amount;
    user_lockup_info.withdrawal_flag = true;
    lockup_info.total_withdrawed += amount;

    SINGLE_LOCKUP_INFO.save(deps.storage, from, &lockup_info)?;
    SINGLE_USER_LOCKUP_INFO.save(deps.storage, (&user, from), &user_lockup_info)?;
    Ok(amount)
}

pub fn handle_relock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    from: u64,
    to: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let timelock_staking = cfg.timelock_staking.unwrap();
    let flexible_staking = cfg.flexible_staking.unwrap();
    let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let sender = info.sender.clone();
    let amount = _unlock(deps, info, from, sender.clone())?;

    ensure!(
        amount.gt(&Uint128::zero()),
        ContractError::InvalidTokenBalance {}
    );

    let mut msgs = vec![CallbackMsg::ClaimSingleStakingRewards {
        user_address: sender.clone(),
        duration: from,
    }
    .to_cosmos_msg(&env)?];

    if from == 0u64 {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: flexible_staking.to_string(),
            msg: to_json_binary(&FlexibleStakingExecuteMsg::Relock {
                amount: Some(amount),
                duration: to,
                recipient: Some(sender.to_string()),
            })?,
            funds: vec![],
        }))
    } else {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: timelock_staking.to_string(),
            msg: to_json_binary(&TimelockStakingExecuteMsg::Relock {
                from_duration: from,
                relocks: vec![(state.countdown_start_at, Some(amount))],
                to_duration: to,
                recipient: Some(sender.to_string()),
            })?,
            funds: vec![],
        }));
    }

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

    if let Some(dao_treasury_address) = new_cfg.dao_treasury_address {
        cfg.dao_treasury_address = deps.api.addr_validate(&dao_treasury_address)?;
        attributes.push(attr("new_timelock_staking", &dao_treasury_address))
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

pub fn handle_update_reward_distribution_config(
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

// can withdraw freely at deposit window
// can withdraw only once at withdraw window
// withdraw amount is limited at withdraw window
pub fn handle_single_locking_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
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

    ensure!(
        env.block.time.seconds() <= cfg.init_timestamp + cfg.deposit_window + cfg.withdrawal_window,
        ContractError::LockdropFinished {}
    );

    ensure_eq!(
        user_lockup_info.withdrawal_flag,
        false,
        ContractError::AlreadyWithdrawed {}
    );
    let max_withdrawal_allowed = user_lockup_info.xastro_amount_in_lockups * max_withdrawal_percent;

    ensure!(
        max_withdrawal_allowed.gt(&Uint128::zero()),
        ContractError::NoDeposit {}
    );

    let mut withdraw_amount = amount.unwrap_or(max_withdrawal_allowed);

    ensure!(
        withdraw_amount.gt(&Uint128::zero()),
        ContractError::ZeroAmount {}
    );
    if withdraw_amount.gt(&max_withdrawal_allowed) {
        withdraw_amount = max_withdrawal_allowed;
    }

    let mut msgs = vec![];
    let mut response =
        Response::new().add_attribute("action", "withdraw_from_single_staking_lockup");

    user_lockup_info.xastro_amount_in_lockups -= withdraw_amount;
    lockup_info.xastro_amount_in_lockups -= withdraw_amount;

    // COSMOS_MSG ::TRANSFER WITHDRAWN tokens
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.xastro_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: user_address.to_string(),
            amount: withdraw_amount,
        })?,
        funds: vec![],
    }));

    response = response
        .add_attribute("token", cfg.xastro_token.to_string())
        .add_attribute("user_address", user_address.to_string())
        .add_attribute("duration", duration.to_string())
        .add_attribute("amount", withdraw_amount);

    // Update withdrawal flag after the deposit window
    if env.block.time.seconds() >= cfg.init_timestamp + cfg.deposit_window {
        user_lockup_info.withdrawal_flag = true;
    }

    if user_lockup_info.xastro_amount_in_lockups.is_zero() {
        SINGLE_USER_LOCKUP_INFO.remove(deps.storage, (&user_address, duration));
    } else {
        SINGLE_USER_LOCKUP_INFO.save(deps.storage, (&user_address, duration), &user_lockup_info)?;
    }
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
    amount: Option<Uint128>,
    duration: u64,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    // Check :: Amount should be within the allowed withdrawal limit bounds
    let max_withdrawal_percent =
        calculate_max_withdrawal_percent_allowed(env.block.time.seconds(), &cfg);
    let user_address = info.sender;
    let mut user_lockup_info = LP_USER_LOCKUP_INFO.load(deps.storage, (&user_address, duration))?;
    let mut lockup_info = LP_LOCKUP_INFO.load(deps.storage, duration)?;

    ensure!(
        env.block.time.seconds() <= cfg.init_timestamp + cfg.deposit_window + cfg.withdrawal_window,
        ContractError::LockdropFinished {}
    );

    ensure_eq!(
        user_lockup_info.withdrawal_flag,
        false,
        ContractError::AlreadyWithdrawed {}
    );

    let max_withdrawal_allowed = user_lockup_info.xastro_amount_in_lockups * max_withdrawal_percent;

    ensure!(
        max_withdrawal_allowed.gt(&Uint128::zero()),
        ContractError::NoDeposit {}
    );

    if let Some(amount) = amount {
        ensure!(amount.gt(&Uint128::zero()), ContractError::ZeroAmount {});
        ensure!(
            amount.le(&max_withdrawal_allowed),
            ContractError::WithdrawLimitExceed(max_withdrawal_allowed.to_string())
        );
    }

    let withdraw_amount = amount.unwrap_or(max_withdrawal_allowed);

    let mut msgs = vec![];
    let mut response = Response::new().add_attribute("action", "withdraw_from_lp_staking_lockup");

    user_lockup_info.xastro_amount_in_lockups -= withdraw_amount;
    lockup_info.xastro_amount_in_lockups -= withdraw_amount;

    // COSMOS_MSG ::TRANSFER WITHDRAWN tokens
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.xastro_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: user_address.to_string(),
            amount: withdraw_amount,
        })?,
        funds: vec![],
    }));

    response = response
        .add_attribute("token", cfg.xastro_token.to_string())
        .add_attribute("user_address", user_address.to_string())
        .add_attribute("duration", duration.to_string())
        .add_attribute("amount", withdraw_amount);

    // Update withdrawal flag after the deposit window
    if env.block.time.seconds() >= cfg.init_timestamp + cfg.deposit_window {
        user_lockup_info.withdrawal_flag = true;
    }

    if user_lockup_info.xastro_amount_in_lockups.is_zero() {
        LP_USER_LOCKUP_INFO.remove(deps.storage, (&user_address, duration));
    } else {
        LP_USER_LOCKUP_INFO.save(deps.storage, (&user_address, duration), &user_lockup_info)?;
    }
    LP_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;

    Ok(response.add_messages(msgs))
}

pub fn handle_increase_eclip_incentives(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut total_eclip_incentives = TOTAL_ECLIP_INCENTIVES
        .load(deps.storage)
        .unwrap_or_default();
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    ensure!(
        env.block.time.seconds() < cfg.init_timestamp + cfg.deposit_window,
        ContractError::DepositWindowClosed {}
    );
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
        total_eclip_incentives += coin.amount;
        TOTAL_ECLIP_INCENTIVES.save(deps.storage, &total_eclip_incentives)?;
        response = response.add_attribute("amount", coin.amount);
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
    if !flexible_staking_reward_response.eclip.is_zero()
        || !flexible_staking_reward_response.eclipastro.is_zero()
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
        .any(|r| !r.eclip.is_zero() || !r.eclipastro.is_zero());

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

    let lp_staking_reward_response: Vec<LpStakingUserRewardResponse> =
        deps.querier.query_wasm_smart(
            cfg.lp_staking.clone().unwrap(),
            &LpStakingQueryMsg::Reward {
                user: env.contract.address.to_string(),
            },
        )?;

    let mut msgs = vec![];

    if lp_staking_reward_response
        .into_iter()
        .any(|r| !r.amount.is_zero())
    {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.lp_staking.unwrap().to_string(),
            msg: to_json_binary(&LpStakingExecuteMsg::Claim {})?,
            funds: vec![],
        }));
    }

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

    msgs.push(
        CallbackMsg::DistributeLpStakingAssetRewards {
            prev_eclip_balance: eclip_balance.amount,
            prev_astro_balance: astro_balance.balance,
            user_address,
            recipient,
            duration,
        }
        .to_cosmos_msg(&env)?,
    );

    Ok(Response::default().add_messages(msgs))
}

pub fn handle_claim_rewards_and_unlock_for_single_lockup(
    deps: DepsMut,
    env: Env,
    duration: u64,
    user_address: Addr,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let reward_cfg = REWARD_DISTRIBUTION_CONFIG.load(deps.storage)?;
    let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;

    ensure!(
        state.are_claims_allowed,
        ContractError::ClaimRewardNotAllowed {}
    );

    let mut user_lockup_info =
        SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&user_address.clone(), duration))?;
    ensure!(
        !user_lockup_info.xastro_amount_in_lockups.is_zero(),
        ContractError::NoDeposit {}
    );
    if user_lockup_info.total_eclip_incentives == Uint128::zero() {
        user_lockup_info.total_eclip_incentives =
            calculate_user_eclip_incentives_for_single_lockup(
                deps.as_ref(),
                user_address.clone(),
                duration,
            )?;
    }
    let instant_amount = user_lockup_info
        .total_eclip_incentives
        .multiply_ratio(reward_cfg.instant, 10000u64);
    let vesting_amount = user_lockup_info.total_eclip_incentives - instant_amount;
    let max_allowed_to_claim =
        if env.block.time.seconds() >= state.countdown_start_at + reward_cfg.vesting_period {
            user_lockup_info.total_eclip_incentives
        } else {
            instant_amount
                .checked_add(vesting_amount.multiply_ratio(
                    env.block.time.seconds() - state.countdown_start_at,
                    reward_cfg.vesting_period,
                ))
                .unwrap()
        };
    let claimable_amount = max_allowed_to_claim
        .checked_sub(user_lockup_info.claimed_eclip_incentives)
        .unwrap_or_default();
    user_lockup_info.claimed_eclip_incentives = max_allowed_to_claim;

    let mut msgs = vec![];
    let mut response =
        Response::new().add_attribute("action", "claim rewards and unlock single lockup");

    if user_lockup_info.total_eclipastro_staked.is_zero()
        || user_lockup_info.total_eclipastro_staked > user_lockup_info.total_eclipastro_withdrawed
    {
        msgs.push(
            CallbackMsg::ClaimSingleStakingAssetRewards {
                user_address: user_address.clone(),
                recipient: user_address.clone(),
                duration,
            }
            .to_cosmos_msg(&env)?,
        );
    }
    if claimable_amount.gt(&Uint128::zero()) {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: user_address.to_string(),
            amount: coins(claimable_amount.into(), cfg.eclip.clone()),
        }));
        response = response
            .add_attribute("asset", cfg.eclip)
            .add_attribute("amount", claimable_amount)
            .add_attribute("to", user_address.clone());
    }

    if let Some(amount) = amount {
        if user_lockup_info.total_eclipastro_staked.is_zero() {
            user_lockup_info.total_eclipastro_staked = user_lockup_info
                .xastro_amount_in_lockups
                .multiply_ratio(state.total_eclipastro_lockup, state.total_xastro);
        }
        ensure!(
            user_lockup_info.total_eclipastro_staked - user_lockup_info.total_eclipastro_withdrawed
                >= amount
                && !amount.is_zero(),
            ContractError::InvalidTokenBalance {}
        );
        msgs.push(
            CallbackMsg::UnlockSingleLockup {
                user_address: user_address.clone(),
                duration,
                amount,
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
    amount: Uint128,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut user_lockup_info =
        SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&user_address.clone(), duration))?;
    let mut lockup_info = SINGLE_LOCKUP_INFO.load(deps.storage, duration)?;
    let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let mut msgs = vec![];
    if duration == 0 {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.flexible_staking.unwrap().to_string(),
            msg: to_json_binary(&FlexibleStakingExecuteMsg::Unstake {
                amount,
                recipient: Some(user_address.to_string()),
            })?,
            funds: vec![],
        }));
    } else {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.timelock_staking.unwrap().to_string(),
            msg: to_json_binary(&TimelockStakingExecuteMsg::Unlock {
                duration,
                locked_at: state.countdown_start_at,
                amount: Some(amount),
                recipient: Some(user_address.to_string()),
            })?,
            funds: vec![],
        }));
    }
    user_lockup_info.total_eclipastro_withdrawed += amount;
    lockup_info.total_withdrawed += amount;
    SINGLE_USER_LOCKUP_INFO.save(deps.storage, (&user_address, duration), &user_lockup_info)?;
    SINGLE_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
    Ok(Response::new()
        .add_attribute("asset", cfg.eclipastro_token.to_string())
        .add_attribute("amount", amount)
        .add_attribute("to", user_address.to_string())
        .add_messages(msgs))
}

pub fn handle_claim_rewards_and_unlock_for_lp_lockup(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    duration: u64,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let reward_cfg = REWARD_DISTRIBUTION_CONFIG.load(deps.storage)?;
    let state = LP_LOCKUP_STATE.load(deps.storage)?;

    ensure!(
        state.are_claims_allowed,
        ContractError::ClaimRewardNotAllowed {}
    );

    let user_address = info.sender;

    let mut user_lockup_info = LP_USER_LOCKUP_INFO.load(deps.storage, (&user_address, duration))?;
    ensure!(
        !user_lockup_info.xastro_amount_in_lockups.is_zero(),
        ContractError::NoDeposit {}
    );

    if user_lockup_info.total_eclip_incentives.is_zero() {
        user_lockup_info.total_eclip_incentives = calculate_user_eclip_incentives_for_lp_lockup(
            deps.as_ref(),
            user_address.clone(),
            duration,
        )?;
    }
    let instant_amount = user_lockup_info
        .total_eclip_incentives
        .multiply_ratio(reward_cfg.instant, 10000u64);
    let vesting_amount = user_lockup_info.total_eclip_incentives - instant_amount;
    let max_allowed_to_claim =
        if env.block.time.seconds() >= state.countdown_start_at + reward_cfg.vesting_period {
            user_lockup_info.total_eclip_incentives
        } else {
            instant_amount
                .checked_add(vesting_amount.multiply_ratio(
                    env.block.time.seconds() - state.countdown_start_at,
                    reward_cfg.vesting_period,
                ))
                .unwrap()
        };
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

    if user_lockup_info.total_lp_staked.is_zero()
        || user_lockup_info.total_lp_staked > user_lockup_info.total_lp_withdrawed
    {
        msgs.push(
            CallbackMsg::ClaimLpStakingAssetRewards {
                user_address: user_address.clone(),
                recipient: user_address.clone(),
                duration,
            }
            .to_cosmos_msg(&env)?,
        );
    }

    if let Some(amount) = amount {
        if user_lockup_info.total_lp_staked.is_zero() {
            user_lockup_info.total_lp_staked = user_lockup_info
                .xastro_amount_in_lockups
                .multiply_ratio(state.total_lp_lockdrop, state.total_xastro);
        }
        ensure!(
            user_lockup_info.total_lp_staked - user_lockup_info.total_lp_withdrawed >= amount
                && !amount.is_zero(),
            ContractError::InvalidTokenBalance {}
        );
        msgs.push(
            CallbackMsg::UnlockLpLockup {
                user_address: user_address.clone(),
                duration,
                amount,
            }
            .to_cosmos_msg(&env)?,
        )
    }

    LP_USER_LOCKUP_INFO.save(deps.storage, (&user_address, duration), &user_lockup_info)?;
    Ok(response.add_messages(msgs))
}

pub fn handle_unlock_lp_lockup(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    user_address: Addr,
    duration: u64,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let state = LP_LOCKUP_STATE.load(deps.storage)?;
    let mut user_lockup_info =
        LP_USER_LOCKUP_INFO.load(deps.storage, (&user_address.clone(), duration))?;
    let mut lockup_info = LP_LOCKUP_INFO.load(deps.storage, duration)?;
    let mut penalty_amount = Uint128::zero();

    if env.block.time.seconds() < state.countdown_start_at + duration {
        penalty_amount = amount.checked_div_ceil((2u128, 1u128)).unwrap();
    }
    let mut msgs = vec![
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.lp_staking.unwrap().to_string(),
            msg: to_json_binary(&LpStakingExecuteMsg::Unstake {
                amount,
                recipient: None,
            })?,
            funds: vec![],
        }),
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.lp_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: user_address.to_string(),
                amount: amount - penalty_amount,
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

    user_lockup_info.total_lp_withdrawed += amount;
    lockup_info.total_withdrawed = lockup_info.total_withdrawed.checked_add(amount).unwrap();
    LP_USER_LOCKUP_INFO.save(deps.storage, (&user_address, duration), &user_lockup_info)?;
    LP_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
    Ok(Response::new()
        .add_attribute("asset", cfg.eclipastro_token.to_string())
        .add_attribute("amount", amount)
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
    state.is_staked = true;
    state.are_claims_allowed = true;
    state.countdown_start_at = env.block.time.seconds();
    let flexible_staking = cfg.flexible_staking.clone().unwrap();
    let timelock_staking = cfg.timelock_staking.clone().unwrap();

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
                        recipient: None,
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
            contract_addr: cfg.xastro_token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                spender: cfg.liquidity_pool.to_string(),
                amount: xastro_amount_to_deposit,
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
                        info: AssetInfo::Token {
                            contract_addr: cfg.xastro_token.clone(),
                        },
                        amount: xastro_amount_to_deposit,
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
    state.is_staked = true;
    state.are_claims_allowed = true;
    state.countdown_start_at = env.block.time.seconds();
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
    let mut single_state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let total_eclipastro_stake = SINGLE_LOCKUP_INFO
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
                        total_eclipastro_stake,
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

    let user_eclipastro_staked = if user_lockup_info.total_eclipastro_staked.is_zero() {
        user_lockup_info.xastro_amount_in_lockups.multiply_ratio(
            single_state.total_eclipastro_lockup,
            single_state.total_xastro,
        )
    } else {
        user_lockup_info.total_eclipastro_staked - user_lockup_info.total_eclipastro_withdrawed
    };

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
        let mut reward_amount =
            user_eclipastro_staked * (reward_weight.weight - user_reward_weight.weight);
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

    let user_lp_staked = if user_lockup_info.total_lp_staked.is_zero() {
        user_lockup_info
            .xastro_amount_in_lockups
            .multiply_ratio(lp_state.total_lp_lockdrop, lp_state.total_xastro)
    } else {
        user_lockup_info.total_lp_staked - user_lockup_info.total_lp_withdrawed
    };

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
        let reward_amount = user_lp_staked
            * (reward_weight
                .weight
                .checked_sub(user_reward_weight.weight)
                .unwrap());
        if !reward_amount.is_zero() {
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
