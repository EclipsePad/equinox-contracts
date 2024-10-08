use astroport::{
    asset::{Asset, AssetInfo, AssetInfoExt, PairInfo},
    pair::{ExecuteMsg as PairExecuteMsg, QueryMsg as AstroportPairQueryMsg},
    staking::ExecuteMsg as AstroStakingExecuteMsg,
};
use cosmwasm_std::{
    attr, coin, coins, ensure, ensure_eq, from_json, to_json_binary, Addr, BankMsg, Coin,
    CosmosMsg, DepsMut, Env, MessageInfo, Order, QuerierWrapper, Response, StdError, StdResult,
    Uint128, Uint256, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_utils::one_coin;
use eclipse_base::staking::msg::ExecuteMsg as EclipStakingExecuteMsg;
use equinox_msg::{
    lockdrop::{
        CallbackMsg, Cw20HookMsg, IncentiveRewards, RewardDistributionConfig, StakeType,
        UpdateConfigMsg,
    },
    lp_staking::{Cw20HookMsg as LpStakingCw20HookMsg, ExecuteMsg as LpExecuteMsg},
    single_sided_staking::{ExecuteMsg as SingleSidedExecuteMsg, UserReward},
    utils::has_unique_elements,
    voter::msg::ExecuteMsg as VoterExecuteMsg,
};

use crate::{
    entry::query::{
        calculate_lp_staking_user_rewards, calculate_lp_total_rewards, calculate_penalty_amount, calculate_pending_lockdrop_incentives, calculate_updated_lp_reward_weights, check_deposit_window, check_lockdrop_ended, get_user_lp_lockdrop_incentives, get_user_single_lockdrop_incentives, query_astro_staking_total_deposit, query_astro_staking_total_shares, query_lp_pool_assets, query_user_single_rewards
    },
    error::ContractError,
    math::{calculate_max_withdrawal_amount_allowed, calculate_weight},
    state::{
        BLACK_LIST, BLACK_LIST_REWARDS, CONFIG, LP_LOCKDROP_INCENTIVES, LP_LOCKUP_INFO,
        LP_LOCKUP_STATE, LP_STAKING_REWARD_WEIGHTS, LP_USER_LOCKUP_INFO, OWNER,
        REWARD_DISTRIBUTION_CONFIG, SINGLE_LOCKDROP_INCENTIVES, SINGLE_LOCKUP_INFO,
        SINGLE_LOCKUP_STATE, SINGLE_USER_LOCKUP_INFO,
    },
};

use super::query::{check_withdrawal_window, query_native_token_supply};

/// Update config
/// Only owner
pub fn try_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_cfg: UpdateConfigMsg,
) -> Result<Response, ContractError> {
    let mut cfg = CONFIG.load(deps.storage)?;
    let mut attributes = vec![attr("action", "update_config")];

    OWNER.assert_admin(deps.as_ref(), &info.sender)?;

    if let Some(single_sided_staking) = new_cfg.single_sided_staking {
        cfg.single_sided_staking = Some(deps.api.addr_validate(&single_sided_staking)?);
        attributes.push(attr("new_single_sided_staking", &single_sided_staking));
    }

    if let Some(lp_staking) = new_cfg.lp_staking {
        cfg.lp_staking = Some(deps.api.addr_validate(&lp_staking)?);
        attributes.push(attr("new_lp_staking", &lp_staking));
    }

    if let Some(liquidity_pool) = new_cfg.liquidity_pool {
        let pool_info: PairInfo = deps
            .querier
            .query_wasm_smart(&liquidity_pool, &AstroportPairQueryMsg::Pair {})?;
        cfg.liquidity_pool = Some(deps.api.addr_validate(&liquidity_pool)?);
        cfg.lp_token = Some(AssetInfo::NativeToken {
            denom: pool_info.liquidity_token.clone(),
        });
        attributes.push(attr("new_liquidity_pool", &liquidity_pool));
        attributes.push(attr("new_lp_token", &pool_info.liquidity_token));
    }

    if let Some(eclipastro_token) = new_cfg.eclipastro_token {
        cfg.eclipastro_token = Some(AssetInfo::NativeToken {
            denom: eclipastro_token.clone(),
        });
        attributes.push(attr("new_eclipastro_token", &eclipastro_token));
    };

    if let Some(voter) = new_cfg.voter {
        cfg.voter = Some(deps.api.addr_validate(&voter)?);
        attributes.push(attr("new_voter", &voter));
    };

    if let Some(eclip_staking) = new_cfg.eclip_staking {
        cfg.eclip_staking = Some(deps.api.addr_validate(&eclip_staking)?);
        attributes.push(attr("new_eclip_staking", &eclip_staking));
    };

    if let Some(dao_treasury_address) = new_cfg.dao_treasury_address {
        cfg.dao_treasury_address = Some(deps.api.addr_validate(&dao_treasury_address)?);
        attributes.push(attr("new_dao_treasury_address", &dao_treasury_address));
    };

    if let Some(init_early_unlock_penalty) = new_cfg.init_early_unlock_penalty {
        cfg.init_early_unlock_penalty = init_early_unlock_penalty;
        attributes.push(attr("new_init_early_unlock_penalty", init_early_unlock_penalty.to_string()));
    };
    CONFIG.save(deps.storage, &cfg)?;
    Ok(Response::new().add_attributes(attributes))
}

/// Update reward distribution config
/// Only owner
/// Only before lockdrop ended
pub fn try_update_reward_distribution_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_cfg: RewardDistributionConfig,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;

    ensure!(
        !check_lockdrop_ended(deps.as_ref(), env.block.time.seconds()).unwrap(),
        ContractError::LockdropEnded {}
    );

    let attributes = vec![
        attr("action", "update_reward_distribution_config"),
        attr("instant_bps", new_cfg.instant.to_string()),
        attr("vesting period", new_cfg.vesting_period.to_string()),
    ];

    REWARD_DISTRIBUTION_CONFIG.save(deps.storage, &new_cfg)?;
    Ok(Response::new().add_attributes(attributes))
}

/// Update owner
/// Only owner
pub fn try_update_owner(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_owner: Addr,
) -> Result<Response, ContractError> {
    // only owner can update owner
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    OWNER.set(deps.branch(), Some(new_owner.clone()))?;
    Ok(Response::new()
        .add_attribute("action", "update owner")
        .add_attribute("to", new_owner.to_string()))
}

/// Deposit assets to Lockdrop
/// Only during deposit window
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
    let block_time = env.block.time.seconds();

    // only deposit window
    ensure!(
        check_deposit_window(deps.as_ref(), block_time).unwrap(),
        ContractError::NotDepositWindow {}
    );
    // check duration is allowed
    ensure!(
        cfg.lock_configs.iter().any(|c| c.duration == duration),
        ContractError::InvalidDuration(duration)
    );
    // only ASTRO / xASTRO tokens are allowed
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
        StakeType::SingleStaking => {
            _increase_single_lockup(deps, duration, sender, received_token.amount)
        }
        StakeType::LpStaking => _increase_lp_lockup(deps, duration, sender, received_token.amount),
    }
}

