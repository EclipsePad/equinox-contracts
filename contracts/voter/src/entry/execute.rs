use std::str::FromStr;

use astroport_governance::emissions_controller::hub::{UserInfoResponse, VotedPoolInfo};
use cosmwasm_std::{
    coin, coins, to_json_binary, Addr, BankMsg, CosmosMsg, Decimal, DepsMut, Empty, Env,
    MessageInfo, ReplyOn, Response, StdResult, SubMsg, SubMsgResult, Uint128, WasmMsg,
};

use eclipse_base::{
    assets::TokenUnverified,
    converters::{str_to_dec, u128_to_dec},
    utils::{check_funds, unwrap_field, FundsType},
};
use equinox_msg::voter::{
    msg::UserType,
    state::{
        ADDRESS_CONFIG, DAO_ESSENCE_ACC, DAO_WEIGHTS_ACC, DATE_CONFIG, DELEGATOR_ADDRESSES,
        ELECTOR_ADDITIONAL_ESSENCE_FRACTION, ELECTOR_ESSENCE_ACC, ELECTOR_WEIGHTS,
        ELECTOR_WEIGHTS_ACC, ELECTOR_WEIGHTS_REF, EPOCH_COUNTER, IS_LOCKED, MAX_EPOCH_AMOUNT,
        RECIPIENT, REWARDS_CLAIM_STAGE, ROUTE_CONFIG, SLACKER_ESSENCE_ACC, STAKE_ASTRO_REPLY_ID,
        SWAP_REWARDS_REPLY_ID_CNT, SWAP_REWARDS_REPLY_ID_MIN, TEMPORARY_REWARDS, TOKEN_CONFIG,
        TRANSFER_ADMIN_STATE, TRANSFER_ADMIN_TIMEOUT, USER_ESSENCE, USER_REWARDS, VOTE_RESULTS,
    },
    types::{
        AddressConfig, BribesAllocationItem, DateConfig, EssenceInfo, PoolInfoItem,
        RewardsClaimStage, RouteListItem, TokenConfig, TransferAdminState, VoteResults,
        WeightAllocationItem,
    },
};

use crate::{
    error::ContractError,
    helpers::{
        get_accumulated_rewards, get_route, get_total_votes, get_user_type, get_user_weights,
        try_unlock, try_unlock_and_check, verify_weight_allocation,
    },
    math::{
        calc_essence_allocation, calc_pool_info_list_with_rewards, calc_updated_essence_allocation,
        calc_weights_from_essence_allocation, split_dao_eclip_rewards, split_rewards,
    },
};

pub fn try_accept_admin_role(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let sender = info.sender;
    let block_time = env.block.time.seconds();
    let TransferAdminState {
        new_admin,
        deadline,
    } = TRANSFER_ADMIN_STATE.load(deps.storage)?;

    if sender != new_admin {
        Err(ContractError::Unauthorized)?;
    }

    if block_time >= deadline {
        Err(ContractError::TransferAdminDeadline)?;
    }

    ADDRESS_CONFIG.update(deps.storage, |mut x| -> StdResult<AddressConfig> {
        x.admin = sender;
        Ok(x)
    })?;

    TRANSFER_ADMIN_STATE.update(deps.storage, |mut x| -> StdResult<TransferAdminState> {
        x.deadline = block_time;
        Ok(x)
    })?;

    Ok(Response::new().add_attribute("action", "try_accept_admin_role"))
}

#[allow(clippy::too_many_arguments)]
pub fn try_update_address_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    admin: Option<String>,
    worker_list: Option<Vec<String>>,
    eclipse_dao: Option<String>,
    eclipsepad_foundry: Option<String>,
    eclipsepad_minter: Option<String>,
    eclipsepad_staking: Option<String>,
    eclipsepad_tribute_market: Option<String>,
    astroport_staking: Option<String>,
    astroport_assembly: Option<String>,
    astroport_voting_escrow: Option<String>,
    astroport_emission_controller: Option<String>,
    astroport_router: Option<String>,
    astroport_tribute_market: Option<String>,
) -> Result<Response, ContractError> {
    let mut config = ADDRESS_CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        Err(ContractError::Unauthorized)?;
    }

    if let Some(x) = admin {
        let block_time = env.block.time.seconds();
        let new_admin = deps.api.addr_validate(&x)?;

        TRANSFER_ADMIN_STATE.save(
            deps.storage,
            &TransferAdminState {
                new_admin,
                deadline: block_time + TRANSFER_ADMIN_TIMEOUT,
            },
        )?;
    }

    if let Some(x) = worker_list {
        config.worker_list = x
            .iter()
            .map(|x| deps.api.addr_validate(x))
            .collect::<StdResult<Vec<Addr>>>()?;
    }

    if let Some(x) = eclipse_dao {
        config.eclipse_dao = deps.api.addr_validate(&x)?;
    }

    if let Some(x) = eclipsepad_foundry {
        config.eclipsepad_foundry = Some(deps.api.addr_validate(&x)?);
    }

    if let Some(x) = eclipsepad_minter {
        config.eclipsepad_minter = deps.api.addr_validate(&x)?;
    }

    if let Some(x) = eclipsepad_staking {
        config.eclipsepad_staking = deps.api.addr_validate(&x)?;
    }

    if let Some(x) = eclipsepad_tribute_market {
        config.eclipsepad_tribute_market = Some(deps.api.addr_validate(&x)?);
    }

    if let Some(x) = astroport_staking {
        config.astroport_staking = deps.api.addr_validate(&x)?;
    }

    if let Some(x) = astroport_assembly {
        config.astroport_assembly = deps.api.addr_validate(&x)?;
    }

    if let Some(x) = astroport_voting_escrow {
        config.astroport_voting_escrow = deps.api.addr_validate(&x)?;
    }

    if let Some(x) = astroport_emission_controller {
        config.astroport_emission_controller = deps.api.addr_validate(&x)?;
    }

    if let Some(x) = astroport_router {
        config.astroport_router = deps.api.addr_validate(&x)?;
    }

    if let Some(x) = astroport_tribute_market {
        config.astroport_tribute_market = Some(deps.api.addr_validate(&x)?);
    }

    ADDRESS_CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "try_update_address_config"))
}

