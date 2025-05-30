use std::str::FromStr;

use astroport::{
    asset::{Asset, AssetInfo, AssetInfoExt},
    incentives::ExecuteMsg as IncentivesExecuteMsg,
    staking::ExecuteMsg as StakingExecuteMsg,
};
use cosmwasm_std::{
    attr, coin, coins, ensure, ensure_eq, to_json_binary, wasm_execute, BankMsg, CosmosMsg,
    Decimal256, DepsMut, Env, MessageInfo, Order, ReplyOn, Response, StdResult, SubMsg,
    SubMsgResult, Uint128, WasmMsg,
};
use cw_storage_plus::Bound;
use cw_utils::{one_coin, ParseReplyError};
use eclipse_base::{
    converters::{str_to_dec, u128_to_dec},
    staking::{
        msg::{ExecuteMsg as EclipStakingExecuteMsg, QueryMsg as EclipStakingQueryMsg},
        types::Config as EclipStakingConfig,
    },
    voter::{
        msg::{ExecuteMsg as VoterExecuteMsg, QueryMsg as VoterQueryMsg},
        types::TokenConfig as VoterTokenConfig,
    },
};
use equinox_msg::{
    lp_staking::{
        CallbackMsg, Config, OwnershipProposal, Reward, RewardDistribution, RewardWeight,
        UpdateConfigMsg, UserStaking,
    },
    single_sided_staking::UnbondedItem,
    utils::{check_unbonding_period, has_unique_elements, UNBONDING_FEE_RATE, UNBONDING_PERIOD_0},
};

