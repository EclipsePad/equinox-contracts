use astroport::{
    asset::{Asset, AssetInfo, AssetInfoExt},
    incentives::ExecuteMsg as IncentivesExecuteMsg,
    staking::ExecuteMsg as StakingExecuteMsg,
};
use cosmwasm_std::{
    attr, coin, ensure, ensure_eq, to_json_binary, BankMsg, CosmosMsg, Decimal256, DepsMut, Env,
    MessageInfo, Response, Uint128, WasmMsg,
};
use cw_utils::one_coin;
use equinox_msg::{
    lp_staking::{
        CallbackMsg, OwnershipProposal, RewardDetails, RewardDistribution, RewardWeight,
        UpdateConfigMsg, UserStaking,
    },
    utils::has_unique_elements,
};

use crate::{
    config::{BPS_DENOMINATOR, MAX_PROPOSAL_TTL},
    entry::query::{
        calculate_incentive_pending_rewards, calculate_pending_eclipse_rewards,
        calculate_updated_reward_weights, calculate_user_staking_rewards, calculate_vault_rewards,
    },
    error::ContractError,
    state::{
        CONFIG, LAST_CLAIMED, OWNER, OWNERSHIP_PROPOSAL, REWARD_CONFIG, REWARD_WEIGHTS, STAKING,
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
    if let Some(lp_token) = new_config.lp_token {
        lp_token.check(deps.api)?;
        config.lp_token = lp_token.clone();
        res = res.add_attribute("lp_token", lp_token.to_string());
    }
    if let Some(lp_contract) = new_config.lp_contract {
        config.lp_contract = deps.api.addr_validate(lp_contract.as_str())?;
        res = res.add_attribute("lp_contract", lp_contract.to_string());
    }
    if let Some(astroport_incentives) = new_config.astroport_incentives {
        config.astroport_incentives = deps.api.addr_validate(astroport_incentives.as_str())?;
        res = res.add_attribute("astroport_incentives", astroport_incentives.to_string());
    }
    if let Some(treasury) = new_config.treasury {
        config.treasury = treasury.clone();
        res = res.add_attribute("treasury", treasury);
    }
    if let Some(stability_pool) = new_config.stability_pool {
        config.stability_pool = deps.api.addr_validate(stability_pool.as_str())?;
        res = res.add_attribute("stability_pool", stability_pool.to_string());
    }
    if let Some(ce_reward_distributor) = new_config.ce_reward_distributor {
        config.ce_reward_distributor = deps.api.addr_validate(ce_reward_distributor.as_str())?;
        res = res.add_attribute("ce_reward_distributor", ce_reward_distributor.to_string());
    }
    CONFIG.save(deps.storage, &config)?;
    Ok(res)
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

/// Update reward config
pub fn update_reward_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    distribution: Option<RewardDistribution>,
    reward_end_time: Option<u64>,
    details: Option<RewardDetails>,
) -> Result<Response, ContractError> {
    // only owner can executable
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let mut reward_config = REWARD_CONFIG.load(deps.storage)?;
    // the sum bps should be 10000
    if let Some(distribution) = distribution {
        ensure_eq!(
            distribution.users
                + distribution.treasury
                + distribution.ce_holders
                + distribution.stability_pool,
            BPS_DENOMINATOR,
            ContractError::RewardDistributionErr {}
        );
        reward_config.distribution = distribution;
    }
    if let Some(reward_end_time) = reward_end_time {
        let current_time = env.block.time.seconds();
        ensure!(
            reward_config.reward_end_time > current_time && reward_end_time > current_time,
            ContractError::InvalidEndTime {}
        );
        reward_config.reward_end_time = reward_end_time;
    }
    if let Some(details) = details {
        reward_config.details = details;
    }
    REWARD_CONFIG.save(deps.storage, &reward_config)?;
    Ok(Response::new().add_attribute("action", "update reward config"))
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
        CallbackMsg::DistributeEclipseRewards { assets } => {
            distribute_eclipse_rewards(deps, env, info, assets)
        }
    }
}