pub fn try_update_token_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    eclip: Option<String>,
    astro: Option<String>,
    xastro: Option<String>,
    eclip_astro: Option<String>,
) -> Result<Response, ContractError> {
    let AddressConfig { admin, .. } = ADDRESS_CONFIG.load(deps.storage)?;
    let mut config = TOKEN_CONFIG.load(deps.storage)?;

    if info.sender != admin {
        Err(ContractError::Unauthorized)?;
    }

    if let Some(x) = eclip {
        config.eclip = x;
    }

    if let Some(x) = astro {
        config.astro = x;
    }

    if let Some(x) = xastro {
        config.xastro = x;
    }

    if let Some(x) = eclip_astro {
        config.eclip_astro = x;
    }

    TOKEN_CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "try_update_token_config"))
}

pub fn try_update_date_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    genesis_epoch_start_date: Option<u64>,
    epoch_length: Option<u64>,
    vote_delay: Option<u64>,
) -> Result<Response, ContractError> {
    let AddressConfig { admin, .. } = ADDRESS_CONFIG.load(deps.storage)?;
    let mut config = DATE_CONFIG.load(deps.storage)?;

    if info.sender != admin {
        Err(ContractError::Unauthorized)?;
    }

    if let Some(x) = genesis_epoch_start_date {
        config.genesis_epoch_start_date = x;
    }

    if let Some(x) = epoch_length {
        config.epoch_length = x;
    }

    if let Some(x) = vote_delay {
        config.vote_delay = x;
    }

    DATE_CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "try_update_date_config"))
}

pub fn try_update_essence_allocation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user_and_essence_list: Vec<(String, EssenceInfo)>,
    _total_essence: EssenceInfo,
) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let block_time = env.block.time.seconds();
    let AddressConfig {
        admin,
        eclipsepad_staking,
        ..
    } = ADDRESS_CONFIG.load(deps.storage)?;
    try_unlock(deps.storage, block_time)?;

    if sender != eclipsepad_staking && sender != admin {
        Err(ContractError::Unauthorized)?;
    }

    for (user_address, user_essence_after) in user_and_essence_list {
        let user = &Addr::unchecked(user_address);
        let user_type = get_user_type(deps.storage, user).unwrap_or(UserType::Slacker);
        let user_essence_before = USER_ESSENCE.load(deps.storage, user).unwrap_or_default();

        // collect rewards
        let (is_updated, user_rewards) = get_accumulated_rewards(deps.storage, user, block_time)?;
        if is_updated {
            USER_REWARDS.save(deps.storage, user, &user_rewards)?;
            ELECTOR_WEIGHTS_REF.remove(deps.storage, user);
        }

        // update user essence
        if user_essence_after.is_zero() {
            USER_ESSENCE.remove(deps.storage, user);
        } else {
            USER_ESSENCE.save(deps.storage, user, &user_essence_after)?;
        }

        match user_type {
            UserType::Elector => {
                let user_weights = get_user_weights(deps.storage, user)?;
                let user_essence_allocation_before =
                    calc_essence_allocation(&user_essence_before, &user_weights);
                let user_essence_allocation_after =
                    calc_essence_allocation(&user_essence_after, &user_weights);

                let elector_essence_acc_before = ELECTOR_ESSENCE_ACC.load(deps.storage)?;
                let elector_weights_acc_before = ELECTOR_WEIGHTS_ACC.load(deps.storage)?;
                let elector_essence_allocation_acc_before = calc_essence_allocation(
                    &elector_essence_acc_before,
                    &elector_weights_acc_before,
                );
                let elector_essence_allocation_acc_after = calc_updated_essence_allocation(
                    &elector_essence_allocation_acc_before,
                    &user_essence_allocation_after,
                    &user_essence_allocation_before,
                );
                let (elector_essence_acc_after, elector_weights_acc_after) =
                    calc_weights_from_essence_allocation(
                        &elector_essence_allocation_acc_after,
                        block_time,
                    );

                ELECTOR_ESSENCE_ACC.save(deps.storage, &elector_essence_acc_after)?;
                ELECTOR_WEIGHTS_ACC.save(deps.storage, &elector_weights_acc_after)?;
            }
            UserType::Delegator => {
                DAO_ESSENCE_ACC.update(deps.storage, |x| -> StdResult<EssenceInfo> {
                    Ok(x.add(&user_essence_after).sub(&user_essence_before))
                })?;
            }
            UserType::Slacker => {
                SLACKER_ESSENCE_ACC.update(deps.storage, |x| -> StdResult<EssenceInfo> {
                    Ok(x.add(&user_essence_after).sub(&user_essence_before))
                })?;
            }
        };
    }

    Ok(Response::new().add_attribute("action", "try_update_essence_allocation"))
}