/// Extend duration
/// Add additional amount is allowed
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
    // user deposits more assets during extending
    let deposit_existing = !received_tokens.is_empty();
    let block_time = env.block.time.seconds();
    let sender = info.sender.to_string();

    ensure!(
        received_tokens.iter().len() <= 1,
        ContractError::InvalidAsset {}
    );
    // check if to duration is allowed
    ensure!(
        cfg.lock_configs.iter().any(|c| c.duration == to_duration),
        ContractError::InvalidDuration(to_duration)
    );
    // check if to duration is greater than from_duration
    ensure!(
        from_duration < to_duration,
        ContractError::ExtendDurationErr(from_duration, to_duration)
    );

    // deposit window only or after Equinox is live
    if check_deposit_window(deps.as_ref(), block_time).unwrap() {
        let mut add_amount = Uint128::zero();
        if deposit_existing {
            let received_token = &received_tokens[0];
            // if user deposits more asset, check asset is ASTRO or xASTRO
            ensure!(
                received_token.denom == cfg.astro_token || received_token.denom == cfg.xastro_token,
                ContractError::InvalidAsset {}
            );
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
                return _extend_single_lockup(deps, from_duration, to_duration, sender, add_amount);
            }
            StakeType::LpStaking => {
                return _extend_lp_lockup(deps, from_duration, to_duration, sender, add_amount);
            }
        }
    } else if check_lockdrop_ended(deps.as_ref(), block_time).unwrap() && cfg.claims_allowed {
        match stake_type {
            StakeType::SingleStaking => {
                let mut add_amount = Uint128::zero();
                if deposit_existing {
                    let received_token = &received_tokens[0];
                    // if user deposits more asset, check asset is ASTRO or xASTRO or eclipASTRO
                    ensure!(
                        received_token.denom == cfg.astro_token
                            || received_token.denom == cfg.xastro_token
                            || received_token.denom
                                == cfg.eclipastro_token.clone().unwrap().to_string(),
                        ContractError::InvalidAsset {}
                    );
                    if received_token.denom != cfg.eclipastro_token.clone().unwrap().to_string() {
                        let eclipastro_balance: Coin = deps.querier.query_balance(
                            env.contract.address.to_string(),
                            cfg.eclipastro_token.unwrap().to_string(),
                        )?;
                        let msgs = vec![
                            convert_eclipastro_msg(cfg.voter.unwrap().to_string(), received_token)?,
                            CallbackMsg::ExtendLockupAfterLockdrop {
                                prev_eclipastro_balance: eclipastro_balance.amount,
                                from_duration,
                                to_duration,
                                sender,
                            }
                            .to_cosmos_msg(&env)?,
                        ];
                        return Ok(Response::new()
                            .add_attribute("action", "convert ASTRO to eclipASTRO")
                            .add_attribute("amount", received_token.amount.to_string())
                            .add_messages(msgs));
                    }
                    add_amount = received_token.amount;
                }
                return _extend_single_lockup_after_lockdrop(
                    deps,
                    env,
                    from_duration,
                    to_duration,
                    sender,
                    add_amount,
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
    let block_time = env.block.time.seconds();

    // check is already staked
    ensure!(!cfg.claims_allowed, ContractError::AlreadyStaked {});

    // check time window
    ensure!(
        check_lockdrop_ended(deps.as_ref(), block_time).unwrap(),
        ContractError::LockdropNotEnded {}
    );

    cfg.claims_allowed = true;
    cfg.countdown_start_at = block_time;

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
    assets: Option<Vec<AssetInfo>>,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    let blacklist = BLACK_LIST.load(deps.storage).unwrap_or_default();
    ensure!(!blacklist.contains(&sender), ContractError::Blacklisted {});
    match stake_type {
        StakeType::SingleStaking => {
            _claim_single_sided_rewards(deps, env, sender, duration, assets)
        }
        StakeType::LpStaking => _claim_lp_rewards(deps, env, sender, duration, assets),
    }
}

pub fn try_claim_all_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stake_type: StakeType,
    with_flexible: bool,
    assets: Option<Vec<AssetInfo>>,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    let blacklist = BLACK_LIST.load(deps.storage).unwrap_or_default();
    ensure!(!blacklist.contains(&sender), ContractError::Blacklisted {});
    match stake_type {
        StakeType::SingleStaking => {
            _claim_all_single_sided_rewards(deps, env, sender, None, with_flexible, assets)
        }
        StakeType::LpStaking => {
            _claim_all_lp_rewards(deps, env, sender, None, with_flexible, assets)
        }
    }
}

pub fn _increase_single_lockup(
    deps: DepsMut,
    duration: u64,
    sender: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut lockup_info = SINGLE_LOCKUP_INFO
        .load(deps.storage, duration)
        .unwrap_or_default();
    let mut user_lockup_info = SINGLE_USER_LOCKUP_INFO
        .load(deps.storage, (&sender, duration))
        .unwrap_or_default();
    lockup_info.xastro_amount_in_lockups = lockup_info
        .xastro_amount_in_lockups
        .checked_add(amount)
        .unwrap();
    user_lockup_info.xastro_amount_in_lockups = user_lockup_info
        .xastro_amount_in_lockups
        .checked_add(amount)
        .unwrap();
    SINGLE_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
    SINGLE_USER_LOCKUP_INFO.save(deps.storage, (&sender, duration), &user_lockup_info)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "increase_lockup_position"),
        attr("type", "single staking"),
        attr("from", sender),
        attr("asset", cfg.xastro_token),
        attr("amount", amount.to_string()),
        attr("duration", duration.to_string()),
    ]))
}

pub fn _increase_lp_lockup(
    deps: DepsMut,
    duration: u64,
    sender: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut lockup_info = LP_LOCKUP_INFO
        .load(deps.storage, duration)
        .unwrap_or_default();
    let mut user_lockup_info = LP_USER_LOCKUP_INFO
        .load(deps.storage, (&sender, duration))
        .unwrap_or_default();
    lockup_info.xastro_amount_in_lockups = lockup_info
        .xastro_amount_in_lockups
        .checked_add(amount)
        .unwrap();
    user_lockup_info.xastro_amount_in_lockups = user_lockup_info
        .xastro_amount_in_lockups
        .checked_add(amount)
        .unwrap();
    LP_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
    LP_USER_LOCKUP_INFO.save(deps.storage, (&sender, duration), &user_lockup_info)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "increase_lockup_position"),
        attr("type", "lp staking"),
        attr("from", sender),
        attr("asset", cfg.xastro_token),
        attr("amount", amount.to_string()),
        attr("duration", duration.to_string()),
    ]))
}

