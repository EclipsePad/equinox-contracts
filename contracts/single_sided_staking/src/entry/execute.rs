use astroport::asset::{AssetInfo, AssetInfoExt};
use cosmwasm_std::{
    attr, coins, ensure, ensure_eq, to_json_binary, BankMsg, Coin, CosmosMsg, Decimal256, DepsMut,
    Env, MessageInfo, Response, Storage, Uint128, WasmMsg,
};
use cw_utils::one_coin;

use crate::{
    config::{MAX_PROPOSAL_TTL, ONE_DAY},
    entry::query::{calculate_penalty, calculate_total_user_reward},
    error::ContractError,
    state::{
        RewardWeights, TotalStakingByDuration, UserStaked, ALLOWED_USERS, CONFIG, LAST_CLAIM_TIME,
        OWNER, OWNERSHIP_PROPOSAL, PENDING_ECLIPASTRO_REWARDS, REWARD_CONFIG, REWARD_WEIGHTS,
        STAKING_DURATION_BY_END_TIME, TOTAL_STAKING, TOTAL_STAKING_BY_DURATION, USER_STAKED,
    },
};

use equinox_msg::{
    single_sided_staking::{
        CallbackMsg, OwnershipProposal, RestakeData, RewardDetails, UpdateConfigMsg, UserReward,
    },
    utils::has_unique_elements,
    voter::msg::ExecuteMsg as VoterExecuteMsg,
};

use super::query::{
    calculate_eclipastro_reward, calculate_user_reward, query_eclipastro_pending_rewards,
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
    if let Some(voter) = new_config.voter {
        config.voter = deps.api.addr_validate(&voter)?;
        res = res.add_attribute("voter", voter);
    }
    if let Some(treasury) = new_config.treasury {
        config.treasury = deps.api.addr_validate(&treasury)?;
        res = res.add_attribute("treasury", treasury);
    }
    if let Some(timelock_config) = new_config.timelock_config {
        config.timelock_config.clone_from(&timelock_config);
        res = res.add_attribute("timelock_config", "update timelock config")
    }
    CONFIG.save(deps.storage, &config)?;
    Ok(res)
}

/// Update reward config
/// Only owner
pub fn update_reward_config(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    details: Option<RewardDetails>,
    reward_end_time: Option<u64>,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let config = CONFIG.load(deps.storage)?;
    let mut reward_config = REWARD_CONFIG.load(deps.storage)?;

    let current_time = env.block.time.seconds();
    let mut msgs = vec![];

    if let Some(details) = details {
        // update all total_staking_by_duration and reward_weights by now
        update_reward_weights(deps.branch(), env)?;
        reward_config.details = details;
        let pending_eclipastro_rewards =
            query_eclipastro_pending_rewards(deps.as_ref(), config.voter.to_string())?;

        if !pending_eclipastro_rewards.is_zero() {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.voter.to_string(),
                msg: to_json_binary(&VoterExecuteMsg::ClaimAstroRewards {})?,
                funds: vec![],
            }));
            PENDING_ECLIPASTRO_REWARDS.save(
                deps.storage,
                current_time,
                &pending_eclipastro_rewards,
            )?;
        }
        LAST_CLAIM_TIME.save(deps.storage, &current_time)?;
    }
    if let Some(reward_end_time) = reward_end_time {
        ensure!(
            reward_end_time > current_time,
            ContractError::InvalidEndTime {}
        );
        reward_config.reward_end_time = Some(reward_end_time);
    }
    REWARD_CONFIG.save(deps.storage, &reward_config)?;
    Ok(Response::new()
        .add_attribute("action", "update config")
        .add_messages(msgs))
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
            ContractError::DuplicatedAddress(user)
        );
        ALLOWED_USERS.remove(deps.storage, &user);
    }
    Ok(Response::new().add_attribute("action", "update allowed users"))
}