use crate::{
    config::{BPS_DENOMINATOR, DEFAULT_REWARD_DISTRIBUTION_PERIOD, MAX_PROPOSAL_TTL},
    entry::query::{
        calculate_incentive_pending_rewards, calculate_pending_eclipse_rewards,
        calculate_updated_reward_weights, calculate_user_staking_rewards, calculate_vault_rewards,
    },
    error::ContractError,
    state::{
        ALLOWED_USERS, BLACK_LIST, BLACK_LIST_REWARDS, CONFIG, ECLIP_ASTRO_TO_ASTRO_REPLY_ID,
        LAST_CLAIMED, OWNER, OWNERSHIP_PROPOSAL, REWARD, REWARD_DISTRIBUTION, REWARD_WEIGHTS,
        STAKING, TOTAL_STAKING, USER_UNBONDED, WITHDRAW_LIQUIDITY_REPLY_ID, WITHDRAW_TEMP_DATA,
        XASTRO_TO_ASTRO_REPLY_ID,
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
    if let Some(lockdrop) = new_config.lockdrop {
        config.lockdrop = deps.api.addr_validate(&lockdrop)?;
        res = res.add_attribute("lockdrop", lockdrop);
    }
    if let Some(treasury) = new_config.treasury {
        config.treasury = deps.api.addr_validate(&treasury)?;
        res = res.add_attribute("treasury", treasury);
    }
    if let Some(funding_dao) = new_config.funding_dao {
        config.funding_dao = deps.api.addr_validate(funding_dao.as_str())?;
        res = res.add_attribute("funding_dao", funding_dao.to_string());
    }
    if let Some(eclip) = new_config.eclip {
        config.eclip = eclip.clone();
        res = res.add_attribute("eclip", eclip);
    }
    if let Some(beclip) = new_config.beclip {
        config.beclip = deps.api.addr_validate(&beclip)?;
        res = res.add_attribute("beclip", beclip);
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

/// Update reward distribution
pub fn update_reward_distribution(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    distribution: RewardDistribution,
) -> Result<Response, ContractError> {
    // only owner can executable
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    // the sum bps should be 10000
    ensure_eq!(
        distribution.users + distribution.treasury + distribution.funding_dao,
        BPS_DENOMINATOR,
        ContractError::RewardDistributionErr {}
    );
    REWARD_DISTRIBUTION.save(deps.storage, &distribution)?;
    Ok(Response::new().add_attribute("action", "update reward distribution"))
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
    let blacklist = BLACK_LIST.load(deps.storage).unwrap_or_default();
    let mut blacklist_rewards = BLACK_LIST_REWARDS.load(deps.storage).unwrap_or_default();
    let is_allowed_user = ALLOWED_USERS
        .load(deps.storage, &sender)
        .unwrap_or_default();
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

        let mut updated_user_reward_weights = vec![];
        for r in user_rewards {
            let claimable = !blacklist.contains(&sender)
                && (assets.clone().is_none()
                    || (assets.clone().is_some()
                        && assets.clone().unwrap().iter().any(|a| a.equal(&r.info))));
            if !r.amount.is_zero() && claimable {
                if r.info.is_native_token() {
                    msgs.push(r.info.with_balance(r.amount).into_msg(sender.clone())?);
                    response = response
                        .add_attribute("action", "claim")
                        .add_attribute("denom", r.info.to_string())
                        .add_attribute("amount", r.amount);
                } else {
                    if r.info.to_string() == cfg.beclip {
                        if is_allowed_user {
                            msgs.push(
                                AssetInfo::NativeToken {
                                    denom: cfg.eclip.clone(),
                                }
                                .with_balance(r.amount)
                                .into_msg(sender.clone())?,
                            );
                        } else {
                            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                                contract_addr: cfg.eclip_staking.to_string(),
                                msg: to_json_binary(&EclipStakingExecuteMsg::BondFor {
                                    address_and_amount_list: vec![(sender.clone(), r.amount)],
                                })?,
                                funds: coins(r.amount.u128(), cfg.eclip.clone()),
                            }));
                        }
                    } else {
                        msgs.push(r.info.with_balance(r.amount).into_msg(sender.clone())?);
                    }
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
                if blacklist.contains(&sender) {
                    let position = blacklist_rewards.iter().position(|x| x.info.equal(&r.info));
                    match position {
                        Some(p) => {
                            blacklist_rewards[p].amount += r.amount;
                        }
                        None => {
                            blacklist_rewards.push(r.clone());
                        }
                    }
                }
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
    LAST_CLAIMED.save(deps.storage, &env.block.time.seconds())?;
    BLACK_LIST_REWARDS.save(deps.storage, &blacklist_rewards)?;

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
    let blacklist = BLACK_LIST.load(deps.storage).unwrap_or_default();
    ensure!(!blacklist.contains(&sender), ContractError::Blacklisted {});
    let (_, _, response) = _claim(deps, env, sender, assets)?;
    Ok(response)
}

pub fn claim_blacklist_rewards(mut deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let blacklist = BLACK_LIST.load(deps.storage).unwrap_or_default();
    for user in blacklist {
        let _ = _claim(deps.branch(), env.clone(), user, None)?;
    }
    let blacklist_rewards = BLACK_LIST_REWARDS.load(deps.storage)?;
    let mut msgs = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.astroport_incentives.to_string(),
        msg: to_json_binary(&IncentivesExecuteMsg::ClaimRewards {
            lp_tokens: vec![cfg.lp_token.to_string()],
        })?,
        funds: vec![],
    })];
    let mut response = Response::new();
    for r in blacklist_rewards {
        if !r.amount.is_zero() {
            if r.info.is_native_token() {
                msgs.push(
                    r.info
                        .with_balance(r.amount)
                        .into_msg(cfg.treasury.to_string())?,
                );
                response = response
                    .add_attribute("action", "claim")
                    .add_attribute("denom", r.info.to_string())
                    .add_attribute("amount", r.amount);
            } else {
                if r.info.to_string() == cfg.beclip {
                    msgs.push(
                        AssetInfo::NativeToken {
                            denom: cfg.eclip.clone(),
                        }
                        .with_balance(r.amount)
                        .into_msg(cfg.treasury.to_string())?,
                    );
                } else {
                    msgs.push(
                        r.info
                            .with_balance(r.amount)
                            .into_msg(cfg.treasury.to_string())?,
                    );
                }
                response = response
                    .add_attribute("action", "claim")
                    .add_attribute("address", r.info.to_string())
                    .add_attribute("amount", r.amount);
            }
        }
    }
    Ok(response.add_messages(msgs))
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

/// it's only way to remove user from bl after instantiation
pub fn remove_from_blacklist(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    user: String,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    deps.api.addr_validate(&user)?;

    BLACK_LIST.update(deps.storage, |mut x| -> StdResult<_> {
        x.retain(|y| y != &user);

        Ok(x)
    })?;

    Ok(Response::new().add_attribute("action", "remove_from_blacklist"))
}

pub fn unbond(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
    period: u64,
) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let block_time = env.block.time.seconds();
    let config = CONFIG.load(deps.storage)?;
    check_unbonding_period(period, ContractError::IncorrectUnbondingPeriod)?;

    let mut total_staking = TOTAL_STAKING.load(deps.storage)?;
    let (mut user_staking, _, response) = _claim(deps.branch(), env, sender.to_string(), None)?;
    let amount_to_unstake = amount.unwrap_or(user_staking.staked);

    if amount_to_unstake.is_zero() {
        Err(ContractError::ZeroAmount {})?;
    }

    if amount_to_unstake > user_staking.staked {
        Err(ContractError::ExeedingUnstakeAmount {
            got: amount_to_unstake.u128(),
            expected: user_staking.staked.u128(),
        })?;
    }

    total_staking = total_staking.checked_sub(amount_to_unstake).unwrap();
    user_staking.staked = user_staking.staked.checked_sub(amount_to_unstake).unwrap();

    USER_UNBONDED.update(deps.storage, sender, |x| -> StdResult<_> {
        let mut unbonded = x.unwrap_or_default();
        let fee = if period == UNBONDING_PERIOD_0 {
            (str_to_dec(UNBONDING_FEE_RATE) * u128_to_dec(amount_to_unstake)).to_uint_floor()
        } else {
            Uint128::zero()
        };

        unbonded.push(UnbondedItem {
            amount: amount_to_unstake - fee,
            fee,
            release_date: block_time + period,
        });

        Ok(unbonded)
    })?;

    TOTAL_STAKING.save(deps.storage, &total_staking)?;
    STAKING.save(deps.storage, &info.sender.to_string(), &user_staking)?;

    // send lp_token to the contract
    let msg = CosmosMsg::Wasm(wasm_execute(
        config.astroport_incentives,
        &IncentivesExecuteMsg::Withdraw {
            lp_token: config.lp_token.to_string(),
            amount: amount_to_unstake,
        },
        vec![],
    )?);

    Ok(response.add_attribute("action", "unbond").add_message(msg))
}