pub fn _extend_single_lockup(
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

pub fn _extend_lp_lockup(
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

pub fn _extend_single_lockup_after_lockdrop(
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
        Some(vec![from_duration]),
        true,
        None,
    )?;
    let cfg = CONFIG.load(deps.storage)?;
    let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let mut lockup_info_from = SINGLE_LOCKUP_INFO.load(deps.storage, from_duration)?;
    let mut user_lockup_info_from =
        SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&sender, from_duration))?;
    if user_lockup_info_from.total_eclipastro_staked.is_zero() {
        user_lockup_info_from.total_eclipastro_staked = user_lockup_info_from
            .xastro_amount_in_lockups
            .multiply_ratio(state.total_eclipastro_lockup, state.total_xastro);
    }
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

    let mut funds = vec![];
    if !add_amount.is_zero() {
        funds.push(coin(
            add_amount.u128(),
            cfg.eclipastro_token.unwrap().to_string(),
        ));
    }
    let mut locked_at = Some(cfg.countdown_start_at);
    if from_duration == 0u64 {
        locked_at = None;
    }
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.single_sided_staking.unwrap().to_string(),
        msg: to_json_binary(&SingleSidedExecuteMsg::Restake {
            from_duration,
            locked_at,
            amount: Some(existing_eclipastro_amount),
            to_duration,
            recipient: Some(sender.clone()),
        })?,
        funds: vec![],
    }));

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
        .query_balance(env.contract.address, cfg.xastro_token)?;
    let xastro_deposit = xastro_balance.amount - prev_xastro_balance;
    ensure!(
        !xastro_deposit.is_zero(),
        ContractError::InvalidDepositAmounts {}
    );

    match stake_type {
        StakeType::SingleStaking => _increase_single_lockup(deps, duration, sender, xastro_deposit),
        StakeType::LpStaking => _increase_lp_lockup(deps, duration, sender, xastro_deposit),
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
            _extend_single_lockup(deps, from_duration, to_duration, sender, xastro_deposit)
        }
        StakeType::LpStaking => {
            _extend_lp_lockup(deps, from_duration, to_duration, sender, xastro_deposit)
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
    let eclipastro_balance = deps.querier.query_balance(
        &env.contract.address,
        cfg.eclipastro_token.unwrap().to_string(),
    )?;
    let eclipastro_deposit = eclipastro_balance.amount - prev_eclipastro_balance;
    ensure!(
        !eclipastro_deposit.is_zero(),
        ContractError::InvalidDepositAmounts {}
    );

    _extend_single_lockup_after_lockdrop(
        deps,
        env,
        from_duration,
        to_duration,
        sender,
        eclipastro_deposit,
    )
}

pub fn receive_cw20(
    mut deps: DepsMut,
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
        Cw20HookMsg::IncreaseIncentives { rewards } => {
            OWNER.assert_admin(deps.as_ref(), &sender)?;
            ensure!(
                cfg.eclip.to_string() == info.sender,
                ContractError::InvalidAsset {}
            );
            ensure!(
                !check_lockdrop_ended(deps.as_ref(), env.block.time.seconds()).unwrap(),
                ContractError::LockdropEnded {}
            );

            let single_rewards = rewards
                .clone()
                .into_iter()
                .find(|d| d.stake_type == StakeType::SingleStaking)
                .unwrap_or(IncentiveRewards {
                    stake_type: StakeType::SingleStaking,
                    eclip: Uint128::zero(),
                    beclip: Uint128::zero(),
                });
            let lp_rewards = rewards
                .into_iter()
                .find(|d| d.stake_type == StakeType::LpStaking)
                .unwrap_or(IncentiveRewards {
                    stake_type: StakeType::LpStaking,
                    eclip: Uint128::zero(),
                    beclip: Uint128::zero(),
                });
            ensure!(
                single_rewards.eclip + single_rewards.beclip + lp_rewards.eclip + lp_rewards.beclip
                    == amount,
                ContractError::InvalidAmountCheck {}
            );

            _handle_increase_incentives(
                deps.branch(),
                single_rewards.eclip,
                single_rewards.beclip,
                lp_rewards.eclip,
                lp_rewards.beclip,
            )?;

            Ok(Response::new().add_attribute("action", "increase Lockdrop incentives"))
        }
    }
}

pub fn try_increase_incentives(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    rewards: Vec<IncentiveRewards>,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let cfg = CONFIG.load(deps.storage)?;
    let asset = one_coin(&info)?;
    ensure!(
        cfg.eclip.to_string() == asset.denom,
        ContractError::InvalidAsset {}
    );
    ensure!(
        !check_lockdrop_ended(deps.as_ref(), env.block.time.seconds()).unwrap(),
        ContractError::LockdropEnded {}
    );

    let single_rewards = rewards
        .clone()
        .into_iter()
        .find(|d| d.stake_type == StakeType::SingleStaking)
        .unwrap_or(IncentiveRewards {
            stake_type: StakeType::SingleStaking,
            eclip: Uint128::zero(),
            beclip: Uint128::zero(),
        });
    let lp_rewards = rewards
        .into_iter()
        .find(|d| d.stake_type == StakeType::LpStaking)
        .unwrap_or(IncentiveRewards {
            stake_type: StakeType::LpStaking,
            eclip: Uint128::zero(),
            beclip: Uint128::zero(),
        });
    ensure!(
        single_rewards.eclip + single_rewards.beclip + lp_rewards.eclip + lp_rewards.beclip
            == asset.amount,
        ContractError::InvalidAmountCheck {}
    );

    _handle_increase_incentives(
        deps.branch(),
        single_rewards.eclip,
        single_rewards.beclip,
        lp_rewards.eclip,
        lp_rewards.beclip,
    )?;

    Ok(Response::new().add_attribute("action", "increase Lockdrop incentives"))
}