pub fn propose_new_owner(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_owner: String,
    expires_in: u64,
) -> Result<Response, ContractError> {
    // only owner can propose new owner
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let new_owner_addr = deps.api.addr_validate(&new_owner)?;

    // Check that the new owner is not the same as the current one
    ensure_eq!(
        OWNER.is_admin(deps.as_ref(), &new_owner_addr).unwrap(),
        false,
        ContractError::SameOwner {}
    );

    if MAX_PROPOSAL_TTL < expires_in {
        return Err(ContractError::ExpiresInErr(MAX_PROPOSAL_TTL));
    }

    let new_proposal = OwnershipProposal {
        owner: new_owner_addr,
        ttl: env.block.time.seconds() + expires_in,
    };

    OWNERSHIP_PROPOSAL.save(deps.storage, &new_proposal)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "propose_new_owner"),
        attr("new_owner", new_owner),
    ]))
}

pub fn drop_ownership_proposal(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // only owner can drop ownership proposal
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;

    OWNERSHIP_PROPOSAL.remove(deps.storage);

    Ok(Response::new().add_attributes(vec![attr("action", "drop_ownership_proposal")]))
}

pub fn claim_ownership(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // only owner can drop ownership proposal
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;

    let proposal = OWNERSHIP_PROPOSAL.load(deps.storage)?;

    ensure!(
        env.block.time.seconds() > proposal.ttl,
        ContractError::OwnershipProposalExpired {}
    );

    OWNER.set(deps.branch(), Some(proposal.owner.clone()))?;

    OWNERSHIP_PROPOSAL.remove(deps.storage);

    Ok(Response::new().add_attributes(vec![
        attr("action", "claim_ownership"),
        attr("new_owner", proposal.owner),
    ]))
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
            let eclipastro_balance = deps
                .querier
                .query_balance(env.contract.address.to_string(), config.token)?;
            _stake(
                deps,
                env,
                duration,
                sender,
                recipient,
                eclipastro_balance.amount - prev_eclipastro_balance,
            )
        }
    }
}

// convert astro/xastro to eclipastro and stake them
pub fn stake(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    duration: u64,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    let recipient = recipient.unwrap_or(sender.clone());
    let received_asset = one_coin(&info)?;
    let config = CONFIG.load(deps.storage)?;
    if received_asset.denom != config.token {
        let eclipastro_balance = deps
            .querier
            .query_balance(env.contract.address.to_string(), config.token)?;
        return Ok(Response::new().add_messages(vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.voter.to_string(),
                msg: to_json_binary(&VoterExecuteMsg::SwapToEclipAstro {})?,
                funds: vec![received_asset],
            }),
            CallbackMsg::Convert {
                prev_eclipastro_balance: eclipastro_balance.amount,
                duration,
                sender,
                recipient,
            }
            .to_cosmos_msg(&env)?,
        ]));
    }
    _stake(
        deps.branch(),
        env,
        duration,
        sender,
        recipient,
        received_asset.amount,
    )
}

/// stake eclipASTRO
pub fn _stake(
    mut deps: DepsMut,
    env: Env,
    lock_duration: u64,
    sender: String,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let block_time = env.block.time.seconds();
    let config = CONFIG.load(deps.storage)?;
    let mut total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();

    // if flexible, locked_at is 0, else current time
    let locked_at = match lock_duration {
        0u64 => 0u64,
        _ => block_time,
    };

    // check if duration is exists in config
    ensure!(
        config
            .timelock_config
            .into_iter()
            .any(|i| i.duration == lock_duration),
        ContractError::NoLockingPeriodFound(lock_duration)
    );
    // update total_staking_by_duration, reward_weights by now
    let reward_weights = update_reward_weights(deps.branch(), env.clone())?;
    // calculate user's rewards
    let (mut user_staking, response) = _claim_single(
        deps.branch(),
        env,
        sender,
        lock_duration,
        locked_at,
        reward_weights,
        None,
    )?;
    user_staking.staked = user_staking.staked.checked_add(amount).unwrap();
    total_staking = total_staking.checked_add(amount).unwrap();

    USER_STAKED.save(
        deps.storage,
        (&recipient, lock_duration, locked_at),
        &user_staking,
    )?;
    TOTAL_STAKING.save(deps.storage, &total_staking)?;
    TotalStakingByDuration::add(deps.storage, amount, lock_duration, block_time)?;
    Ok(response
        .add_attribute("action", "stake eclipastro")
        .add_attribute("duration", lock_duration.to_string())
        .add_attribute("locked_at", locked_at.to_string())
        .add_attribute("amount", amount.to_string()))
}