pub fn try_swap_to_eclip_astro(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let (sender_address, asset_amount, asset_info) = check_funds(
        deps.as_ref(),
        &info,
        FundsType::Single {
            sender: None,
            amount: None,
        },
    )?;
    let token_in = asset_info.try_get_native()?;
    let AddressConfig {
        astroport_staking, ..
    } = ADDRESS_CONFIG.load(deps.storage)?;
    let TokenConfig { astro, xastro, .. } = TOKEN_CONFIG.load(deps.storage)?;

    // check if ASTRO or xASTRO was sent
    if token_in != astro && token_in != xastro {
        Err(ContractError::UnknownToken(token_in.to_string()))?;
    }

    // check if amount isn't zero
    if asset_amount.is_zero() {
        Err(ContractError::ZeroAmount)?;
    }

    // get xastro first
    if token_in == astro {
        RECIPIENT.save(deps.storage, &sender_address)?;

        let msg = SubMsg {
            id: STAKE_ASTRO_REPLY_ID,
            msg: WasmMsg::Execute {
                contract_addr: astroport_staking.to_string(),
                msg: to_json_binary(&astroport::staking::ExecuteMsg::Enter { receiver: None })?,
                funds: coins(asset_amount.u128(), astro),
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Success,
        };

        return Ok(Response::new().add_submessage(msg));
    }

    lock_xastro(deps, env, asset_amount, &sender_address)
}

pub fn handle_stake_astro_reply(
    deps: DepsMut,
    env: Env,
    result: &SubMsgResult,
) -> Result<Response, ContractError> {
    let res = result
        .to_owned()
        .into_result()
        .map_err(|_| ContractError::StakeError)?;

    let mut xastro_amount = Uint128::zero();
    for event in res.events.iter() {
        for attr in event.attributes.iter() {
            if attr.key == "xastro_amount" {
                xastro_amount = Uint128::from_str(&attr.value).unwrap();
            }
        }
    }

    let recipient = &RECIPIENT.load(deps.storage)?;
    lock_xastro(deps, env, xastro_amount, recipient)
}

fn lock_xastro(
    deps: DepsMut,
    _env: Env,
    xastro_amount: Uint128,
    recipient: &Addr,
) -> Result<Response, ContractError> {
    let AddressConfig {
        astroport_staking,
        astroport_voting_escrow,
        eclipsepad_minter,
        ..
    } = ADDRESS_CONFIG.load(deps.storage)?;
    let TokenConfig {
        xastro,
        eclip_astro,
        ..
    } = TOKEN_CONFIG.load(deps.storage)?;

    // calculate eclipASTRO amount
    let total_xastro_amount: Uint128 = deps.querier.query_wasm_smart(
        astroport_staking.to_string(),
        &astroport::staking::QueryMsg::TotalShares {},
    )?;
    let total_astro_amount: Uint128 = deps.querier.query_wasm_smart(
        astroport_staking.to_string(),
        &astroport::staking::QueryMsg::TotalDeposit {},
    )?;
    let eclip_astro_amount = total_astro_amount * xastro_amount / total_xastro_amount;

    let msg_list = vec![
        // replenish existent lock or create new one
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: astroport_voting_escrow.to_string(),
            msg: to_json_binary(&astroport_governance::voting_escrow::ExecuteMsg::Lock {
                receiver: None,
            })?,
            funds: coins(xastro_amount.u128(), xastro),
        }),
        // mint eclipAstro to user
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: eclipsepad_minter.to_string(),
            msg: to_json_binary(&eclipse_base::minter::msg::ExecuteMsg::Mint {
                token: TokenUnverified::new_native(&eclip_astro),
                amount: eclip_astro_amount,
                recipient: recipient.to_string(),
            })?,
            funds: vec![],
        }),
    ];

    Ok(Response::new()
        .add_messages(msg_list)
        .add_attribute("action", "try_swap_to_eclip_astro")
        .add_attribute("eclip_astro_amount", eclip_astro_amount))
}