pub fn _handle_increase_incentives(
    deps: DepsMut,
    single_eclip: Uint128,
    single_beclip: Uint128,
    lp_eclip: Uint128,
    lp_beclip: Uint128,
) -> Result<bool, ContractError> {
    let mut single_lockdrop_incentives = SINGLE_LOCKDROP_INCENTIVES
        .load(deps.storage)
        .unwrap_or_default();
    let mut lp_lockdrop_incentives = LP_LOCKDROP_INCENTIVES
        .load(deps.storage)
        .unwrap_or_default();
    single_lockdrop_incentives.eclip += single_eclip;
    single_lockdrop_incentives.beclip += single_beclip;
    lp_lockdrop_incentives.eclip += lp_eclip;
    lp_lockdrop_incentives.beclip += lp_beclip;
    SINGLE_LOCKDROP_INCENTIVES.save(deps.storage, &single_lockdrop_incentives)?;
    LP_LOCKDROP_INCENTIVES.save(deps.storage, &lp_lockdrop_incentives)?;
    Ok(true)
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
            total_xastro_amount,
            xastro_amount_to_deposit,
            weighted_amount,
        } => handle_deposit_into_pool(
            deps,
            env,
            prev_eclipastro_balance,
            total_xastro_amount,
            xastro_amount_to_deposit,
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
                    .multiply_ratio(duration_multiplier, 10000u128),
            )
            .unwrap()
        });

    let mut msgs = vec![];

    if total_xastro_amount_to_staking.gt(&Uint128::zero()) {
        msgs.push(convert_eclipastro_msg(
            cfg.voter.unwrap().to_string(),
            &coin(total_xastro_amount_to_staking.u128(), cfg.xastro_token),
        )?);
    }

    // callback function to stake eclipASTRO to single staking vaults
    let prev_eclipastro_balance: Coin = deps.querier.query_balance(
        env.contract.address.to_string(),
        cfg.eclipastro_token.unwrap().to_string(),
    )?;

    msgs.push(
        CallbackMsg::StakeToSingleVault {
            prev_eclipastro_balance: prev_eclipastro_balance.amount,
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

    ensure!(
        cfg.single_sided_staking.is_some()
            && cfg.lp_staking.is_some()
            && cfg.liquidity_pool.is_some(),
        ContractError::EquinoxNotLive {}
    );

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
                    .multiply_ratio(duration_multiplier, 10000u128),
            )
            .unwrap()
        });

    let astro_staking_total_deposit = query_astro_staking_total_deposit(deps.as_ref())?;
    let astro_staking_total_shares = query_astro_staking_total_shares(deps.as_ref())?;
    let lp_pool_assets = query_lp_pool_assets(deps.as_ref())?;
    let eclipastro_asset = lp_pool_assets
        .iter()
        .find(|a| a.info.to_string() == cfg.eclipastro_token.clone().unwrap().to_string())
        .unwrap();
    let xastro_asset = lp_pool_assets
        .iter()
        .find(|a| a.info.to_string() == cfg.xastro_token.clone())
        .unwrap();
    let numerator = Uint256::from_uint128(astro_staking_total_shares)
        .checked_mul(Uint256::from_uint128(eclipastro_asset.amount))
        .unwrap();
    let denominator = numerator
        .checked_add(
            Uint256::from_uint128(astro_staking_total_deposit)
                .checked_mul(Uint256::from_uint128(xastro_asset.amount))
                .unwrap(),
        )
        .unwrap();

    let xastro_exchange_amount =
        if eclipastro_asset.amount.is_zero() || xastro_asset.amount.is_zero() {
            xastro_amount_to_stake.multiply_ratio(1u128, 2u128)
        } else {
            Uint256::from_uint128(xastro_amount_to_stake)
                .multiply_ratio(numerator, denominator)
                .try_into()
                .unwrap()
        };

    let mut msgs = vec![];

    if xastro_exchange_amount.gt(&Uint128::zero()) {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.voter.unwrap().to_string(),
            msg: to_json_binary(&VoterExecuteMsg::SwapToEclipAstro {})?,
            funds: vec![coin(xastro_exchange_amount.u128(), cfg.xastro_token)],
        }));
    }

    // callback function to stake eclipASTRO to single staking vaults
    let prev_eclipastro_balance: Coin = deps.querier.query_balance(
        env.contract.address.to_string(),
        cfg.eclipastro_token.unwrap().to_string(),
    )?;
    let lp_token = cfg.lp_token.unwrap();
    let prev_lp_token_balance =
        lp_token.query_pool(&deps.querier, env.contract.address.to_string())?;
    msgs.push(
        CallbackMsg::DepositIntoPool {
            prev_eclipastro_balance: prev_eclipastro_balance.amount,
            total_xastro_amount: xastro_amount_to_stake,
            xastro_amount_to_deposit: xastro_amount_to_stake - xastro_exchange_amount,
            weighted_amount: total_weighted_xastro_amount_to_staking,
        }
        .to_cosmos_msg(&env)?,
    );
    msgs.push(
        CallbackMsg::StakeLpToken {
            prev_lp_token_balance,
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
    let eclipastro_token = cfg.eclipastro_token.clone().unwrap();
    let eclipastro_balance: Coin = deps.querier.query_balance(
        env.contract.address.to_string(),
        eclipastro_token.to_string(),
    )?;
    let eclipastro_balance_to_lockup = eclipastro_balance
        .amount
        .checked_sub(prev_eclipastro_balance)
        .unwrap();
    state.total_xastro = xastro_amount_to_convert;
    state.weighted_total_xastro = weighted_amount;
    state.total_eclipastro_lockup = eclipastro_balance_to_lockup;

    let mut response = Response::new()
        .add_attribute("action", "convert xASTRO to eclipASTRO")
        .add_attribute("from", cfg.xastro_token.to_string())
        .add_attribute("amount", xastro_amount_to_convert)
        .add_attribute("to", eclipastro_token.to_string())
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
            contract_addr: cfg.single_sided_staking.clone().unwrap().to_string(),
            msg: to_json_binary(&SingleSidedExecuteMsg::Stake {
                duration: c.duration,
                recipient: None,
            })?,
            funds: vec![coin(
                eclipastro_amount_to_stake.u128(),
                eclipastro_token.to_string(),
            )],
        }));
        response = response
            .add_attribute("action", "lock to single sided staking vault")
            .add_attribute("token", eclipastro_token.to_string())
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
    total_xastro_amount: Uint128,
    xastro_amount_to_deposit: Uint128,
    weighted_amount: Uint128,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut state = LP_LOCKUP_STATE.load(deps.storage)?;
    let eclipastro_token = cfg.eclipastro_token.clone().unwrap();
    let current_eclipastro_balance: Coin = deps.querier.query_balance(
        env.contract.address.to_string(),
        eclipastro_token.to_string(),
    )?;
    let eclipastro_amount_to_deposit = current_eclipastro_balance
        .amount
        .checked_sub(prev_eclipastro_balance)
        .unwrap();
    ensure!(
        eclipastro_amount_to_deposit.gt(&Uint128::zero())
            && xastro_amount_to_deposit.gt(&Uint128::zero()),
        ContractError::InvalidTokenBalance {}
    );
    state.total_xastro = total_xastro_amount;
    state.weighted_total_xastro = weighted_amount;
    LP_LOCKUP_STATE.save(deps.storage, &state)?;
    let msgs = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.liquidity_pool.unwrap().to_string(),
        msg: to_json_binary(&PairExecuteMsg::ProvideLiquidity {
            assets: vec![
                Asset {
                    info: eclipastro_token.clone(),
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
            min_lp_to_receive: None,
        })?,
        funds: vec![
            coin(xastro_amount_to_deposit.u128(), cfg.xastro_token.clone()),
            coin(
                eclipastro_amount_to_deposit.u128(),
                eclipastro_token.to_string(),
            ),
        ],
    })];
    Ok(Response::new()
        .add_attribute("action", "convert xASTRO to eclipASTRO")
        .add_attribute("from", cfg.xastro_token.to_string())
        .add_attribute("amount", xastro_amount_to_deposit)
        .add_attribute("to", eclipastro_token.to_string())
        .add_attribute("amount", eclipastro_amount_to_deposit)
        .add_attribute(
            "action",
            "deposit eclipASTRO/xASTRO token pair to liquidity pool",
        )
        .add_attribute("token1", eclipastro_token.to_string())
        .add_attribute("amount", eclipastro_amount_to_deposit)
        .add_attribute("token2", cfg.xastro_token)
        .add_attribute("amount", xastro_amount_to_deposit)
        .add_messages(msgs))
}