pub fn restake(
    mut deps: DepsMut,
    env: Env,
    funds: Vec<Coin>,
    data: RestakeData,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let from_duration = data.from_duration;
    let locked_at = data.locked_at;
    let to_duration = data.to_duration;
    let sender = data.sender;
    let amount = data.amount;
    let recipient = data.recipient;
    let block_time = env.block.time.seconds();
    let is_allowed_user = ALLOWED_USERS
        .load(deps.storage, &sender)
        .unwrap_or_default();
    let mut add_amount = Uint128::zero();

    if !funds.is_empty() {
        ensure!(
            funds.len() == 1 && funds[0].denom == config.token,
            ContractError::InvalidAsset {}
        );
        add_amount = funds[0].amount;
    }

    // update total_staking_by_duration, reward_weights by now
    let reward_weights = update_reward_weights(deps.branch(), env.clone())?;
    // claim all assets
    let (user_staking_from, response) = _claim_single(
        deps.branch(),
        env,
        sender.to_string(),
        from_duration,
        locked_at,
        reward_weights.clone(),
        None,
    )?;
    let mut user_staking_to = USER_STAKED
        .load(deps.storage, (&recipient, to_duration, block_time))
        .unwrap_or_default();
    let mut total_staking = TOTAL_STAKING.load(deps.storage)?;
    // to duration must be longer than from duration
    ensure!(
        from_duration <= to_duration,
        ContractError::ExtendDurationErr(from_duration, to_duration)
    );
    // check to duration is exist
    ensure!(
        config
            .timelock_config
            .into_iter()
            .any(|i| i.duration == to_duration),
        ContractError::NoLockingPeriodFound(to_duration)
    );
    // check if there is restaking amount
    ensure!(
        !user_staking_from.staked.is_zero(),
        ContractError::NoLockedAmount {}
    );
    // only allowed users can partial restake
    if let Some(amount) = amount {
        ensure!(is_allowed_user, ContractError::NotAllowed(sender));
        ensure!(
            !user_staking_from.staked >= amount,
            ContractError::ExceedAmount {}
        );
    }
    // if additional amount, increate total staking
    if !add_amount.is_zero() {
        total_staking += add_amount;
        TOTAL_STAKING.save(deps.storage, &total_staking)?;
    }
    let restake_amount = amount.unwrap_or(user_staking_from.staked);
    TotalStakingByDuration::sub(
        deps.storage,
        restake_amount,
        from_duration,
        locked_at,
        block_time,
    )?;
    TotalStakingByDuration::add(
        deps.storage,
        restake_amount + add_amount,
        to_duration,
        block_time,
    )?;
    if amount.is_none() || amount.unwrap().eq(&user_staking_from.staked) {
        USER_STAKED.remove(deps.storage, (&sender, from_duration, locked_at));
    } else {
        USER_STAKED.save(
            deps.storage,
            (&sender, from_duration, locked_at),
            &UserStaked {
                staked: user_staking_from.staked - amount.unwrap(),
                reward_weights: reward_weights.clone(),
            },
        )?;
    }
    user_staking_to.staked += restake_amount + add_amount;
    user_staking_to.reward_weights = reward_weights.clone();
    USER_STAKED.save(
        deps.storage,
        (&recipient, to_duration, block_time),
        &user_staking_to,
    )?;

    Ok(response
        .add_attribute("action", "add lock")
        .add_attribute("user", sender.to_string())
        .add_attribute("amount", add_amount.to_string())
        .add_attribute("action", "extend duration")
        .add_attribute("from", from_duration.to_string())
        .add_attribute("user", sender)
        .add_attribute("amount", restake_amount)
        .add_attribute("to", to_duration.to_string())
        .add_attribute("receiver", recipient))
}
/// claim user rewards message, update user reward weights
pub fn _claim_single(
    deps: DepsMut,
    env: Env,
    sender: String,
    duration: u64,
    locked_at: u64,
    reward_weights: RewardWeights,
    assets: Option<Vec<AssetInfo>>,
) -> Result<(UserStaked, Response), ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let block_time = env.block.time.seconds();
    let reward_config = REWARD_CONFIG.load(deps.storage)?;
    let mut user_staking = USER_STAKED
        .load(deps.storage, (&sender, duration, locked_at))
        .unwrap_or_default();
    let total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    // if assets is some, get it
    let assets_list = assets
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|a| a.to_string());
    // check if there is duplicated asset
    ensure!(
        has_unique_elements(assets_list),
        ContractError::DuplicatedAssets {}
    );
    // calculate user reward by duration and lock time
    let user_reward = calculate_user_reward(
        deps.as_ref(),
        sender.clone(),
        duration,
        locked_at,
        block_time,
    )?;
    let mut user_reward_to_claim = UserReward::default();
    // if assets is exist, only claim those, else claim all
    if let Some(assets) = assets {
        for asset in assets {
            if asset.equal(&AssetInfo::NativeToken {
                denom: config.token.clone(),
            }) {
                user_staking
                    .reward_weights
                    .eclipastro
                    .clone_from(&reward_weights.eclipastro);
                user_reward_to_claim.eclipastro = user_reward.eclipastro;
            }
            if asset.equal(&reward_config.details.eclip.info) {
                user_staking.reward_weights.eclip = reward_weights.eclip;
                user_reward_to_claim.eclip = user_reward.eclip;
            }
            if asset.equal(&reward_config.details.beclip.info) {
                user_staking.reward_weights.beclip = reward_weights.beclip;
                user_reward_to_claim.beclip = user_reward.beclip;
            }
        }
    } else {
        user_reward_to_claim = user_reward;
        user_staking.reward_weights = reward_weights.clone();
    }
    // save user reward weights
    USER_STAKED.save(deps.storage, (&sender, duration, locked_at), &user_staking)?;
    // update last claim time
    let mut last_claim_time = block_time;
    if let Some(reward_end_time) = reward_config.reward_end_time {
        if reward_end_time < block_time {
            last_claim_time = reward_end_time;
        }
    }
    LAST_CLAIM_TIME.save(deps.storage, &last_claim_time)?;
    if total_staking.is_zero() {
        let response: Response = Response::new();
        return Ok((user_staking, response));
    }
    Ok((
        user_staking,
        _claim(deps, env, sender, user_reward_to_claim)?,
    ))
}