pub fn withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let recipient = recipient
        .map(|x| deps.api.addr_validate(&x))
        .transpose()?
        .unwrap_or(sender.to_owned());
    let block_time = env.block.time.seconds();
    let config = CONFIG.load(deps.storage)?;
    let unbonded = USER_UNBONDED.load(deps.storage, sender).unwrap_or_default();
    let (amount_to_send, fee_to_send) = unbonded.iter().fold(
        (Uint128::zero(), Uint128::zero()),
        |(mut amount_acc, mut fee_acc), cur| {
            if block_time >= cur.release_date {
                amount_acc += cur.amount;
                fee_acc += cur.fee;
            }

            (amount_acc, fee_acc)
        },
    );
    let items_to_preserve: Vec<_> = unbonded
        .into_iter()
        .filter(|x| block_time < x.release_date)
        .collect();

    if amount_to_send.is_zero() {
        Err(ContractError::EarlyWithdraw)?;
    }

    if items_to_preserve.is_empty() {
        USER_UNBONDED.remove(deps.storage, sender);
    } else {
        USER_UNBONDED.save(deps.storage, sender, &items_to_preserve)?;
    }

    let fee_rate = u128_to_dec(fee_to_send) / u128_to_dec(amount_to_send + fee_to_send);
    WITHDRAW_TEMP_DATA.save(deps.storage, &(recipient, Uint128::zero(), fee_rate))?;

    // withdraw liquidity from pair contract
    let submsg = SubMsg {
        id: WITHDRAW_LIQUIDITY_REPLY_ID,
        msg: CosmosMsg::Wasm(wasm_execute(
            config.lp_contract,
            &astroport::pair::ExecuteMsg::WithdrawLiquidity {
                assets: vec![],
                min_assets_to_receive: None,
            },
            coins(
                (amount_to_send + fee_to_send).u128(),
                config.lp_token.to_string(),
            ),
        )?),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };

    Ok(Response::new().add_submessage(submsg))
}