fn handle_stake_lp_token(
    deps: DepsMut,
    env: Env,
    prev_lp_token_balance: Uint128,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut state = LP_LOCKUP_STATE.load(deps.storage)?;
    let lp_token = cfg.lp_token.clone().unwrap();

    let lp_token_balance = lp_token.query_pool(&deps.querier, env.contract.address.to_string())?;
    let lp_token_to_stake = lp_token_balance.checked_sub(prev_lp_token_balance).unwrap();
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
    let msg = match lp_token.clone() {
        AssetInfo::NativeToken { denom } => CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.lp_staking.unwrap().to_string(),
            msg: to_json_binary(&LpExecuteMsg::Stake { recipient: None })?,
            funds: vec![coin(lp_token_to_stake.u128(), denom)],
        }),
        AssetInfo::Token { contract_addr } => CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: cfg.lp_staking.unwrap().to_string(),
                amount: lp_token_to_stake,
                msg: to_json_binary(&LpStakingCw20HookMsg::Stake { recipient: None })?,
            })?,
            funds: vec![],
        }),
    };
    LP_LOCKUP_STATE.save(deps.storage, &state)?;
    Ok(Response::new()
        .add_attribute("action", "stake lp token")
        .add_attribute("token", lp_token.to_string())
        .add_attribute("amount", lp_token_to_stake)
        .add_message(msg))
}

pub fn _claim_single_sided_rewards(
    mut deps: DepsMut,
    env: Env,
    sender: String,
    duration: u64,
    assets: Option<Vec<AssetInfo>>,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let blacklist = BLACK_LIST.load(deps.storage).unwrap_or_default();
    let mut blacklist_rewards = BLACK_LIST_REWARDS.load(deps.storage).unwrap_or_default();
    let eclipastro_token = cfg.eclipastro_token.clone().unwrap();
    let block_time = env.block.time.seconds();

    ensure_eq!(
        cfg.claims_allowed,
        true,
        ContractError::ClaimRewardNotAllowed {}
    );

    // check if there are duplicated assets
    let assets_list = assets
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|a| a.to_string());
    ensure!(
        has_unique_elements(assets_list),
        ContractError::DuplicatedAssets {}
    );

    let locked_at = if duration == 0u64 {
        None
    } else {
        Some(cfg.countdown_start_at)
    };

    let rewards = calculate_single_user_rewards(
        deps.branch(),
        sender.clone(),
        duration,
        block_time,
        locked_at,
        assets,
    )?;

    let mut msgs = vec![];

    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.single_sided_staking.clone().unwrap().to_string(),
        msg: to_json_binary(&SingleSidedExecuteMsg::Claim {
            duration,
            locked_at,
            assets: None,
        })?,
        funds: vec![],
    }));

    if blacklist.contains(&sender) {
        blacklist_rewards.eclipastro += rewards.eclipastro;
        blacklist_rewards.beclip += rewards.beclip;
        blacklist_rewards.eclip += rewards.eclip;
        BLACK_LIST_REWARDS.save(deps.storage, &blacklist_rewards)?;
    } else {
        if !rewards.eclipastro.is_zero() {
            msgs.push(
                eclipastro_token
                    .with_balance(rewards.eclipastro)
                    .into_msg(sender.clone())?,
            );
        }

        if !rewards.beclip.is_zero() {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.eclip_staking.unwrap().to_string(),
                msg: to_json_binary(&EclipStakingExecuteMsg::BondFor {
                    address_and_amount_list: vec![(sender.clone(), rewards.beclip)],
                })?,
                funds: coins(rewards.beclip.u128(), cfg.eclip.to_string()),
            }));
        }

        if !rewards.eclip.is_zero() {
            msgs.push(
                cfg.eclip
                    .with_balance(rewards.eclip)
                    .into_msg(sender.clone())?,
            );
        }
    }

    Ok(Response::new().add_messages(msgs))
}

pub fn _claim_lp_rewards(
    deps: DepsMut,
    env: Env,
    sender: String,
    duration: u64,
    assets: Option<Vec<AssetInfo>>,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let state = LP_LOCKUP_STATE.load(deps.storage)?;
    let blacklist = BLACK_LIST.load(deps.storage).unwrap_or_default();
    let mut blacklist_rewards = BLACK_LIST_REWARDS.load(deps.storage).unwrap_or_default();

    ensure_eq!(
        cfg.claims_allowed,
        true,
        ContractError::ClaimRewardNotAllowed {}
    );

    // check if there are duplicated assets in list
    let assets_list = assets
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|a| a.to_string());
    ensure!(
        has_unique_elements(assets_list),
        ContractError::DuplicatedAssets {}
    );

    let mut user_lockup_info = LP_USER_LOCKUP_INFO.load(deps.storage, (&sender, duration))?;
    if user_lockup_info.total_lp_staked.is_zero() {
        user_lockup_info.total_lp_staked = user_lockup_info
            .xastro_amount_in_lockups
            .multiply_ratio(state.total_lp_lockdrop, state.total_xastro);
    }
    user_lockup_info.lockdrop_incentives = get_user_lp_lockdrop_incentives(
        deps.as_ref(),
        user_lockup_info.lockdrop_incentives,
        user_lockup_info.xastro_amount_in_lockups,
        duration,
    )?;

    let pending_lockdrop_incentives = calculate_pending_lockdrop_incentives(
        deps.as_ref(),
        env.block.time.seconds(),
        user_lockup_info.lockdrop_incentives.clone(),
    )?;

    let lp_staking_rewards =
        calculate_lp_total_rewards(deps.as_ref(), env.contract.address.to_string())?;

    let mut msgs = vec![];

    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.lp_staking.clone().unwrap().to_string(),
        msg: to_json_binary(&LpExecuteMsg::Claim { assets: None })?,
        funds: vec![],
    }));

    let updated_lp_reward_weights =
        calculate_updated_lp_reward_weights(deps.as_ref(), &lp_staking_rewards)?;
    LP_STAKING_REWARD_WEIGHTS.save(deps.storage, &updated_lp_reward_weights)?;
    let user_rewards = calculate_lp_staking_user_rewards(
        deps.as_ref(),
        updated_lp_reward_weights.clone(),
        pending_lockdrop_incentives.clone(),
        user_lockup_info.clone(),
    )?;

    let mut astro_rewards = Uint128::zero();
    let mut beclip_rewards = Uint128::zero();
    let mut eclip_rewards = Uint128::zero();

    if let Some(assets) = assets {
        for asset in assets {
            if asset.equal(&AssetInfo::NativeToken {
                denom: cfg.astro_token.clone(),
            }) {
                user_lockup_info.reward_weights.astro = updated_lp_reward_weights.astro;
                astro_rewards += user_rewards.astro;
            }
            if asset.equal(&cfg.beclip) {
                user_lockup_info.reward_weights.beclip = updated_lp_reward_weights.beclip;
                user_lockup_info.lockdrop_incentives.beclip.claimed +=
                    pending_lockdrop_incentives.beclip;
                beclip_rewards += user_rewards.beclip;
            }
            if asset.equal(&cfg.eclip) {
                user_lockup_info.reward_weights.eclip = updated_lp_reward_weights.eclip;
                user_lockup_info.lockdrop_incentives.eclip.claimed +=
                    pending_lockdrop_incentives.eclip;
                eclip_rewards += user_rewards.eclip;
            }
        }
    } else {
        user_lockup_info.reward_weights = updated_lp_reward_weights;
        user_lockup_info.lockdrop_incentives.beclip.claimed += pending_lockdrop_incentives.beclip;
        user_lockup_info.lockdrop_incentives.eclip.claimed += pending_lockdrop_incentives.eclip;
        astro_rewards += user_rewards.astro;
        beclip_rewards += user_rewards.beclip;
        eclip_rewards += user_rewards.eclip;
    }
    if blacklist.contains(&sender) {
        blacklist_rewards.astro += astro_rewards;
        blacklist_rewards.eclip += eclip_rewards;
        blacklist_rewards.beclip += beclip_rewards;
        BLACK_LIST_REWARDS.save(deps.storage, &blacklist_rewards)?;
    } else {
        if !astro_rewards.is_zero() {
            msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: sender.clone(),
                amount: vec![coin(astro_rewards.u128(), cfg.astro_token.clone())],
            }));
        }
        if !beclip_rewards.is_zero() {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.eclip_staking.unwrap().to_string(),
                msg: to_json_binary(&EclipStakingExecuteMsg::BondFor {
                    address_and_amount_list: vec![(sender.clone(), beclip_rewards)],
                })?,
                funds: coins(beclip_rewards.u128(), cfg.eclip.to_string()),
            }));
        }

        if !eclip_rewards.is_zero() {
            msgs.push(
                cfg.eclip
                    .with_balance(eclip_rewards)
                    .into_msg(sender.clone())?,
            );
        }
    }

    LP_USER_LOCKUP_INFO.save(deps.storage, (&sender, duration), &user_lockup_info)?;

    Ok(Response::new().add_messages(msgs))
}