pub fn _claim(
    deps: DepsMut,
    env: Env,
    sender: String,
    rewards: UserReward,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let reward_config = REWARD_CONFIG.load(deps.storage)?;

    let pending_eclipastro_rewards =
        query_eclipastro_pending_rewards(deps.as_ref(), config.voter.to_string())?;

    let mut response = Response::new().add_attribute("action", "claim rewards");
    let mut msgs = vec![];

    if !pending_eclipastro_rewards.is_zero() {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.voter.to_string(),
            msg: to_json_binary(&VoterExecuteMsg::ClaimAstroRewards {})?,
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

    if !rewards.eclipastro.is_zero() {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: sender.clone(),
            amount: coins(rewards.eclipastro.u128(), config.token),
        }));
        response = response
            .add_attribute("action", "claim user eclipastro reward")
            .add_attribute("amount", rewards.eclipastro.to_string());
    }

    if !rewards.beclip.is_zero() {
        msgs.push(
            reward_config
                .details
                .beclip
                .info
                .with_balance(rewards.beclip)
                .into_msg(sender.clone())?,
        );
        response = response
            .add_attribute("action", "claim user beclip reward")
            .add_attribute("amount", rewards.beclip.to_string());
    }

    if !rewards.eclip.is_zero() {
        msgs.push(
            reward_config
                .details
                .eclip
                .info
                .with_balance(rewards.eclip)
                .into_msg(sender)?,
        );
        response = response
            .add_attribute("action", "claim user eclip reward")
            .add_attribute("amount", rewards.eclip.to_string());
    }

    Ok(response.add_messages(msgs))
}