pub fn handle_withdraw_liquidity_reply(
    deps: DepsMut,
    _env: Env,
    result: &SubMsgResult,
) -> Result<Response, ContractError> {
    let mut response = Response::new();
    let res = result
        .to_owned()
        .into_result()
        .map_err(|e| ContractError::ParseReplyError(ParseReplyError::SubMsgFailure(e)))?;

    // parse received lp token amount
    let mut refund_assets = String::default();
    for event in res.events.iter() {
        for attr in event.attributes.iter() {
            if attr.key == "refund_assets" {
                refund_assets = attr.value.to_string();
            }
        }
    }

    let Config {
        xastro,
        eclip_staking,
        ..
    } = &CONFIG.load(deps.storage)?;
    let EclipStakingConfig { equinox_voter, .. } = deps
        .querier
        .query_wasm_smart(eclip_staking, &EclipStakingQueryMsg::QueryConfig {})?;
    let equinox_voter = equinox_voter.ok_or(ContractError::NoVoter)?;
    let VoterTokenConfig { eclip_astro, .. } = &deps
        .querier
        .query_wasm_smart(&equinox_voter, &VoterQueryMsg::TokenConfig {})?;

    let eclip_astro_amount = get_token_amount(&refund_assets, eclip_astro);
    let xastro_amount = get_token_amount(&refund_assets, xastro);

    // swap eclipAstro to astro
    response = response.add_submessage(SubMsg {
        id: ECLIP_ASTRO_TO_ASTRO_REPLY_ID,
        msg: wasm_execute(
            &equinox_voter,
            &VoterExecuteMsg::SwapToAstro { recipient: None },
            coins(eclip_astro_amount.u128(), eclip_astro),
        )?
        .into(),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    });

    // swap xAstro to astro
    response = response.add_submessage(SubMsg {
        id: XASTRO_TO_ASTRO_REPLY_ID,
        msg: wasm_execute(
            &equinox_voter,
            &VoterExecuteMsg::SwapToAstro { recipient: None },
            coins(xastro_amount.u128(), xastro),
        )?
        .into(),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    });

    Ok(response)
}