pub fn _claim_all_single_sided_rewards(
    mut deps: DepsMut,
    env: Env,
    sender: String,
    durations: Option<Vec<u64>>,
    with_flexible: bool,
    assets: Option<Vec<AssetInfo>>,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let blacklist = BLACK_LIST.load(deps.storage).unwrap_or_default();
    let mut blacklist_rewards = BLACK_LIST_REWARDS.load(deps.storage).unwrap_or_default();
    let eclipastro_token = cfg.eclipastro_token.clone().unwrap();
    let block_time = env.block.time.seconds();

    ensure!(cfg.claims_allowed, ContractError::ClaimRewardNotAllowed {});
    let assets_list = assets
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|a| a.to_string());
    ensure!(
        has_unique_elements(assets_list),
        ContractError::DuplicatedAssets {}
    );

    let mut msgs = vec![];

    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.single_sided_staking.clone().unwrap().to_string(),
        msg: to_json_binary(&SingleSidedExecuteMsg::ClaimAll {
            with_flexible: true,
            assets: None,
        })?,
        funds: vec![],
    }));

    let mut eclipastro_rewards = Uint128::zero();
    let mut beclip_rewards = Uint128::zero();
    let mut eclip_rewards = Uint128::zero();

    for lock_cfg in cfg.lock_configs.iter() {
        let duration = lock_cfg.duration;
        // skip if user doesn't want to claim rewards from this duration
        if let Some(ref durations) = durations {
            if !durations.iter().any(|d| d == &duration) {
                continue;
            }
        }
        if !with_flexible && duration == 0 {
            continue;
        }
        let locked_at = if duration == 0u64 {
            None
        } else {
            Some(cfg.countdown_start_at)
        };
        let rewards = calculate_single_user_rewards(
            deps.branch(),
            sender.clone(),
            duration,
            block_time,
            locked_at,
            assets.clone(),
        )?;
        eclipastro_rewards += rewards.eclipastro;
        beclip_rewards += rewards.beclip;
        eclip_rewards += rewards.eclip;
    }

    if blacklist.contains(&sender) {
        blacklist_rewards.eclipastro += eclipastro_rewards;
        blacklist_rewards.eclip += eclip_rewards;
        blacklist_rewards.beclip += beclip_rewards;
        BLACK_LIST_REWARDS.save(deps.storage, &blacklist_rewards)?;
    } else {
        // add message to claim rewards and incentives
        if !eclipastro_rewards.is_zero() {
            msgs.push(
                eclipastro_token
                    .with_balance(eclipastro_rewards)
                    .into_msg(sender.clone())?,
            );
        }
        if !beclip_rewards.is_zero() {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.eclip_staking.unwrap().to_string(),
                msg: to_json_binary(&EclipStakingExecuteMsg::BondFor {
                    address_and_amount_list: vec![(sender.clone(), beclip_rewards)],
                })?,
                funds: coins(beclip_rewards.u128(), cfg.eclip.to_string()),
            }));
        }
        if !eclip_rewards.is_zero() {
            msgs.push(cfg.eclip.with_balance(eclip_rewards).into_msg(sender)?);
        }
    }

    Ok(Response::new().add_messages(msgs))
}