pub fn _claim_all(
    mut deps: DepsMut,
    env: Env,
    sender: String,
    with_flexible: bool,
    assets: Option<Vec<AssetInfo>>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let block_time = env.block.time.seconds();
    let reward_config = REWARD_CONFIG.load(deps.storage)?;

    let assets_list = assets
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|a| a.to_string());
    ensure!(
        has_unique_elements(assets_list),
        ContractError::DuplicatedAssets {}
    );

    let reward_weights = update_reward_weights(deps.branch(), env.clone())?;
    let total_user_reward = calculate_total_user_reward(deps.as_ref(), sender.clone(), block_time)?;
    let mut total_eclipastro_reward = Uint128::zero();
    let mut total_beclip_reward = Uint128::zero();
    let mut total_eclip_reward = Uint128::zero();

    for reward_duration in total_user_reward {
        if reward_duration.duration == 0 && !with_flexible {
            continue;
        }
        for reward_locked_at in reward_duration.rewards {
            let locked_at = reward_locked_at.locked_at;
            let mut user_staking = USER_STAKED
                .load(deps.storage, (&sender, reward_duration.duration, locked_at))
                .unwrap_or_default();
            if let Some(asset_list) = assets.clone() {
                for asset in asset_list {
                    if asset.equal(&AssetInfo::NativeToken {
                        denom: config.token.clone(),
                    }) {
                        user_staking
                            .reward_weights
                            .eclipastro
                            .clone_from(&reward_weights.eclipastro);
                        total_eclipastro_reward += reward_locked_at.rewards.eclipastro;
                    }
                    if asset.equal(&reward_config.details.eclip.info) {
                        user_staking.reward_weights.eclip = reward_weights.eclip;
                        total_eclip_reward += reward_locked_at.rewards.eclip;
                    }
                    if asset.equal(&reward_config.details.beclip.info) {
                        user_staking.reward_weights.beclip = reward_weights.beclip;
                        total_beclip_reward += reward_locked_at.rewards.beclip;
                    }
                }
            } else {
                total_eclipastro_reward += reward_locked_at.rewards.eclipastro;
                total_beclip_reward += reward_locked_at.rewards.beclip;
                total_eclip_reward += reward_locked_at.rewards.eclip;
                user_staking.reward_weights = reward_weights.clone();
            }
            USER_STAKED.save(
                deps.storage,
                (&sender, reward_duration.duration, locked_at),
                &user_staking,
            )?;
        }
    }

    let mut last_claim_time = block_time;
    if let Some(reward_end_time) = reward_config.reward_end_time {
        if reward_end_time < block_time {
            last_claim_time = reward_end_time;
        }
    }
    LAST_CLAIM_TIME.save(deps.storage, &last_claim_time)?;

    _claim(
        deps,
        env,
        sender,
        UserReward {
            eclipastro: total_eclipastro_reward,
            eclip: total_eclip_reward,
            beclip: total_beclip_reward,
        },
    )
}

/// Claim user rewards
pub fn claim(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    duration: u64,
    locked_at: Option<u64>,
    assets: Option<Vec<AssetInfo>>,
) -> Result<Response, ContractError> {
    let locked_at = locked_at.unwrap_or_default();
    let reward_weights = update_reward_weights(deps.branch(), env.clone())?;
    let (_, response) = _claim_single(
        deps,
        env,
        info.sender.to_string(),
        duration,
        locked_at,
        reward_weights,
        assets,
    )?;
    Ok(response)
}

