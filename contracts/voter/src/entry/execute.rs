use std::str::FromStr;

use cosmwasm_std::{
    coins, to_json_binary, Addr, CosmosMsg, Decimal, DepsMut, Empty, Env, MessageInfo, ReplyOn,
    Response, StdResult, Storage, SubMsg, SubMsgResult, Uint128, WasmMsg,
};

use eclipse_base::{
    assets::TokenUnverified,
    converters::{str_to_dec, u128_to_dec},
    utils::{check_funds, FundsType},
};
use equinox_msg::voter::{
    AddressConfig, DateConfig, EssenceAllocationItem, EssenceInfo, PoolInfoItem, TokenConfig,
    TransferAdminState, VoteResults, WeightAllocationItem,
};

use crate::{
    error::ContractError,
    helpers::{try_unlock, try_unlock_and_check, verify_weight_allocation},
    math::{
        calc_essence_allocation, calc_scaled_essence_allocation, calc_updated_essence_allocation,
        calc_weights_from_essence_allocation,
    },
    state::{
        ADDRESS_CONFIG, DAO_ESSENCE, DAO_WEIGHTS, DATE_CONFIG, DELEGATOR_ESSENCE,
        ELECTOR_ADDITIONAL_ESSENCE_FRACTION, ELECTOR_ESSENCE, ELECTOR_VOTES, ELECTOR_WEIGHTS,
        EPOCH_COUNTER, IS_LOCKED, MAX_EPOCH_AMOUNT, RECIPIENT, SLACKER_ESSENCE,
        SLACKER_ESSENCE_ACC, STAKE_ASTRO_REPLY_ID, TOKEN_CONFIG, TOTAL_VOTES, TRANSFER_ADMIN_STATE,
        TRANSFER_ADMIN_TIMEOUT, VOTE_RESULTS,
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
    astro: Option<String>,
    xastro: Option<String>,
    eclip_astro: Option<String>,
) -> Result<Response, ContractError> {
    let AddressConfig { admin, .. } = ADDRESS_CONFIG.load(deps.storage)?;
    let mut config = TOKEN_CONFIG.load(deps.storage)?;

    if info.sender != admin {
        Err(ContractError::Unauthorized)?;
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

    for (user_address, essence_after) in user_and_essence_list {
        let user = &Addr::unchecked(user_address);

        // check if user is elector and update
        if let Ok(essence_before) = ELECTOR_ESSENCE.load(deps.storage, &user) {
            // update own essence
            if essence_after.is_zero() {
                ELECTOR_ESSENCE.remove(deps.storage, user);
            } else {
                ELECTOR_ESSENCE.save(deps.storage, user, &essence_after)?;
            }

            // if elector updated weights after new epoch start change all electors and total allocations
            // otherwise it will be updated on vote by elector
            if let Ok(weights) = ELECTOR_WEIGHTS.load(deps.storage, user) {
                let essence_allocation_before = calc_essence_allocation(&essence_before, &weights);
                let essence_allocation_after = calc_essence_allocation(&essence_after, &weights);

                ELECTOR_VOTES.update(
                    deps.storage,
                    |x| -> StdResult<Vec<EssenceAllocationItem>> {
                        Ok(calc_updated_essence_allocation(
                            &x,
                            &essence_allocation_after,
                            &essence_allocation_before,
                        ))
                    },
                )?;

                TOTAL_VOTES.update(deps.storage, |x| -> StdResult<Vec<EssenceAllocationItem>> {
                    Ok(calc_updated_essence_allocation(
                        &x,
                        &essence_allocation_after,
                        &essence_allocation_before,
                    ))
                })?;
            }

            continue;
        }

        // check if user is delegator and update
        if let Ok(essence_before) = DELEGATOR_ESSENCE.load(deps.storage, &user) {
            // update own essence
            if essence_after.is_zero() {
                DELEGATOR_ESSENCE.remove(deps.storage, user);
            } else {
                DELEGATOR_ESSENCE.save(deps.storage, user, &essence_after)?;
            }

            // update DAO and total allocations
            DAO_ESSENCE.update(deps.storage, |x| -> StdResult<EssenceInfo> {
                Ok(x.add(&essence_after).sub(&essence_before))
            })?;

            let weights = DAO_WEIGHTS.load(deps.storage)?;
            let essence_allocation_before = calc_essence_allocation(&essence_before, &weights);
            let essence_allocation_after = calc_essence_allocation(&essence_after, &weights);

            TOTAL_VOTES.update(deps.storage, |x| -> StdResult<Vec<EssenceAllocationItem>> {
                Ok(calc_updated_essence_allocation(
                    &x,
                    &essence_allocation_after,
                    &essence_allocation_before,
                ))
            })?;

            continue;
        }

        // update/add user as slacker
        let essence_before = SLACKER_ESSENCE.load(deps.storage, user).unwrap_or_default();

        // update own essence
        if essence_after.is_zero() {
            SLACKER_ESSENCE.remove(deps.storage, user);
        } else {
            SLACKER_ESSENCE.save(deps.storage, user, &essence_after)?;
        }

        // update slackers essence accumulator
        SLACKER_ESSENCE_ACC.update(deps.storage, |x| -> StdResult<EssenceInfo> {
            Ok(x.add(&essence_after).sub(&essence_before))
        })?;
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
    let sender = &info.sender;
    let block_time = env.block.time.seconds();
    try_unlock_and_check(deps.storage, block_time)?;

    // slacker -> delegator
    if let Ok(essence) = SLACKER_ESSENCE.load(deps.storage, sender) {
        // update own essence
        SLACKER_ESSENCE.remove(deps.storage, sender);

        // update slackers essence accumulator
        SLACKER_ESSENCE_ACC.update(deps.storage, |x| -> StdResult<EssenceInfo> {
            Ok(x.sub(&essence))
        })?;

        return add_delegator(deps.storage, sender, &essence);
    }

    // elector -> delegator
    if let Ok(essence) = ELECTOR_ESSENCE.load(deps.storage, sender) {
        // update own essence
        ELECTOR_ESSENCE.remove(deps.storage, sender);

        // if elector updated weights after new epoch start change all electors and total allocations
        // otherwise it will be updated on vote by elector
        if let Ok(weights) = ELECTOR_WEIGHTS.load(deps.storage, sender) {
            let essence_allocation_before = calc_essence_allocation(&essence, &weights);
            let essence_allocation_after =
                calc_essence_allocation(&EssenceInfo::default(), &weights);

            ELECTOR_VOTES.update(deps.storage, |x| -> StdResult<Vec<EssenceAllocationItem>> {
                Ok(calc_updated_essence_allocation(
                    &x,
                    &essence_allocation_after,
                    &essence_allocation_before,
                ))
            })?;

            TOTAL_VOTES.update(deps.storage, |x| -> StdResult<Vec<EssenceAllocationItem>> {
                Ok(calc_updated_essence_allocation(
                    &x,
                    &essence_allocation_after,
                    &essence_allocation_before,
                ))
            })?;
        }

        return add_delegator(deps.storage, sender, &essence);
    }

    Err(ContractError::DelegateTwice)
}

fn add_delegator(
    storage: &mut dyn Storage,
    sender: &Addr,
    essence: &EssenceInfo,
) -> Result<Response, ContractError> {
    // update own essence
    DELEGATOR_ESSENCE.save(storage, sender, essence)?;

    // update DAO and total allocations
    DAO_ESSENCE.update(storage, |x| -> StdResult<EssenceInfo> {
        Ok(x.add(&essence))
    })?;

    let weights = DAO_WEIGHTS.load(storage)?;
    let essence_allocation_before = calc_essence_allocation(&EssenceInfo::default(), &weights);
    let essence_allocation_after = calc_essence_allocation(&essence, &weights);

    TOTAL_VOTES.update(storage, |x| -> StdResult<Vec<EssenceAllocationItem>> {
        Ok(calc_updated_essence_allocation(
            &x,
            &essence_allocation_after,
            &essence_allocation_before,
        ))
    })?;

    Ok(Response::new().add_attribute("action", "try_delegate"))
}

pub fn try_undelegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let block_time = env.block.time.seconds();
    try_unlock_and_check(deps.storage, block_time)?;

    // check if user is delegator and update
    let essence = DELEGATOR_ESSENCE
        .load(deps.storage, sender)
        .map_err(|_| ContractError::DelegatorIsNotFound)?;

    // update own essence
    DELEGATOR_ESSENCE.remove(deps.storage, sender);

    // update DAO and total allocations
    DAO_ESSENCE.update(deps.storage, |x| -> StdResult<EssenceInfo> {
        Ok(x.sub(&essence))
    })?;

    let weights = DAO_WEIGHTS.load(deps.storage)?;
    let essence_allocation_before = calc_essence_allocation(&essence, &weights);
    let essence_allocation_after = calc_essence_allocation(&EssenceInfo::default(), &weights);

    TOTAL_VOTES.update(deps.storage, |x| -> StdResult<Vec<EssenceAllocationItem>> {
        Ok(calc_updated_essence_allocation(
            &x,
            &essence_allocation_after,
            &essence_allocation_before,
        ))
    })?;

    // move to slackers
    // update own essence
    SLACKER_ESSENCE.save(deps.storage, sender, &essence)?;

    // update slackers essence accumulator
    SLACKER_ESSENCE_ACC.update(deps.storage, |x| -> StdResult<EssenceInfo> {
        Ok(x.add(&essence))
    })?;

    Ok(Response::new().add_attribute("action", "try_undelegate"))
}

// TODO: compare current epoch with historical data to try update BRIBE_REWARDS
pub fn try_place_vote(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    weight_allocation: Vec<WeightAllocationItem>,
) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let block_time = env.block.time.seconds();
    let AddressConfig { eclipse_dao, .. } = ADDRESS_CONFIG.load(deps.storage)?;
    try_unlock_and_check(deps.storage, block_time)?;
    verify_weight_allocation(deps.as_ref(), &weight_allocation)?;

    // delegator can't place vote
    if DELEGATOR_ESSENCE.has(deps.storage, sender) {
        Err(ContractError::DelegatorCanNotVote)?;
    }

    // dao can't place vote as regular user
    if sender == eclipse_dao {
        Err(ContractError::Unauthorized)?;
    }

    // if user is slacker move him to electors first
    if let Ok(essence) = SLACKER_ESSENCE.load(deps.storage, sender) {
        // update own essence
        SLACKER_ESSENCE.remove(deps.storage, sender);

        // update slackers essence accumulator
        SLACKER_ESSENCE_ACC.update(deps.storage, |x| -> StdResult<EssenceInfo> {
            Ok(x.sub(&essence))
        })?;

        // update own essence
        ELECTOR_ESSENCE.save(deps.storage, sender, &essence)?;
    }

    // update weights
    let essence = ELECTOR_ESSENCE.load(deps.storage, sender)?;
    let weights_before = ELECTOR_WEIGHTS
        .load(deps.storage, sender)
        .unwrap_or_default();

    let essence_allocation_before = calc_essence_allocation(&essence, &weights_before);
    let essence_allocation_after = calc_essence_allocation(&essence, &weight_allocation);

    ELECTOR_WEIGHTS.save(deps.storage, sender, &weight_allocation)?;

    ELECTOR_VOTES.update(deps.storage, |x| -> StdResult<Vec<EssenceAllocationItem>> {
        Ok(calc_updated_essence_allocation(
            &x,
            &essence_allocation_after,
            &essence_allocation_before,
        ))
    })?;

    TOTAL_VOTES.update(deps.storage, |x| -> StdResult<Vec<EssenceAllocationItem>> {
        Ok(calc_updated_essence_allocation(
            &x,
            &essence_allocation_after,
            &essence_allocation_before,
        ))
    })?;

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
    try_unlock_and_check(deps.storage, block_time)?;
    verify_weight_allocation(deps.as_ref(), &weight_allocation)?;

    if IS_LOCKED.load(deps.storage)? {
        Err(ContractError::EpochEnd)?;
    }

    if sender != eclipse_dao {
        Err(ContractError::Unauthorized)?;
    }

    // update weights
    let essence = DAO_ESSENCE.load(deps.storage)?;
    let weights_before = DAO_WEIGHTS.load(deps.storage)?;

    let essence_allocation_before = calc_essence_allocation(&essence, &weights_before);
    let essence_allocation_after = calc_essence_allocation(&essence, &weight_allocation);

    DAO_WEIGHTS.save(deps.storage, &weight_allocation)?;

    TOTAL_VOTES.update(deps.storage, |x| -> StdResult<Vec<EssenceAllocationItem>> {
        Ok(calc_updated_essence_allocation(
            &x,
            &essence_allocation_after,
            &essence_allocation_before,
        ))
    })?;

    Ok(Response::new().add_attribute("action", "try_place_vote_as_dao"))
}

pub fn try_vote(
    deps: DepsMut,
    env: Env,
    // info: MessageInfo
) -> Result<Response, ContractError> {
    let block_time = env.block.time.seconds();
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

    // final voting must be executed single time right before epoch end
    if IS_LOCKED.load(deps.storage)? {
        Err(ContractError::EpochEnd)?;
    }

    if block_time < current_epoch.start_date + vote_delay {
        Err(ContractError::VotingDelay)?;
    }

    // will be unlocked on next epoch
    IS_LOCKED.save(deps.storage, &true)?;

    // it's required to update TOTAL_VOTES using slackers info
    let mut total_votes = TOTAL_VOTES.load(deps.storage)?;
    let slacker_essence = SLACKER_ESSENCE_ACC.load(deps.storage)?;
    let elector_additional_essence_fraction = str_to_dec(ELECTOR_ADDITIONAL_ESSENCE_FRACTION);
    // 80 % goes to electors
    let elector_votes_before = ELECTOR_VOTES.load(deps.storage)?;
    let (base_essence, base_weights) =
        calc_weights_from_essence_allocation(&elector_votes_before, block_time);
    let elector_votes_after = calc_scaled_essence_allocation(
        &base_essence,
        &base_weights,
        &slacker_essence,
        elector_additional_essence_fraction,
    );
    total_votes =
        calc_updated_essence_allocation(&total_votes, &elector_votes_after, &elector_votes_before);
    // 20 % goes to dao
    let dao_essence = DAO_ESSENCE.load(deps.storage)?;
    let weights_before = DAO_WEIGHTS.load(deps.storage)?;
    let dao_votes_before = calc_essence_allocation(&dao_essence, &weights_before);
    let dao_votes_after = calc_scaled_essence_allocation(
        &dao_essence,
        &weights_before,
        &slacker_essence,
        Decimal::one() - elector_additional_essence_fraction,
    );
    total_votes =
        calc_updated_essence_allocation(&total_votes, &dao_votes_after, &dao_votes_before);

    // update vote results
    let total_essence = total_votes.iter().fold(Uint128::zero(), |acc, cur| {
        acc + cur.essence_info.capture(block_time)
    });
    let total_essence_decimal = u128_to_dec(total_essence);
    let votes: Vec<(String, Decimal)> = total_votes
        .iter()
        .map(|x| {
            (
                x.lp_token.to_string(),
                u128_to_dec(x.essence_info.capture(block_time)) / total_essence_decimal,
            )
        })
        .collect();

    VOTE_RESULTS.update(deps.storage, |mut x| -> StdResult<Vec<VoteResults>> {
        x.push(VoteResults {
            epoch_id: current_epoch.id,
            end_date: current_epoch.start_date + epoch_length,
            essence: total_essence,
            pool_info_list: votes
                .iter()
                .cloned()
                .map(|(lp_token, weight)| PoolInfoItem {
                    lp_token,
                    weight,
                    rewards: Uint128::zero(), // TODO: update when tribute market will be released
                })
                .collect(),
        });
        x.retain(|y| y.epoch_id + MAX_EPOCH_AMOUNT > current_epoch.id);
        Ok(x)
    })?;

    // reset elector votes to motivate them vote again in next epoch
    ELECTOR_WEIGHTS.clear(deps.storage);
    ELECTOR_VOTES.save(deps.storage, &vec![])?;
    // reset dao votes as well
    DAO_WEIGHTS.save(deps.storage, &vec![])?;
    let mut total_votes = TOTAL_VOTES.load(deps.storage)?;
    total_votes = calc_updated_essence_allocation(&total_votes, &vec![], &elector_votes_before);
    total_votes = calc_updated_essence_allocation(&total_votes, &vec![], &dao_votes_before);
    TOTAL_VOTES.save(deps.storage, &total_votes)?;

    // update epoch counter
    current_epoch.id += 1;
    current_epoch.start_date += epoch_length;
    EPOCH_COUNTER.save(deps.storage, &current_epoch)?;

    // send vote msg
    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: astroport_emission_controller.to_string(),
        msg: to_json_binary(
            &astroport_governance::emissions_controller::msg::ExecuteMsg::<Empty>::Vote { votes },
        )?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "try_vote"))
}