pub fn _claim_all_lp_rewards(
    deps: DepsMut,
    env: Env,
    sender: String,
    durations: Option<Vec<u64>>,
    with_flexible: bool,
    assets: Option<Vec<AssetInfo>>,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let blacklist = BLACK_LIST.load(deps.storage).unwrap_or_default();
    let mut blacklist_rewards = BLACK_LIST_REWARDS.load(deps.storage).unwrap_or_default();
    let state = LP_LOCKUP_STATE.load(deps.storage)?;

    ensure_eq!(
        cfg.claims_allowed,
        true,
        ContractError::ClaimRewardNotAllowed {}
    );

    // check asset list includes duplicated assets.
    let assets_list = assets
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|a| a.to_string());
    ensure!(
        has_unique_elements(assets_list),
        ContractError::DuplicatedAssets {}
    );

    // calculate contract's lp staking rewards
    let lp_staking_rewards =
        calculate_lp_total_rewards(deps.as_ref(), env.contract.address.to_string())?;

    // update contract reward_weights
    let updated_lp_reward_weights =
        calculate_updated_lp_reward_weights(deps.as_ref(), &lp_staking_rewards)?;
    LP_STAKING_REWARD_WEIGHTS.save(deps.storage, &updated_lp_reward_weights)?;

    let mut msgs = vec![];

    // claim rewards from lp staking vault
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.lp_staking.clone().unwrap().to_string(),
        msg: to_json_binary(&LpExecuteMsg::Claim { assets: None })?,
        funds: vec![],
    }));

    let mut astro_rewards = Uint128::zero();
    let mut beclip_rewards = Uint128::zero();
    let mut eclip_rewards = Uint128::zero();

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

        if user_lockup_info.total_lp_staked.is_zero() {
            user_lockup_info.total_lp_staked = user_lockup_info
                .xastro_amount_in_lockups
                .multiply_ratio(state.total_lp_lockdrop, state.total_xastro);
        }

        // calculate user lockdrop incentives
        user_lockup_info.lockdrop_incentives = get_user_lp_lockdrop_incentives(
            deps.as_ref(),
            user_lockup_info.lockdrop_incentives,
            user_lockup_info.xastro_amount_in_lockups,
            duration,
        )?;
        let pending_lockdrop_incentives = calculate_pending_lockdrop_incentives(
            deps.as_ref(),
            env.block.time.seconds(),
            user_lockup_info.lockdrop_incentives.clone(),
        )?;

        // update user claimed lockdrop incentives, reward_weights, assets amount to claim
        let user_rewards = calculate_lp_staking_user_rewards(
            deps.as_ref(),
            updated_lp_reward_weights.clone(),
            pending_lockdrop_incentives.clone(),
            user_lockup_info.clone(),
        )?;
        if let Some(assets) = assets.clone() {
            if assets.iter().any(|a| a.equal(&cfg.beclip)) {
                beclip_rewards += user_rewards.beclip;
                user_lockup_info.lockdrop_incentives.beclip.claimed +=
                    &pending_lockdrop_incentives.beclip;
                user_lockup_info.reward_weights.beclip = updated_lp_reward_weights.clone().beclip;
            }
            if assets.iter().any(|a| a.equal(&cfg.beclip)) {
                eclip_rewards += user_rewards.eclip;
                user_lockup_info.lockdrop_incentives.eclip.claimed +=
                    &pending_lockdrop_incentives.eclip;
                user_lockup_info.reward_weights.eclip = updated_lp_reward_weights.clone().eclip;
            }
            if assets.iter().any(|a| {
                a.equal(&AssetInfo::NativeToken {
                    denom: cfg.astro_token.clone(),
                })
            }) {
                astro_rewards += user_rewards.astro;
                user_lockup_info.reward_weights.astro = updated_lp_reward_weights.clone().astro;
            }
        } else {
            astro_rewards += user_rewards.astro;
            beclip_rewards += user_rewards.beclip;
            eclip_rewards += user_rewards.eclip;
            user_lockup_info.lockdrop_incentives.eclip.claimed += pending_lockdrop_incentives.eclip;
            user_lockup_info.lockdrop_incentives.beclip.claimed +=
                pending_lockdrop_incentives.beclip;
            user_lockup_info.reward_weights = updated_lp_reward_weights.clone();
        }

        LP_USER_LOCKUP_INFO.save(deps.storage, (&sender, duration), &user_lockup_info)?;
    }

    if blacklist.contains(&sender) {
        blacklist_rewards.astro += astro_rewards;
        blacklist_rewards.eclip += eclip_rewards;
        blacklist_rewards.beclip += beclip_rewards;
        BLACK_LIST_REWARDS.save(deps.storage, &blacklist_rewards)?;
    } else {
        if !astro_rewards.is_zero() {
            msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: sender.clone(),
                amount: vec![coin(astro_rewards.u128(), cfg.astro_token.clone())],
            }));
        }
        if !beclip_rewards.is_zero() {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.eclip_staking.unwrap().to_string(),
                msg: to_json_binary(&EclipStakingExecuteMsg::BondFor {
                    address_and_amount_list: vec![(sender.clone(), beclip_rewards)],
                })?,
                funds: coins(beclip_rewards.u128(), cfg.eclip.to_string()),
            }));
        }
        if !eclip_rewards.is_zero() {
            msgs.push(cfg.eclip.with_balance(eclip_rewards).into_msg(sender)?);
        }
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

pub fn try_claim_blacklist_rewards(mut deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let treasury = cfg.dao_treasury_address.unwrap().to_string();
    let blacklist = BLACK_LIST.load(deps.storage)?;
    for user in blacklist {
        _claim_all_single_sided_rewards(
            deps.branch(),
            env.clone(),
            user.clone(),
            None,
            true,
            None,
        )?;
        _claim_all_lp_rewards(deps.branch(), env.clone(), user.clone(), None, true, None)?;
    }
    let blacklist_rewards = BLACK_LIST_REWARDS.load(deps.storage)?;
    let mut msgs = vec![
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.single_sided_staking.clone().unwrap().to_string(),
            msg: to_json_binary(&SingleSidedExecuteMsg::ClaimAll {
                with_flexible: true,
                assets: None,
            })?,
            funds: vec![],
        }),
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.lp_staking.clone().unwrap().to_string(),
            msg: to_json_binary(&LpExecuteMsg::Claim { assets: None })?,
            funds: vec![],
        }),
    ];
    if !blacklist_rewards.astro.is_zero() {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: treasury.clone(),
            amount: vec![coin(
                blacklist_rewards.astro.u128(),
                cfg.astro_token.clone(),
            )],
        }));
    }
    if !(blacklist_rewards.eclip + blacklist_rewards.beclip).is_zero() {
        msgs.push(
            cfg.eclip
                .with_balance(blacklist_rewards.eclip + blacklist_rewards.beclip)
                .into_msg(treasury)?,
        );
    }
    Ok(Response::new().add_messages(msgs))
}

pub fn _unlock_single_lockup(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    sender: String,
    duration: u64,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let block_time = env.block.time.seconds();
    let cfg = CONFIG.load(deps.storage)?;
    let mut lockup_info = SINGLE_LOCKUP_INFO.load(deps.storage, duration)?;
    if !check_lockdrop_ended(deps.as_ref(), block_time).unwrap() {
        let mut user_lockup_info = SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&sender, duration))?;
        let mut withdraw_amount = calculate_max_withdrawal_amount_allowed(
            block_time,
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
        if check_withdrawal_window(deps.as_ref(), block_time).unwrap() {
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

        Ok(Response::new()
            .add_message(msg)
            .add_attribute("action", "withdraw")
            .add_attribute("withdraw_amount", withdraw_amount.to_string()))
    } else {
        ensure!(cfg.claims_allowed, ContractError::ClaimRewardNotAllowed {});
        let response =
            _claim_single_sided_rewards(deps.branch(), env, sender.clone(), duration, None)?;
        let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
        let mut user_lockup_info = SINGLE_USER_LOCKUP_INFO.load(deps.storage, (&sender, duration))?;
        if user_lockup_info.total_eclipastro_staked.is_zero() {
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
                contract_addr: cfg.single_sided_staking.unwrap().to_string(),
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
                contract_addr: cfg.single_sided_staking.unwrap().to_string(),
                msg: to_json_binary(&SingleSidedExecuteMsg::Unstake {
                    duration,
                    locked_at: Some(cfg.countdown_start_at),
                    amount: Some(withdraw_amount),
                    recipient: Some(sender.clone()),
                })?,
                funds: vec![],
            }));
        }

        SINGLE_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
        SINGLE_USER_LOCKUP_INFO.save(deps.storage, (&sender, duration), &user_lockup_info)?;

        Ok(response
            .add_messages(msgs)
            .add_attribute("action", "withdraw")
            .add_attribute("withdraw_amount", withdraw_amount.to_string()))
    }
}