pub fn try_delegate(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let block_time = env.block.time.seconds();
    try_unlock_and_check(deps.storage, block_time)?;

    let user = &info.sender;
    let user_type = get_user_type(deps.storage, user)?;
    let user_essence_before = USER_ESSENCE.load(deps.storage, user)?;

    // collect rewards
    let (is_updated, user_rewards) = get_accumulated_rewards(deps.storage, user, block_time)?;
    if is_updated {
        USER_REWARDS.save(deps.storage, user, &user_rewards)?;
        ELECTOR_WEIGHTS_REF.remove(deps.storage, user);
    }

    match user_type {
        UserType::Elector => {
            let user_weights = get_user_weights(deps.storage, user)?;
            let user_essence_allocation_before =
                calc_essence_allocation(&user_essence_before, &user_weights);
            let user_essence_allocation_after =
                calc_essence_allocation(&EssenceInfo::default(), &user_weights);

            let elector_essence_acc_before = ELECTOR_ESSENCE_ACC.load(deps.storage)?;
            let elector_weights_acc_before = ELECTOR_WEIGHTS_ACC.load(deps.storage)?;
            let elector_essence_allocation_acc_before =
                calc_essence_allocation(&elector_essence_acc_before, &elector_weights_acc_before);
            let elector_essence_allocation_acc_after = calc_updated_essence_allocation(
                &elector_essence_allocation_acc_before,
                &user_essence_allocation_after,
                &user_essence_allocation_before,
            );
            let (elector_essence_acc_after, elector_weights_acc_after) =
                calc_weights_from_essence_allocation(
                    &elector_essence_allocation_acc_after,
                    block_time,
                );

            ELECTOR_WEIGHTS.remove(deps.storage, user);
            ELECTOR_ESSENCE_ACC.save(deps.storage, &elector_essence_acc_after)?;
            ELECTOR_WEIGHTS_ACC.save(deps.storage, &elector_weights_acc_after)?;

            DELEGATOR_ADDRESSES.save(deps.storage, user, &true)?;
            DAO_ESSENCE_ACC.update(deps.storage, |x| -> StdResult<EssenceInfo> {
                Ok(x.add(&user_essence_before))
            })?;
        }
        UserType::Delegator => Err(ContractError::DelegateTwice)?,
        UserType::Slacker => {
            DELEGATOR_ADDRESSES.save(deps.storage, user, &true)?;

            SLACKER_ESSENCE_ACC.update(deps.storage, |x| -> StdResult<EssenceInfo> {
                Ok(x.sub(&user_essence_before))
            })?;
            DAO_ESSENCE_ACC.update(deps.storage, |x| -> StdResult<EssenceInfo> {
                Ok(x.add(&user_essence_before))
            })?;
        }
    };

    Ok(Response::new().add_attribute("action", "try_delegate"))
}

pub fn try_undelegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let block_time = env.block.time.seconds();
    try_unlock_and_check(deps.storage, block_time)?;

    let user = &info.sender;
    let user_type = get_user_type(deps.storage, user)?;
    let user_essence = USER_ESSENCE.load(deps.storage, user)?;

    // collect rewards
    let (is_updated, user_rewards) = get_accumulated_rewards(deps.storage, user, block_time)?;
    if is_updated {
        USER_REWARDS.save(deps.storage, user, &user_rewards)?;
        ELECTOR_WEIGHTS_REF.remove(deps.storage, user);
    }

    match user_type {
        UserType::Elector => Err(ContractError::DelegatorIsNotFound)?,
        UserType::Delegator => {
            DELEGATOR_ADDRESSES.remove(deps.storage, user);

            DAO_ESSENCE_ACC.update(deps.storage, |x| -> StdResult<EssenceInfo> {
                Ok(x.sub(&user_essence))
            })?;
            SLACKER_ESSENCE_ACC.update(deps.storage, |x| -> StdResult<EssenceInfo> {
                Ok(x.add(&user_essence))
            })?;
        }
        UserType::Slacker => Err(ContractError::DelegatorIsNotFound)?,
    };

    Ok(Response::new().add_attribute("action", "try_undelegate"))
}