// stake lp token when it is native token
pub fn stake(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let asset = one_coin(&info)?;
    let cfg = CONFIG.load(deps.storage)?;
    let sender = info.sender.clone();
    let recipient = recipient.unwrap_or(info.sender.to_string());
    let mut total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    let (mut user_staking, _, response) = _claim(deps.branch(), env, recipient.clone(), None)?;

    ensure!(
        cfg.lp_token.is_native_token() && cfg.lp_token.to_string() == asset.denom,
        ContractError::AssetsNotMatch {
            got: asset.denom,
            expected: cfg.lp_token.to_string()
        }
    );
    ensure!(
        asset.amount.gt(&Uint128::zero()),
        ContractError::ZeroAmount {}
    );

    // stake LP token to Astroport generator contract
    let msgs = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.astroport_incentives.to_string(),
        msg: to_json_binary(&IncentivesExecuteMsg::Deposit { recipient: None })?,
        funds: vec![asset.clone()],
    })];

    total_staking += asset.amount;
    user_staking.staked += asset.amount;

    TOTAL_STAKING.save(deps.storage, &total_staking)?;
    STAKING.save(deps.storage, &recipient, &user_staking)?;
    Ok(response
        .add_messages(msgs)
        .add_attribute("action", "stake")
        .add_attribute("sender", sender)
        .add_attribute("amount", asset.amount.to_string())
        .add_attribute("recipient", recipient))
}

pub fn _claim(
    deps: DepsMut,
    env: Env,
    sender: String,
    assets: Option<Vec<AssetInfo>>,
) -> Result<(UserStaking, Vec<RewardWeight>, Response), ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
    let mut user_staking = STAKING.load(deps.storage, &sender).unwrap_or_default();
    let reward_config = REWARD_CONFIG.load(deps.storage)?;
    let reward_end_time = reward_config.reward_end_time;
    let mut msgs = vec![];

    let assets_list = assets
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|a| a.to_string());
    ensure!(
        has_unique_elements(assets_list),
        ContractError::DuplicatedAssets {}
    );

    // claim astro reward
    if !total_staking.is_zero() {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.astroport_incentives.to_string(),
            msg: to_json_binary(&IncentivesExecuteMsg::ClaimRewards {
                lp_tokens: vec![cfg.lp_token.to_string()],
            })?,
            funds: vec![],
        }));
    }

    let mut response = Response::new()
        .add_attribute("action", "claim rewards")
        .add_attribute("recipient", sender.clone());

    let astroport_rewards =
        calculate_incentive_pending_rewards(deps.as_ref(), env.contract.address.clone())?;
    let vault_rewards = calculate_vault_rewards(deps.as_ref(), env.block.time.seconds())?;
    let pending_eclipse_rewards =
        calculate_pending_eclipse_rewards(deps.as_ref(), astroport_rewards.clone())?;
    let updated_reward_weights =
        calculate_updated_reward_weights(deps.as_ref(), astroport_rewards, vault_rewards)?;
    if !user_staking.staked.is_zero() {
        let user_rewards = calculate_user_staking_rewards(
            deps.as_ref(),
            sender.clone(),
            updated_reward_weights.clone(),
        )?;
        let mut coins = vec![];
        let mut updated_user_reward_weights = vec![];
        for r in user_rewards {
            let claimable = assets.clone().is_none()
                || (assets.clone().is_some()
                    && assets.clone().unwrap().iter().any(|a| a.equal(&r.info)));
            if !r.amount.is_zero() && claimable {
                if r.info.is_native_token() {
                    coins.push(coin(r.amount.u128(), r.info.to_string()));
                    response = response
                        .add_attribute("action", "claim")
                        .add_attribute("denom", r.info.to_string())
                        .add_attribute("amount", r.amount);
                } else {
                    msgs.push(r.info.with_balance(r.amount).into_msg(sender.clone())?);
                    response = response
                        .add_attribute("action", "claim")
                        .add_attribute("address", r.info.to_string())
                        .add_attribute("amount", r.amount);
                }
                updated_user_reward_weights.push(
                    updated_reward_weights
                        .clone()
                        .into_iter()
                        .find(|w| w.info.equal(&r.info))
                        .unwrap(),
                );
            } else {
                updated_user_reward_weights.push(
                    user_staking
                        .reward_weights
                        .clone()
                        .into_iter()
                        .find(|w| w.info.equal(&r.info))
                        .unwrap_or(RewardWeight {
                            info: r.info,
                            reward_weight: Decimal256::zero(),
                        }),
                );
            }
        }
        user_staking.reward_weights = updated_user_reward_weights;
        if !coins.is_empty() {
            msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: sender.clone(),
                amount: coins,
            }));
        }
    } else {
        user_staking
            .reward_weights
            .clone_from(&updated_reward_weights);
    }

    if !pending_eclipse_rewards.is_empty() {
        msgs.push(
            CallbackMsg::DistributeEclipseRewards {
                assets: pending_eclipse_rewards,
            }
            .to_cosmos_msg(&env)?,
        );
    }

    REWARD_WEIGHTS.save(deps.storage, &updated_reward_weights)?;
    STAKING.save(deps.storage, &sender, &user_staking)?;
    if reward_end_time > env.block.time.seconds() {
        LAST_CLAIMED.save(deps.storage, &env.block.time.seconds())?;
    } else {
        LAST_CLAIMED.save(deps.storage, &reward_end_time)?;
    }

    Ok((
        user_staking,
        updated_reward_weights,
        response.add_messages(msgs),
    ))
}