pub fn handle_swap_to_astro_reply(
    deps: DepsMut,
    _env: Env,
    result: &SubMsgResult,
) -> Result<Response, ContractError> {
    let mut response = Response::new();
    let res = result
        .to_owned()
        .into_result()
        .map_err(|_| ContractError::SwapToAstroError)?;

    let mut astro_amount = Uint128::zero();
    for event in res.events.iter() {
        for attr in event.attributes.iter() {
            if attr.key == "exchanged_astro" {
                astro_amount = Uint128::from_str(&attr.value).unwrap();
            }
        }
    }

    let (recipient, mut amount, fee_rate) = WITHDRAW_TEMP_DATA.load(deps.storage)?;

    // 1st time only update amount info
    if amount.is_zero() {
        WITHDRAW_TEMP_DATA.save(deps.storage, &(recipient, astro_amount, fee_rate))?;

        return Ok(response);
    }

    // 2nd time send tokens
    amount += astro_amount;
    let fee_to_send = (fee_rate * u128_to_dec(amount)).to_uint_floor();
    let amount_to_send = amount - fee_to_send;

    let Config {
        astro, treasury, ..
    } = CONFIG.load(deps.storage)?;

    // return astro to user
    response = response.add_message(CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient.to_string(),
        amount: coins(amount_to_send.u128(), &astro),
    }));

    // send fee to treasury
    if !fee_to_send.is_zero() {
        response = response.add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: treasury.to_string(),
            amount: coins(fee_to_send.u128(), &astro),
        }));
    }

    Ok(response
        .add_attribute("action", "withdraw")
        .add_attribute("amount_to_send", amount_to_send)
        .add_attribute("fee_to_send", fee_to_send))
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

    // if info.sender != cfg.lockdrop {
    //     // use unbond + withdraw instead
    //     Err(ContractError::MessageIsDisabled)?;
    // }

    let receiver = recipient.unwrap_or(info.sender.to_string());
    let mut msgs = vec![];

    let (mut user_staking, _, response) =
        _claim(deps.branch(), env, info.sender.to_string(), None)?;

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
// add reweards
pub fn add_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    from: Option<u64>,
    duration: Option<u64>,
    eclip: Uint128,
    beclip: Uint128,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let cfg = CONFIG.load(deps.storage)?;
    let asset = one_coin(&info)?;
    ensure!(cfg.eclip == asset.denom, ContractError::InvalidAsset {});
    ensure!(
        eclip + beclip == asset.amount,
        ContractError::AmountNotMatch {
            got: asset.amount.u128(),
            expected: (eclip + beclip).u128()
        }
    );
    let block_time = env.block.time.seconds();
    // get reward start time
    let reward_start_time = if let Some(from) = from {
        ensure!(
            from >= block_time,
            ContractError::InvalidStartTime {
                got: from,
                expect: block_time
            }
        );
        from
    } else {
        // fetch reward data which end time is bigger than current time
        let last_rewards = REWARD
            .range(
                deps.storage,
                Some(Bound::exclusive((block_time, 0u64))),
                None,
                Order::Descending,
            )
            .take(1)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        if last_rewards.is_empty() {
            block_time
        } else {
            last_rewards[0].0 .0
        }
    };
    // get reward duration
    let duration = duration.unwrap_or(DEFAULT_REWARD_DISTRIBUTION_PERIOD);
    REWARD.save(
        deps.storage,
        (reward_start_time + duration, reward_start_time),
        &Reward { eclip, beclip },
    )?;
    Ok(Response::new()
        .add_attribute("action", "add_rewards")
        .add_attribute("from", reward_start_time.to_string())
        .add_attribute("duration", duration.to_string())
        .add_attribute("eclip", eclip.to_string())
        .add_attribute("beclip", beclip.to_string()))
}

pub fn distribute_eclipse_rewards(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    assets: Vec<Asset>,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let reward_distribution = REWARD_DISTRIBUTION.load(deps.storage)?;
    let mut msgs = vec![];
    for asset in assets {
        if asset.info.to_string() == cfg.astro.clone() {
            let funding_dao_rewards = asset.amount.multiply_ratio(
                reward_distribution.funding_dao,
                BPS_DENOMINATOR - reward_distribution.users,
            );
            let treasury_rewards = asset
                .amount
                .checked_sub(funding_dao_rewards)
                .unwrap_or_default();
            if funding_dao_rewards.gt(&Uint128::zero()) {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.astro_staking.to_string(),
                    msg: to_json_binary(&StakingExecuteMsg::Enter {
                        receiver: Some(cfg.funding_dao.to_string()),
                    })?,
                    funds: vec![coin(funding_dao_rewards.u128(), cfg.astro.clone())],
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

fn get_token_amount(input: &str, target_token: &str) -> Uint128 {
    for token_part in input.split(',').map(|s| s.trim()) {
        if token_part.contains(target_token) {
            let numeric_prefix = token_part
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>();

            if !numeric_prefix.is_empty() {
                if let Ok(amount) = numeric_prefix.parse::<u128>() {
                    return Uint128::new(amount);
                }
            }
        }
    }

    Uint128::zero()
}