pub fn try_place_vote(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    weight_allocation: Vec<WeightAllocationItem>,
) -> Result<Response, ContractError> {
    let block_time = env.block.time.seconds();
    try_unlock_and_check(deps.storage, block_time)?;
    verify_weight_allocation(deps.as_ref(), &weight_allocation)?;

    let user = &info.sender;
    let user_type = get_user_type(deps.storage, user)?;
    let user_essence = USER_ESSENCE.load(deps.storage, user)?;

    // collect rewards
    let (is_updated, user_rewards) = get_accumulated_rewards(deps.storage, user, block_time)?;
    if is_updated {
        USER_REWARDS.save(deps.storage, user, &user_rewards)?;
        ELECTOR_WEIGHTS_REF.remove(deps.storage, user);
    }

    match user_type {
        UserType::Elector => {}
        UserType::Delegator => Err(ContractError::DelegatorCanNotVote)?,
        UserType::Slacker => {
            SLACKER_ESSENCE_ACC.update(deps.storage, |x| -> StdResult<EssenceInfo> {
                Ok(x.sub(&user_essence))
            })?;
        }
    };

    // update elector
    let user_weights_before = ELECTOR_WEIGHTS.load(deps.storage, user).unwrap_or_default();
    let user_essence_allocation_before =
        calc_essence_allocation(&user_essence, &user_weights_before);
    let user_essence_allocation_after = calc_essence_allocation(&user_essence, &weight_allocation);

    let elector_essence_acc_before = ELECTOR_ESSENCE_ACC.load(deps.storage)?;
    let elector_weights_acc_before = ELECTOR_WEIGHTS_ACC.load(deps.storage)?;
    let elector_essence_allocation_acc_before =
        calc_essence_allocation(&elector_essence_acc_before, &elector_weights_acc_before);
    let elector_essence_allocation_acc_after = calc_updated_essence_allocation(
        &elector_essence_allocation_acc_before,
        &user_essence_allocation_after,
        &user_essence_allocation_before,
    );
    let (elector_essence_acc_after, elector_weights_acc_after) =
        calc_weights_from_essence_allocation(&elector_essence_allocation_acc_after, block_time);

    ELECTOR_WEIGHTS.save(deps.storage, user, &weight_allocation)?;
    ELECTOR_WEIGHTS_REF.save(deps.storage, user, &weight_allocation)?;

    ELECTOR_ESSENCE_ACC.save(deps.storage, &elector_essence_acc_after)?;
    ELECTOR_WEIGHTS_ACC.save(deps.storage, &elector_weights_acc_after)?;

    Ok(Response::new().add_attribute("action", "try_place_vote"))
}

pub fn try_place_vote_as_dao(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    weight_allocation: Vec<WeightAllocationItem>,
) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let block_time = env.block.time.seconds();
    let AddressConfig { eclipse_dao, .. } = ADDRESS_CONFIG.load(deps.storage)?;
    // TODO: replace with RewardsClaimStage checker
    try_unlock_and_check(deps.storage, block_time)?;
    verify_weight_allocation(deps.as_ref(), &weight_allocation)?;

    if IS_LOCKED.load(deps.storage)? {
        Err(ContractError::EpochEnd)?;
    }

    if sender != eclipse_dao {
        Err(ContractError::Unauthorized)?;
    }

    DAO_WEIGHTS_ACC.save(deps.storage, &weight_allocation)?;

    Ok(Response::new().add_attribute("action", "try_place_vote_as_dao"))
}