pub fn claim_all(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    with_flexible: bool,
    assets: Option<Vec<AssetInfo>>,
) -> Result<Response, ContractError> {
    _claim_all(deps, env, info.sender.to_string(), with_flexible, assets)
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
    let block_time = env.block.time.seconds();
    let is_allowed_user = ALLOWED_USERS
        .load(deps.storage, &sender)
        .unwrap_or_default();
    let config = CONFIG.load(deps.storage)?;
    let mut total_staking = TOTAL_STAKING.load(deps.storage)?;
    let reward_weights = update_reward_weights(deps.branch(), env.clone())?;
    let (mut user_staking, mut response) = _claim_single(
        deps.branch(),
        env.clone(),
        sender.clone(),
        duration,
        locked_at,
        reward_weights,
        None,
    )?;

    if amount.is_some() && duration > 0 {
        ensure!(is_allowed_user, ContractError::NotAllowed(sender));
    }
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

    total_staking = total_staking.checked_sub(unlock_amount).unwrap();

    if unlock_amount == user_staking.staked {
        USER_STAKED.remove(deps.storage, (&sender, duration, locked_at));
    } else {
        user_staking.staked -= unlock_amount;
        USER_STAKED.save(deps.storage, (&sender, duration, locked_at), &user_staking)?;
    }
    TOTAL_STAKING.save(deps.storage, &total_staking)?;
    TotalStakingByDuration::sub(deps.storage, unlock_amount, duration, locked_at, block_time)?;

    let penalty_amount = calculate_penalty(deps.as_ref(), env, unlock_amount, duration, locked_at)?;

    let mut msgs = vec![BankMsg::Send {
        to_address: receiver,
        amount: coins(
            unlock_amount.checked_sub(penalty_amount).unwrap().u128(),
            config.token.clone(),
        ),
    }];
    if !penalty_amount.is_zero() {
        msgs.push(BankMsg::Send {
            to_address: config.treasury.to_string(),
            amount: coins(penalty_amount.u128(), config.token),
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
// update each total_staking_by_duration by now
pub fn update_duration_total_staking(
    storage: &mut dyn Storage,
    duration: u64,
    block_time: u64,
) -> Result<(), ContractError> {
    let last_claim_time = LAST_CLAIM_TIME.load(storage).unwrap_or(block_time);
    let mut last_data =
        TotalStakingByDuration::load_at_ts(storage, duration, block_time, Some(last_claim_time))
            .unwrap_or_default();
    let mut next_check_time = last_claim_time / ONE_DAY * ONE_DAY + ONE_DAY;
    loop {
        if next_check_time > block_time {
            break;
        }
        let ended_lock = STAKING_DURATION_BY_END_TIME
            .load(storage, (duration, next_check_time))
            .unwrap_or_default();
        last_data = TotalStakingByDuration {
            staked: last_data.staked,
            valid_staked: last_data.valid_staked.checked_sub(ended_lock).unwrap(),
        };
        TOTAL_STAKING_BY_DURATION.save(storage, duration, &last_data, next_check_time)?;
        next_check_time += ONE_DAY;
    }
    Ok(())
}
/// update all total_staking_by_duration and reward_weights by now
pub fn update_reward_weights(deps: DepsMut, env: Env) -> Result<RewardWeights, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut block_time = env.block.time.seconds();
    // update all total_staking_by_duration by current time
    for tl_cfg in config.timelock_config.clone().into_iter() {
        update_duration_total_staking(deps.storage, tl_cfg.duration, block_time)?;
    }
    let reward_cfg = REWARD_CONFIG.load(deps.storage)?;
    if let Some(reward_end_time) = reward_cfg.reward_end_time {
        if reward_end_time < block_time {
            block_time = reward_end_time;
        }
    }
    // if it's first time to stake, last_claim_time is zero
    let last_claim_time = LAST_CLAIM_TIME.load(deps.storage).unwrap_or(block_time);
    let total_staking = TOTAL_STAKING.load(deps.storage)?;
    // if last_claim_time is zero, no total_staking, no reward
    let mut reward_weights =
        RewardWeights::load_at_ts(deps.storage, block_time, Some(last_claim_time))
            .unwrap_or_default();
    let mut start_time = last_claim_time;
    let mut end_time = last_claim_time / ONE_DAY * ONE_DAY + ONE_DAY;
    // loop from last_claim_time to min(now, reward_end_time), update reward_weights
    loop {
        if end_time > block_time {
            end_time = block_time;
        }
        let boost_sum = TotalStakingByDuration::load_boost_sum_at_ts(
            deps.storage,
            block_time,
            Some(start_time),
        )?;
        reward_weights.eclip +=
            Decimal256::from_ratio(reward_cfg.details.eclip.daily_reward, boost_sum)
                .checked_mul(Decimal256::from_ratio(end_time - start_time, ONE_DAY))
                .unwrap();
        reward_weights.beclip +=
            Decimal256::from_ratio(reward_cfg.details.beclip.daily_reward, boost_sum)
                .checked_mul(Decimal256::from_ratio(end_time - start_time, ONE_DAY))
                .unwrap();
        let pending_eclipastro_reward =
            calculate_eclipastro_reward(deps.as_ref(), env.clone(), start_time, end_time)?;
        reward_weights.eclipastro +=
            Decimal256::from_ratio(pending_eclipastro_reward, total_staking);
        REWARD_WEIGHTS.save(deps.storage, &reward_weights, end_time)?;
        if end_time == block_time {
            break;
        }
        start_time = end_time;
        end_time += ONE_DAY;
    }
    Ok(reward_weights)
}