/// Claim user rewards
pub fn claim(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    sender: String,
    assets: Option<Vec<AssetInfo>>,
) -> Result<Response, ContractError> {
    let (_, _, response) = _claim(deps, env, sender, assets)?;
    Ok(response)
}

/// Unstake amount and claim rewards of user
/// check unstake amount
pub fn unstake(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut total_staking = TOTAL_STAKING.load(deps.storage)?;

    let receiver = recipient.unwrap_or(info.sender.to_string());
    let mut msgs = vec![];

    let (mut user_staking, _, response) = _claim(deps.branch(), env, receiver.clone(), None)?;

    ensure!(
        amount.le(&user_staking.staked),
        ContractError::ExeedingUnstakeAmount {
            got: amount.u128(),
            expected: user_staking.staked.u128()
        }
    );

    total_staking = total_staking.checked_sub(amount).unwrap();
    user_staking.staked = user_staking.staked.checked_sub(amount).unwrap();

    TOTAL_STAKING.save(deps.storage, &total_staking)?;
    STAKING.save(deps.storage, &info.sender.to_string(), &user_staking)?;

    // send lp_token to user
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.astroport_incentives.to_string(),
        msg: to_json_binary(&IncentivesExecuteMsg::Withdraw {
            lp_token: cfg.lp_token.to_string(),
            amount,
        })?,
        funds: vec![],
    }));
    msgs.push(cfg.lp_token.with_balance(amount).into_msg(receiver)?);
    Ok(response
        .add_attribute("action", "unstake")
        .add_attribute("from", info.sender.to_string())
        .add_attribute("amount", amount.to_string())
        .add_messages(msgs))
}

pub fn distribute_eclipse_rewards(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    assets: Vec<Asset>,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let reward_cfg = REWARD_CONFIG.load(deps.storage)?;
    let mut msgs = vec![];
    for asset in assets {
        if asset.info.to_string() == cfg.astro.clone() {
            let ce_holders_rewards = asset.amount.multiply_ratio(
                reward_cfg.distribution.ce_holders,
                BPS_DENOMINATOR - reward_cfg.distribution.users,
            );
            let stability_pool_rewards = asset.amount.multiply_ratio(
                reward_cfg.distribution.stability_pool,
                BPS_DENOMINATOR - reward_cfg.distribution.users,
            );
            let treasury_rewards = asset
                .amount
                .checked_sub(ce_holders_rewards)
                .unwrap_or_default()
                .checked_sub(stability_pool_rewards)
                .unwrap_or_default();
            if ce_holders_rewards.gt(&Uint128::zero()) {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.astro_staking.to_string(),
                    msg: to_json_binary(&StakingExecuteMsg::Enter {
                        receiver: Some(cfg.ce_reward_distributor.to_string()),
                    })?,
                    funds: vec![coin(ce_holders_rewards.u128(), cfg.astro.clone())],
                }));
            }
            if stability_pool_rewards.gt(&Uint128::zero()) {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.astro_staking.to_string(),
                    msg: to_json_binary(&StakingExecuteMsg::Enter {
                        receiver: Some(cfg.stability_pool.to_string()),
                    })?,
                    funds: vec![coin(stability_pool_rewards.u128(), cfg.astro.clone())],
                }));
            }
            if treasury_rewards.gt(&Uint128::zero()) {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.astro_staking.to_string(),
                    msg: to_json_binary(&StakingExecuteMsg::Enter {
                        receiver: Some(cfg.treasury.clone().to_string()),
                    })?,
                    funds: vec![coin(treasury_rewards.u128(), cfg.astro.clone())],
                }));
            }
        }
    }
    Ok(Response::new().add_messages(msgs))
}