pub fn try_vote(
    deps: DepsMut,
    env: Env,
    // info: MessageInfo
) -> Result<Response, ContractError> {
    let block_time = env.block.time.seconds();
    let rewards_claim_stage = REWARDS_CLAIM_STAGE.load(deps.storage)?;
    let AddressConfig {
        astroport_emission_controller,
        ..
    } = ADDRESS_CONFIG.load(deps.storage)?;
    let DateConfig {
        epoch_length,
        vote_delay,
        ..
    } = DATE_CONFIG.load(deps.storage)?;
    let mut current_epoch = EPOCH_COUNTER.load(deps.storage)?;

    // only swapped -> unclaimed transition is allowed
    if !matches!(rewards_claim_stage, RewardsClaimStage::Swapped) {
        Err(ContractError::WrongRewardsClaimStage)?;
    }
    REWARDS_CLAIM_STAGE.save(deps.storage, &RewardsClaimStage::Unclaimed)?;

    // final voting must be executed single time right before epoch end
    if IS_LOCKED.load(deps.storage)? {
        Err(ContractError::EpochEnd)?;
    }

    if block_time < current_epoch.start_date + vote_delay {
        Err(ContractError::VotingDelay)?;
    }

    // will be unlocked on next epoch
    IS_LOCKED.save(deps.storage, &true)?;

    let elector_essence_acc_before = ELECTOR_ESSENCE_ACC.load(deps.storage)?;
    let elector_weights_acc_before = ELECTOR_WEIGHTS_ACC.load(deps.storage)?;
    let dao_essence_acc_before = DAO_ESSENCE_ACC.load(deps.storage)?;
    let dao_weights_acc_before = DAO_WEIGHTS_ACC.load(deps.storage)?;
    let slacker_essence = SLACKER_ESSENCE_ACC.load(deps.storage)?;
    let elector_additional_essence_fraction = str_to_dec(ELECTOR_ADDITIONAL_ESSENCE_FRACTION);

    let (_total_essence_allocation, total_weights_allocation) =
        get_total_votes(deps.storage, block_time)?;

    // update vote results
    VOTE_RESULTS.update(deps.storage, |mut x| -> StdResult<Vec<VoteResults>> {
        x.push(VoteResults {
            epoch_id: current_epoch.id,
            end_date: current_epoch.start_date + epoch_length,

            elector_essence: elector_essence_acc_before
                .add(&slacker_essence.scale(elector_additional_essence_fraction))
                .capture(block_time),
            dao_essence: dao_essence_acc_before
                .add(&slacker_essence.scale(Decimal::one() - elector_additional_essence_fraction))
                .capture(block_time),
            slacker_essence: slacker_essence.capture(block_time),

            elector_weights: elector_weights_acc_before,
            dao_weights: dao_weights_acc_before,

            dao_treasury_eclip_rewards: Uint128::zero(), // will be updated on claim and swap
            dao_delegators_eclip_rewards: Uint128::zero(), // will be updated on claim and swap
            pool_info_list: total_weights_allocation
                .iter()
                .cloned()
                .map(|(lp_token, weight)| PoolInfoItem {
                    lp_token,
                    weight,
                    rewards: vec![], // will be updated on claim and swap
                })
                .collect(),
        });
        x.retain(|y| y.epoch_id + MAX_EPOCH_AMOUNT > current_epoch.id);
        Ok(x)
    })?;

    // reset elector votes to motivate them vote again in next epoch
    ELECTOR_WEIGHTS.clear(deps.storage);
    ELECTOR_WEIGHTS_ACC.save(deps.storage, &vec![])?;
    ELECTOR_ESSENCE_ACC.save(deps.storage, &EssenceInfo::default())?;
    SLACKER_ESSENCE_ACC.update(deps.storage, |x| -> StdResult<EssenceInfo> {
        Ok(x.add(&elector_essence_acc_before))
    })?;
    // reset dao votes as well
    DAO_WEIGHTS_ACC.save(deps.storage, &vec![])?;

    // update epoch counter
    current_epoch.id += 1;
    current_epoch.start_date += epoch_length;
    EPOCH_COUNTER.save(deps.storage, &current_epoch)?;

    // send vote msg
    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: astroport_emission_controller.to_string(),
        msg: to_json_binary(
            &astroport_governance::emissions_controller::msg::ExecuteMsg::<Empty>::Vote {
                votes: total_weights_allocation,
            },
        )?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "try_vote"))
}