pub fn _unlock_lp_lockup(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    sender: String,
    duration: u64,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let current_time = env.block.time.seconds();
    let cfg = CONFIG.load(deps.storage)?;
    let mut lockup_info = LP_LOCKUP_INFO.load(deps.storage, duration)?;
    if !check_lockdrop_ended(deps.as_ref(), current_time).unwrap() {
        let mut user_lockup_info = LP_USER_LOCKUP_INFO.load(deps.storage, (&sender, duration))?;
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
        if check_withdrawal_window(deps.as_ref(), current_time).unwrap() {
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

        Ok(Response::new()
            .add_message(msg)
            .add_attribute("action", "withdraw")
            .add_attribute("withdraw_amount", withdraw_amount.to_string()))
    } else {
        ensure!(cfg.claims_allowed, ContractError::ClaimRewardNotAllowed {});
        let response = _claim_lp_rewards(deps.branch(), env, sender.clone(), duration, None)?;
        let mut user_lockup_info = LP_USER_LOCKUP_INFO.load(deps.storage, (&sender, duration))?;
        let state = LP_LOCKUP_STATE.load(deps.storage)?;
        if user_lockup_info.total_lp_staked.is_zero() {
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

        let penalty_amount = calculate_penalty_amount(deps.as_ref(), withdraw_amount, duration, current_time)?;

        let lp_token = cfg.lp_token.unwrap();
        let mut msgs = vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.lp_staking.unwrap().to_string(),
                msg: to_json_binary(&LpExecuteMsg::Unstake {
                    amount: withdraw_amount,
                    recipient: None,
                })?,
                funds: vec![],
            }),
            lp_token
                .with_balance(withdraw_amount - penalty_amount)
                .into_msg(sender.clone())?,
        ];
        if !penalty_amount.is_zero() {
            msgs.push(
                lp_token
                    .with_balance(penalty_amount)
                    .into_msg(cfg.dao_treasury_address.unwrap().to_string())?,
            );
        }

        LP_LOCKUP_INFO.save(deps.storage, duration, &lockup_info)?;
        LP_USER_LOCKUP_INFO.save(deps.storage, (&sender, duration), &user_lockup_info)?;

        Ok(response
            .add_messages(msgs)
            .add_attribute("action", "withdraw")
            .add_attribute(
                "withdraw_amount",
                (withdraw_amount - penalty_amount).to_string(),
            ))
    }
}

pub fn astro_convert_msg(astro_staking: String, coin: &Coin) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: astro_staking,
        msg: to_json_binary(&AstroStakingExecuteMsg::Enter { receiver: None })?,
        funds: vec![coin.clone()],
    }))
}

pub fn convert_eclipastro_msg(voter: String, coin: &Coin) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: voter,
        msg: to_json_binary(&VoterExecuteMsg::SwapToEclipAstro {})?,
        funds: vec![coin.clone()],
    }))
}

pub fn send_token_msg(
    contract: String,
    recipient: String,
    amount: Uint128,
    funds: Vec<Coin>,
) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract,
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer { recipient, amount })?,
        funds,
    }))
}

pub fn check_native_token_denom(querier: &QuerierWrapper, denom: String) -> StdResult<bool> {
    let total_supply = query_native_token_supply(querier, denom)?;
    Ok(!total_supply.amount.is_zero())
}

pub fn calculate_single_user_rewards(
    deps: DepsMut,
    sender: String,
    duration: u64,
    block_time: u64,
    locked_at: Option<u64>,
    assets: Option<Vec<AssetInfo>>,
) -> Result<UserReward, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let state = SINGLE_LOCKUP_STATE.load(deps.storage)?;
    let mut user_lockup_info = SINGLE_USER_LOCKUP_INFO
        .load(deps.storage, (&sender, duration))
        .unwrap_or_default();
    if user_lockup_info.total_eclipastro_staked.is_zero() {
        user_lockup_info.total_eclipastro_staked = user_lockup_info
            .xastro_amount_in_lockups
            .multiply_ratio(state.total_eclipastro_lockup, state.total_xastro);
    }
    user_lockup_info.lockdrop_incentives = get_user_single_lockdrop_incentives(
        deps.as_ref(),
        user_lockup_info.lockdrop_incentives,
        user_lockup_info.xastro_amount_in_lockups,
        duration,
    )?;
    let pending_lockdrop_incentives = calculate_pending_lockdrop_incentives(
        deps.as_ref(),
        block_time,
        user_lockup_info.lockdrop_incentives.clone(),
    )?;

    if user_lockup_info.total_eclipastro_staked.is_zero() {
        user_lockup_info.total_eclipastro_staked = user_lockup_info
            .xastro_amount_in_lockups
            .multiply_ratio(state.total_eclipastro_lockup, state.total_xastro);
    }

    let user_rewards = query_user_single_rewards(
        deps.as_ref(),
        user_lockup_info.total_eclipastro_staked - user_lockup_info.total_eclipastro_withdrawed,
        duration,
        locked_at,
        user_lockup_info
            .last_claimed
            .unwrap_or(cfg.countdown_start_at),
    )?;

    let eclipastro_rewards = if assets.is_none()
        || assets
            .clone()
            .unwrap()
            .contains(&cfg.eclipastro_token.unwrap())
    {
        user_rewards.eclipastro + user_lockup_info.unclaimed_rewards.eclipastro
    } else {
        user_lockup_info.unclaimed_rewards.eclipastro += user_rewards.eclipastro;
        Uint128::zero()
    };

    let beclip_rewards = if assets.is_none() || assets.clone().unwrap().contains(&cfg.beclip) {
        let beclip_rewards = user_rewards.beclip
            + user_lockup_info.unclaimed_rewards.beclip
            + pending_lockdrop_incentives.beclip;
        user_lockup_info.unclaimed_rewards.beclip = Uint128::zero();
        user_lockup_info.lockdrop_incentives.beclip.claimed += pending_lockdrop_incentives.beclip;
        beclip_rewards
    } else {
        user_lockup_info.unclaimed_rewards.beclip += user_rewards.beclip;
        Uint128::zero()
    };

    let eclip_rewards = if assets.is_none() || assets.clone().unwrap().contains(&cfg.eclip) {
        let eclip_rewards = user_rewards.eclip
            + user_lockup_info.unclaimed_rewards.eclip
            + pending_lockdrop_incentives.eclip;
        user_lockup_info.unclaimed_rewards.eclip = Uint128::zero();
        user_lockup_info.lockdrop_incentives.eclip.claimed += pending_lockdrop_incentives.eclip;
        eclip_rewards
    } else {
        user_lockup_info.unclaimed_rewards.eclip += user_rewards.eclip;
        Uint128::zero()
    };
    user_lockup_info.last_claimed = Some(block_time);

    SINGLE_USER_LOCKUP_INFO.save(deps.storage, (&sender, duration), &user_lockup_info)?;
    Ok(UserReward {
        eclipastro: eclipastro_rewards,
        beclip: beclip_rewards,
        eclip: eclip_rewards,
    })
}