// TODO: split claim and swap as tx is still heavy, add state machine
pub fn try_claim(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let block_time = env.block.time.seconds();
    let epoch = EPOCH_COUNTER.load(deps.storage)?;
    let rewards_claim_stage = REWARDS_CLAIM_STAGE.load(deps.storage)?;
    let AddressConfig {
        astroport_tribute_market,
        astroport_emission_controller,
        astroport_voting_escrow,
        ..
    } = ADDRESS_CONFIG.load(deps.storage)?;
    let astroport_tribute_market =
        &unwrap_field(astroport_tribute_market, "astroport_tribute_market")?;

    // only unclaimed -> claimed transition is allowed
    if !matches!(rewards_claim_stage, RewardsClaimStage::Unclaimed) {
        Err(ContractError::WrongRewardsClaimStage)?;
    }
    REWARDS_CLAIM_STAGE.save(deps.storage, &RewardsClaimStage::Claimed)?;

    // execute only on new epoch
    if block_time < epoch.start_date {
        Err(ContractError::EpochIsNotStarted)?;
    }

    // check rewards
    let rewards = deps
        .querier
        .query_wasm_smart::<Vec<(Uint128, String)>>(
            astroport_tribute_market,
            &tribute_market_mocks::msg::QueryMsg::Rewards {
                user: env.contract.address.to_string(),
            },
        )
        .unwrap_or_default();

    if rewards.is_empty() {
        Err(ContractError::RewardsAreNotFound)?;
    }

    // TODO: add equinox tribute market
    // get voter bribes allocation:
    // 1) query tribute market bribes allocation
    let tribute_market_bribe_allocation =
        deps.querier.query_wasm_smart::<Vec<BribesAllocationItem>>(
            astroport_tribute_market,
            &tribute_market_mocks::msg::QueryMsg::BribesAllocation {},
        )?;

    // 2) query voter voting power
    let voter_voting_power = deps.querier.query_wasm_smart::<Uint128>(
        astroport_voting_escrow,
        &astroport_governance::voting_escrow::QueryMsg::UserVotingPower {
            user: env.contract.address.to_string(),
            timestamp: Some(epoch.start_date),
        },
    )?;
    let voter_voting_power_decimal = u128_to_dec(voter_voting_power);

    // 3) get voter to tribute market voting power ratio allocation
    let voter_to_tribute_voting_power_ratio_allocation = deps
        .querier
        .query_wasm_smart::<UserInfoResponse>(
            astroport_emission_controller.clone(),
            &astroport_governance::emissions_controller::hub::QueryMsg::UserInfo {
                user: env.contract.address.to_string(),
                timestamp: Some(epoch.start_date),
            },
        )?
        .applied_votes
        .iter()
        .map(|(lp_token, weight)| -> StdResult<(String, Decimal)> {
            let tribute_market_voting_power = deps
                .querier
                .query_wasm_smart::<VotedPoolInfo>(
                    astroport_emission_controller.clone(),
                    &astroport_governance::emissions_controller::hub::QueryMsg::VotedPool {
                        pool: lp_token.to_owned(),
                        timestamp: Some(epoch.start_date),
                    },
                )?
                .voting_power;

            let ratio = if tribute_market_voting_power.is_zero() {
                Decimal::zero()
            } else {
                voter_voting_power_decimal * weight / u128_to_dec(tribute_market_voting_power)
            };

            Ok((lp_token.to_owned(), ratio))
        })
        .collect::<StdResult<Vec<(String, Decimal)>>>()?;

    // 4) update vote results
    let mut vote_results = VOTE_RESULTS.load(deps.storage)?;

    // compare pools from vote results and applied votes
    let last_vote_results = &vote_results
        .iter()
        .last()
        .ok_or(ContractError::LastVoteResultsAreNotFound)?
        .pool_info_list;

    let applied_votes_pool_list: Vec<String> = voter_to_tribute_voting_power_ratio_allocation
        .iter()
        .map(|(lp_token, _ratio)| lp_token.to_owned())
        .collect();

    if !(last_vote_results.len() == applied_votes_pool_list.len()
        && last_vote_results
            .iter()
            .all(|x| applied_votes_pool_list.contains(&x.lp_token)))
    {
        Err(ContractError::UnequalPools)?;
    }

    vote_results = vote_results
        .into_iter()
        .map(|mut x| {
            if x.epoch_id + 1 == epoch.id {
                x.pool_info_list = calc_pool_info_list_with_rewards(
                    &x.pool_info_list,
                    &tribute_market_bribe_allocation,
                    &voter_to_tribute_voting_power_ratio_allocation,
                );
            }

            x
        })
        .collect();
    VOTE_RESULTS.save(deps.storage, &vote_results)?;

    // claim rewards
    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: astroport_tribute_market.to_string(),
        msg: to_json_binary(&tribute_market_mocks::msg::ExecuteMsg::ClaimRewards {})?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "try_claim"))
}

pub fn try_swap(deps: DepsMut, _env: Env) -> Result<Response, ContractError> {
    let mut response = Response::new().add_attribute("action", "try_swap");

    let epoch = EPOCH_COUNTER.load(deps.storage)?;
    let rewards_claim_stage = REWARDS_CLAIM_STAGE.load(deps.storage)?;
    let TokenConfig { eclip, .. } = TOKEN_CONFIG.load(deps.storage)?;
    let AddressConfig {
        astroport_router, ..
    } = ADDRESS_CONFIG.load(deps.storage)?;

    // only claimed -> swapped transition is allowed
    if !matches!(rewards_claim_stage, RewardsClaimStage::Claimed) {
        Err(ContractError::WrongRewardsClaimStage)?;
    }

    // write elector rewards in vote results
    let mut vote_results = VOTE_RESULTS.load(deps.storage)?;
    let vote_results_last = vote_results.last().ok_or(ContractError::Unauthorized)?;

    let (pool_info_list_with_elector_rewards, dao_rewards) = split_rewards(
        &vote_results_last.pool_info_list,
        &vote_results_last.dao_weights,
        vote_results_last.elector_essence,
        vote_results_last.dao_essence,
    );

    vote_results = vote_results
        .into_iter()
        .map(|mut x| {
            if x.epoch_id + 1 == epoch.id {
                x.pool_info_list = pool_info_list_with_elector_rewards.clone();
            }

            x
        })
        .collect();
    VOTE_RESULTS.save(deps.storage, &vote_results)?;

    // swap dao rewards to eclip
    let mut swap_rewards_reply_cnt = SWAP_REWARDS_REPLY_ID_CNT.load(deps.storage)?;

    for (amount_in, denom_in) in dao_rewards {
        if denom_in == eclip {
            TEMPORARY_REWARDS.save(deps.storage, &amount_in)?;
            continue;
        }

        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: astroport_router.to_string(),
            msg: to_json_binary(&astroport::router::ExecuteMsg::ExecuteSwapOperations {
                operations: get_route(deps.storage, &denom_in)?,
                minimum_receive: None,
                to: None,
                max_spread: None,
            })?,
            funds: coins(amount_in.u128(), &denom_in),
        });

        // use reply id with counter to process eclip rewards on last swap
        let swap_rewards_reply_id = SWAP_REWARDS_REPLY_ID_MIN + swap_rewards_reply_cnt as u64;
        let submsg = SubMsg::reply_on_success(msg, swap_rewards_reply_id);
        response = response.add_submessage(submsg);

        swap_rewards_reply_cnt = swap_rewards_reply_cnt
            .checked_add(1)
            .ok_or(ContractError::ReplyIdCounterOverflow)?;
    }

    SWAP_REWARDS_REPLY_ID_CNT.save(deps.storage, &swap_rewards_reply_cnt)?;

    Ok(response)
}

pub fn handle_swap_reply(
    deps: DepsMut,
    _env: Env,
    result: &SubMsgResult,
) -> Result<Response, ContractError> {
    let mut response = Response::new();
    let res = result
        .to_owned()
        .into_result()
        .map_err(|_| ContractError::StakeError)?;

    let eclip_amount = res
        .events
        .iter()
        .rev()
        .find(|x| x.ty == "wasm")
        .ok_or(ContractError::EventIsNotFound)?
        .attributes
        .iter()
        .find(|x| x.key == "return_amount")
        .ok_or(ContractError::AttributeIsNotFound)?
        .value
        .parse::<Uint128>()?;

    let swap_rewards_reply_cnt = SWAP_REWARDS_REPLY_ID_CNT.load(deps.storage)?;
    SWAP_REWARDS_REPLY_ID_CNT.save(deps.storage, &(swap_rewards_reply_cnt - 1))?;
    response = response.add_attribute("reply_cnt", (swap_rewards_reply_cnt - 1).to_string());

    // continue swap rewards
    if swap_rewards_reply_cnt > 1 {
        TEMPORARY_REWARDS.update(deps.storage, |x| -> StdResult<Uint128> {
            Ok(x + eclip_amount)
        })?;

        return Ok(response);
    }

    // only claimed -> swapped transition is allowed
    REWARDS_CLAIM_STAGE.save(deps.storage, &RewardsClaimStage::Swapped)?;

    // distribute to previous epoch according to weights
    let temporary_rewards = eclip_amount + TEMPORARY_REWARDS.load(deps.storage)?;
    TEMPORARY_REWARDS.save(deps.storage, &Uint128::zero())?;

    // split rewards
    let (dao_treasury_eclip_rewards, delegator_rewards) =
        split_dao_eclip_rewards(temporary_rewards);

    let epoch = EPOCH_COUNTER.load(deps.storage)?;
    VOTE_RESULTS.update(deps.storage, |x| -> StdResult<Vec<VoteResults>> {
        let vote_results = x
            .into_iter()
            .map(|mut y| {
                if y.epoch_id + 1 == epoch.id {
                    y.dao_treasury_eclip_rewards = dao_treasury_eclip_rewards;
                    y.dao_delegators_eclip_rewards = delegator_rewards;
                }

                y
            })
            .collect();

        Ok(vote_results)
    })?;

    // send eclip rewards to dao treasury
    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: ADDRESS_CONFIG.load(deps.storage)?.eclipse_dao.to_string(),
        amount: coins(
            dao_treasury_eclip_rewards.u128(),
            TOKEN_CONFIG.load(deps.storage)?.eclip,
        ),
    });
    response = response.add_message(msg);

    Ok(response)
}

pub fn try_claim_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let user = &info.sender;
    let block_time = env.block.time.seconds();

    // collect rewards
    let (_is_updated, mut user_rewards) = get_accumulated_rewards(deps.storage, user, block_time)?;
    if user_rewards.value.is_empty() {
        Err(ContractError::RewardsAreNotFound)?;
    }

    // send rewards to user
    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: user.to_string(),
        amount: user_rewards
            .value
            .into_iter()
            .map(|(amount, denom)| coin(amount.u128(), denom))
            .collect(),
    });

    // update storages
    user_rewards.value = vec![];
    USER_REWARDS.save(deps.storage, user, &user_rewards)?;
    ELECTOR_WEIGHTS_REF.remove(deps.storage, user);

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "try_claim_rewards"))
}

pub fn try_update_route_list(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    route_list: Vec<RouteListItem>,
) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let AddressConfig { admin, .. } = ADDRESS_CONFIG.load(deps.storage)?;

    if sender != admin {
        Err(ContractError::Unauthorized)?;
    }

    for RouteListItem { denom, route } in route_list {
        ROUTE_CONFIG.save(deps.storage, &denom, &route)?;
    }

    Ok(Response::new().add_attribute("action", "try_rewrite_route_list"))
}
