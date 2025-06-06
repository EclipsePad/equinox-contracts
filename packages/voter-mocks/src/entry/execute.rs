use std::{cmp::min, str::FromStr};

use cosmwasm_std::{
    coin, coins, ensure_eq, ensure_ne, to_json_binary, wasm_execute, Addr, BankMsg, CosmosMsg,
    Decimal, DepsMut, Env, MessageInfo, ReplyOn, Response, StdResult, SubMsg, SubMsgResult,
    Uint128, WasmMsg,
};

use eclipse_base::{
    assets::Token,
    converters::str_to_dec,
    error::ContractError,
    utils::{check_funds, get_transfer_msg, unwrap_field, FundsType},
    voter::{
        msg::AstroStakingRewardResponse,
        state::{
            ADDRESS_CONFIG, ASTRO_PENDING_TREASURY_REWARD, ASTRO_STAKING_REWARD_CONFIG,
            DAO_ESSENCE_ACC, DAO_WEIGHTS_ACC, DATE_CONFIG, DELEGATOR_ESSENCE_FRACTIONS,
            ECLIP_ASTRO_MINTED_BY_VOTER, ELECTOR_ADDITIONAL_ESSENCE_FRACTION,
            ELECTOR_BASE_ESSENCE_FRACTION, ELECTOR_ESSENCE_ACC, ELECTOR_WEIGHTS,
            ELECTOR_WEIGHTS_ACC, ELECTOR_WEIGHTS_REF, EPOCH_COUNTER, IS_PAUSED, MAX_EPOCH_AMOUNT,
            RECIPIENT_AND_AMOUNT, REWARDS_CLAIM_STAGE, ROUTE_CONFIG, SLACKER_ESSENCE_ACC,
            STAKE_ASTRO_REPLY_ID, SWAP_REWARDS_REPLY_ID_CNT, SWAP_REWARDS_REPLY_ID_MIN,
            TEMPORARY_REWARDS, TOKEN_CONFIG, TOTAL_CONVERT_INFO, TRANSFER_ADMIN_STATE,
            TRANSFER_ADMIN_TIMEOUT, UNLOCK_XASTRO_REPLY_ID, UNSTAKE_ASTRO_REPLY_ID, USER_ESSENCE,
            USER_REWARDS, VOTE_RESULTS,
        },
        types::{
            AddressConfig, AstroStakingRewardConfig, ConvertInfo, DateConfig, EssenceInfo,
            PoolInfoItem, RewardsClaimStage, RouteListItem, TokenConfig, TransferAdminState,
            UserType, VoteResults, WeightAllocationItem,
        },
    },
};

use crate::{
    entry::query::{_query_astro_staking_rewards, query_voter_xastro},
    helpers::{
        check_pause_state, check_rewards_claim_stage, get_accumulated_rewards,
        get_astro_and_xastro_supply, get_route, get_total_votes, get_user_types, get_user_weights,
        query_astroport_rewards, query_eclipsepad_rewards, split_user_essence_info,
        verify_weight_allocation,
    },
    math::{
        calc_eclip_astro_for_xastro, calc_essence_allocation, calc_splitted_user_essence_info,
        calc_updated_essence_allocation, calc_weights_from_essence_allocation,
        calc_xastro_for_eclip_astro, split_dao_eclip_rewards, split_rewards,
    },
};

pub fn try_pause(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let AddressConfig { admin, .. } = ADDRESS_CONFIG.load(deps.storage)?;

    if sender != admin {
        Err(ContractError::Unauthorized)?;
    }

    IS_PAUSED.save(deps.storage, &true)?;

    Ok(Response::new().add_attribute("action", "try_pause"))
}

pub fn try_unpause(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let AddressConfig { admin, .. } = ADDRESS_CONFIG.load(deps.storage)?;

    if sender != admin {
        Err(ContractError::Unauthorized)?;
    }

    IS_PAUSED.save(deps.storage, &false)?;

    Ok(Response::new().add_attribute("action", "try_unpause"))
}

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
    eclipse_single_sided_vault: Option<String>,
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

    if let Some(x) = eclipse_single_sided_vault {
        config.eclipse_single_sided_vault = Some(deps.api.addr_validate(&x)?);
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
    address_list: Vec<String>,
) -> Result<Response, ContractError> {
    check_rewards_claim_stage(deps.storage)?;
    let sender = &info.sender;
    let block_time = env.block.time.seconds();
    let AddressConfig {
        admin,
        eclipsepad_staking,
        eclipsepad_foundry,
        ..
    } = ADDRESS_CONFIG.load(deps.storage)?;
    let whitelist = match &eclipsepad_foundry {
        Some(eclipsepad_foundry) => vec![
            admin,
            eclipsepad_staking.to_owned(),
            eclipsepad_foundry.to_owned(),
        ],
        None => vec![admin, eclipsepad_staking.to_owned()],
    };

    if !whitelist.contains(sender) {
        Err(ContractError::Unauthorized)?;
    }

    // query full gov essence from splitter or reduced gov essence from staking
    let user_and_essence_list: Vec<(Addr, EssenceInfo)> = match &eclipsepad_foundry {
        Some(x) => deps.querier.query_wasm_smart(
            x,
            &eclipse_base::splitter::msg::QueryMsg::GovEssence { address_list },
        )?,
        None => deps.querier.query_wasm_smart(
            eclipsepad_staking,
            &eclipse_base::staking::msg::QueryMsg::QueryGovEssenceReduced { address_list },
        )?,
    };

    for (user, user_essence_after) in user_and_essence_list {
        let user_essence_before = USER_ESSENCE.load(deps.storage, &user).unwrap_or_default();
        let delegator_essence_fraction = DELEGATOR_ESSENCE_FRACTIONS
            .load(deps.storage, &user)
            .unwrap_or_default();

        let (delegator_essence_info_before, elector_or_slacker_essence_info_before) =
            calc_splitted_user_essence_info(&user_essence_before, delegator_essence_fraction);
        let (delegator_essence_info_after, elector_or_slacker_essence_info_after) =
            calc_splitted_user_essence_info(&user_essence_after, delegator_essence_fraction);

        // collect rewards
        let (is_updated, user_rewards) = get_accumulated_rewards(deps.storage, &user, block_time)?;
        if is_updated {
            USER_REWARDS.save(deps.storage, &user, &user_rewards)?;
            ELECTOR_WEIGHTS_REF.remove(deps.storage, &user);
        }

        for user_type in get_user_types(deps.storage, &user).unwrap_or(vec![UserType::Slacker]) {
            match user_type {
                UserType::Elector => {
                    let user_weights = get_user_weights(deps.storage, &user, &user_type);
                    let user_essence_allocation_before = calc_essence_allocation(
                        &elector_or_slacker_essence_info_before,
                        &user_weights,
                    );
                    let user_essence_allocation_after = calc_essence_allocation(
                        &elector_or_slacker_essence_info_after,
                        &user_weights,
                    );

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
                        Ok(x.add(&delegator_essence_info_after)
                            .sub(&delegator_essence_info_before))
                    })?;
                }
                UserType::Slacker => {
                    SLACKER_ESSENCE_ACC.update(deps.storage, |x| -> StdResult<EssenceInfo> {
                        Ok(x.add(&elector_or_slacker_essence_info_after)
                            .sub(&elector_or_slacker_essence_info_before))
                    })?;
                }
            };

            // update user essence
            if user_essence_after.is_zero() {
                USER_ESSENCE.remove(deps.storage, &user);
                // rewards must be claimed before decreasing essence to zero
                USER_REWARDS.remove(deps.storage, &user);

                match user_type {
                    UserType::Elector => {
                        ELECTOR_WEIGHTS.remove(deps.storage, &user);
                        ELECTOR_WEIGHTS_REF.remove(deps.storage, &user);
                    }
                    UserType::Delegator => {
                        DELEGATOR_ESSENCE_FRACTIONS.remove(deps.storage, &user);
                    }
                    UserType::Slacker => {}
                }
            } else {
                USER_ESSENCE.save(deps.storage, &user, &user_essence_after)?;
            }
        }
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
        Err(ContractError::WrongToken)?;
    }

    // check if amount isn't zero
    if asset_amount.is_zero() {
        Err(ContractError::ZeroAmount)?;
    }

    // get xastro first
    if token_in == astro {
        RECIPIENT_AND_AMOUNT.save(deps.storage, &(sender_address, Some(asset_amount)))?;

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

    lock_xastro(deps, env, asset_amount, &None, &sender_address)
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

    let (recipient, astro_amount) = &RECIPIENT_AND_AMOUNT.load(deps.storage)?;
    lock_xastro(deps, env, xastro_amount, astro_amount, recipient)
}

fn lock_xastro(
    deps: DepsMut,
    _env: Env,
    xastro_amount: Uint128,
    astro_amount: &Option<Uint128>,
    recipient: &Addr,
) -> Result<Response, ContractError> {
    let AddressConfig {
        astroport_voting_escrow: _,
        eclipsepad_minter,
        ..
    } = ADDRESS_CONFIG.load(deps.storage)?;
    let TokenConfig {
        xastro: _,
        eclip_astro,
        ..
    } = TOKEN_CONFIG.load(deps.storage)?;
    let mut total_convert_info = TOTAL_CONVERT_INFO.load(deps.storage).unwrap_or_default();

    // calculate eclipASTRO amount
    let (astro_supply, xastro_supply) = get_astro_and_xastro_supply(deps.as_ref())?;
    let eclip_astro_amount = astro_amount.unwrap_or(calc_eclip_astro_for_xastro(
        xastro_amount,
        astro_supply,
        xastro_supply,
    ));

    ECLIP_ASTRO_MINTED_BY_VOTER.update(deps.storage, |x| -> StdResult<Uint128> {
        Ok(x + eclip_astro_amount)
    })?;

    total_convert_info.total_xastro += xastro_amount;
    total_convert_info.total_astro_deposited += eclip_astro_amount;
    TOTAL_CONVERT_INFO.save(deps.storage, &total_convert_info)?;

    let msg_list = vec![
        // replenish existent lock or create new one
        // CosmosMsg::Wasm(WasmMsg::Execute {
        //     contract_addr: astroport_voting_escrow.to_string(),
        //     msg: to_json_binary(&astroport_governance::voting_escrow::ExecuteMsg::Lock {
        //         receiver: None,
        //     })?,
        //     funds: coins(xastro_amount.u128(), xastro),
        // }),
        // mint eclipAstro to user
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: eclipsepad_minter.to_string(),
            msg: to_json_binary(&eclipse_base::minter::msg::ExecuteMsg::Mint {
                denom_or_address: eclip_astro,
                amount: eclip_astro_amount,
                recipient: Some(recipient.to_string()),
            })?,
            funds: vec![],
        }),
    ];

    Ok(Response::new()
        .add_messages(msg_list)
        .add_attribute("action", "try_swap_to_eclip_astro")
        .add_attribute("eclip_astro_amount", eclip_astro_amount))
}

pub fn try_swap_to_astro(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let mut response = Response::new();
    // don't allow unlock xastro when votes are in emissions_controller
    check_rewards_claim_stage(deps.storage)?;
    let (sender_address, asset_amount, asset_info) = check_funds(
        deps.as_ref(),
        &info,
        FundsType::Single {
            sender: None,
            amount: None,
        },
    )?;
    let recipient = recipient
        .map(|x| deps.api.addr_validate(&x))
        .transpose()?
        .unwrap_or(sender_address.to_owned());
    let token_in = asset_info.try_get_native()?;
    let AddressConfig {
        eclipsepad_minter,
        worker_list,
        ..
    } = ADDRESS_CONFIG.load(deps.storage)?;
    let TokenConfig {
        xastro,
        eclip_astro,
        ..
    } = TOKEN_CONFIG.load(deps.storage)?;

    if !worker_list.contains(&sender_address) {
        Err(ContractError::Unauthorized)?;
    }

    // check if eclipASTRO or xAstro was sent
    if token_in != eclip_astro && token_in != xastro {
        Err(ContractError::WrongToken)?;
    }

    // calculate xASTRO and ASTRO amounts
    let (astro_supply, xastro_supply) = get_astro_and_xastro_supply(deps.as_ref())?;
    let xastro_amount = if token_in == xastro {
        asset_amount
    } else {
        calc_xastro_for_eclip_astro(asset_amount, astro_supply, xastro_supply)
    };
    let eclip_astro_amount = if token_in == xastro {
        calc_eclip_astro_for_xastro(xastro_amount, astro_supply, xastro_supply)
    } else {
        asset_amount
    };

    // check if amount isn't zero
    if xastro_amount.is_zero() || eclip_astro_amount.is_zero() {
        Err(ContractError::ZeroAmount)?;
    }

    ECLIP_ASTRO_MINTED_BY_VOTER.update(deps.storage, |x| -> StdResult<_> {
        Ok(x - min(eclip_astro_amount, x))
    })?;

    // store xastro_amount for voter-mocks where astroport_voting_escrow isn't available
    RECIPIENT_AND_AMOUNT.save(deps.storage, &(recipient, Some(xastro_amount)))?;

    // burn eclipAstro
    if token_in == eclip_astro {
        response = response.add_message(CosmosMsg::Wasm(wasm_execute(
            eclipsepad_minter,
            &eclipse_base::minter::msg::ExecuteMsg::Burn {},
            coins(eclip_astro_amount.u128(), eclip_astro),
        )?));
    }

    // replaced with useless msg
    // unlock xAstro instantly
    let msg = SubMsg {
        id: UNLOCK_XASTRO_REPLY_ID,
        msg: get_transfer_msg(
            &env.contract.address,
            Uint128::one(),
            &Token::new_native(&xastro),
        )?,
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };

    Ok(response.add_submessage(msg))
}

pub fn handle_unlock_xastro_reply(
    deps: DepsMut,
    _env: Env,
    result: &SubMsgResult,
) -> Result<Response, ContractError> {
    let _res = result
        .to_owned()
        .into_result()
        .map_err(|_| ContractError::StakeError)?;

    // let mut unlocked_xastro = Uint128::zero();
    // for event in res.events.iter() {
    //     for attr in event.attributes.iter() {
    //         if attr.key == "unlocked_amount" {
    //             unlocked_xastro = Uint128::from_str(&attr.value).unwrap();
    //         }
    //     }
    // }

    let (_recipient, unlocked_xastro) = RECIPIENT_AND_AMOUNT.load(deps.storage)?;
    let unlocked_xastro = unlocked_xastro.unwrap_or_default();

    let AddressConfig {
        astroport_staking, ..
    } = ADDRESS_CONFIG.load(deps.storage)?;
    let TokenConfig { xastro, .. } = TOKEN_CONFIG.load(deps.storage)?;

    TOTAL_CONVERT_INFO.update(deps.storage, |mut x| -> StdResult<_> {
        x.total_xastro -= min(unlocked_xastro, x.total_xastro);

        Ok(x)
    })?;

    //  unstake astro
    let msg = SubMsg {
        id: UNSTAKE_ASTRO_REPLY_ID,
        msg: wasm_execute(
            astroport_staking,
            &astroport::staking::ExecuteMsg::Leave {},
            coins(unlocked_xastro.u128(), xastro),
        )?
        .into(),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    };

    Ok(Response::new().add_submessage(msg))
}

pub fn handle_unstake_astro_reply(
    deps: DepsMut,
    _env: Env,
    result: &SubMsgResult,
) -> Result<Response, ContractError> {
    let res = result
        .to_owned()
        .into_result()
        .map_err(|_| ContractError::StakeError)?;

    let mut unstaked_astro = Uint128::zero();
    for event in res.events.iter() {
        for attr in event.attributes.iter() {
            if attr.key == "astro_amount" {
                unstaked_astro = Uint128::from_str(&attr.value).unwrap();
            }
        }
    }

    let TokenConfig { astro, .. } = TOKEN_CONFIG.load(deps.storage)?;
    let (recipient, _) = RECIPIENT_AND_AMOUNT.load(deps.storage)?;

    TOTAL_CONVERT_INFO.update(deps.storage, |mut x| -> StdResult<_> {
        x.total_astro_deposited -= min(unstaked_astro, x.total_astro_deposited);

        Ok(x)
    })?;

    // send astro to user
    let msg = get_transfer_msg(&recipient, unstaked_astro, &Token::new_native(&astro))?;

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "try_swap_to_astro")
        .add_attribute("exchanged_astro", unstaked_astro))
}

pub fn try_update_astro_staking_reward_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    config: AstroStakingRewardConfig,
) -> Result<Response, ContractError> {
    let cfg = ADDRESS_CONFIG.load(deps.storage)?;

    if info.sender != cfg.admin {
        Err(ContractError::Unauthorized)?;
    }
    ensure_eq!(
        config.users + config.treasury,
        10000u32,
        ContractError::InvalidRewardConfig
    );
    ASTRO_STAKING_REWARD_CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update astro staking reward config"))
}

pub fn try_claim_astro_staking_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let AddressConfig {
        eclipsepad_minter,
        eclipse_single_sided_vault,
        ..
    } = ADDRESS_CONFIG.load(deps.storage)?;
    let TokenConfig { eclip_astro, .. } = TOKEN_CONFIG.load(deps.storage)?;
    let mut total_convert_info = TOTAL_CONVERT_INFO.load(deps.storage).unwrap_or_default();
    let mut astro_pending_treasury_reward = ASTRO_PENDING_TREASURY_REWARD
        .load(deps.storage)
        .unwrap_or_default();
    let (rewards, claimable_xastro): (AstroStakingRewardResponse, Uint128) =
        _query_astro_staking_rewards(deps.as_ref(), env)?;
    // must be single_sided_vault
    ensure_eq!(
        Some(info.sender),
        eclipse_single_sided_vault,
        ContractError::Unauthorized
    );
    // must exist users reward
    ensure_ne!(
        rewards.users,
        Uint128::zero(),
        ContractError::NoAstroStakingRewards
    );

    ECLIP_ASTRO_MINTED_BY_VOTER.update(deps.storage, |x| -> StdResult<Uint128> {
        Ok(x + rewards.users)
    })?;

    total_convert_info.claimed_xastro += claimable_xastro;
    astro_pending_treasury_reward += rewards.treasury;
    TOTAL_CONVERT_INFO.save(deps.storage, &total_convert_info)?;
    ASTRO_PENDING_TREASURY_REWARD.save(deps.storage, &astro_pending_treasury_reward)?;

    // mint eclipAstro to user
    let msgs = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: eclipsepad_minter.to_string(),
        msg: to_json_binary(&eclipse_base::minter::msg::ExecuteMsg::Mint {
            denom_or_address: eclip_astro,
            amount: rewards.users,
            recipient: Some(eclipse_single_sided_vault.unwrap().to_string()),
        })?,
        funds: vec![],
    })];

    Ok(Response::new().add_messages(msgs))
}

pub fn try_claim_astro_staking_treasury_rewards(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = ADDRESS_CONFIG.load(deps.storage)?;
    let AddressConfig {
        eclipsepad_minter,
        eclipse_dao,
        ..
    } = ADDRESS_CONFIG.load(deps.storage)?;
    let TokenConfig { eclip_astro, .. } = TOKEN_CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        Err(ContractError::Unauthorized)?;
    }

    let astro_pending_treasury_reward = ASTRO_PENDING_TREASURY_REWARD
        .load(deps.storage)
        .unwrap_or_default();
    ensure_ne!(
        astro_pending_treasury_reward,
        Uint128::zero(),
        ContractError::NoAstroStakingRewards
    );
    ECLIP_ASTRO_MINTED_BY_VOTER.update(deps.storage, |x| -> StdResult<Uint128> {
        Ok(x + astro_pending_treasury_reward)
    })?;
    ASTRO_PENDING_TREASURY_REWARD.save(deps.storage, &Uint128::zero())?;

    // mint eclipAstro to user
    let msgs = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: eclipsepad_minter.to_string(),
        msg: to_json_binary(&eclipse_base::minter::msg::ExecuteMsg::Mint {
            denom_or_address: eclip_astro,
            amount: astro_pending_treasury_reward,
            recipient: Some(eclipse_dao.to_string()),
        })?,
        funds: vec![],
    })];

    Ok(Response::new().add_messages(msgs))
}

pub fn try_set_delegation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    weight: Decimal,
) -> Result<Response, ContractError> {
    check_pause_state(deps.storage)?;
    check_rewards_claim_stage(deps.storage)?;
    let block_time = env.block.time.seconds();
    let user = &info.sender;
    let user_types = get_user_types(deps.storage, user)?;
    let essence_info = USER_ESSENCE.load(deps.storage, user).unwrap_or_default();
    let delegator_essence_fraction = DELEGATOR_ESSENCE_FRACTIONS
        .load(deps.storage, user)
        .unwrap_or_default();

    // collect rewards
    let (is_updated, user_rewards) = get_accumulated_rewards(deps.storage, user, block_time)?;
    if is_updated {
        USER_REWARDS.save(deps.storage, user, &user_rewards)?;
        ELECTOR_WEIGHTS_REF.remove(deps.storage, user);
    }

    // check if weight isn't out of range
    if weight > Decimal::one() {
        Err(ContractError::WeightIsOutOfRange)?;
    }

    // user can't undelegate if he wasn't delegator before
    if weight.is_zero() && delegator_essence_fraction.is_zero() {
        Err(ContractError::DelegatorIsNotFound)?;
    }

    // don't allow useless txs
    if weight == delegator_essence_fraction {
        Err(ContractError::DelegateTwice)?;
    }

    let (delegator_essence_info_before, elector_or_slacker_essence_info_before) =
        calc_splitted_user_essence_info(&essence_info, delegator_essence_fraction);
    let (delegator_essence_info_after, elector_or_slacker_essence_info_after) =
        calc_splitted_user_essence_info(&essence_info, weight);

    for user_type in user_types {
        match user_type {
            UserType::Elector => {
                let user_weights = get_user_weights(deps.storage, user, &user_type);
                let user_essence_allocation_before =
                    calc_essence_allocation(&elector_or_slacker_essence_info_before, &user_weights);
                let user_essence_allocation_after =
                    calc_essence_allocation(&elector_or_slacker_essence_info_after, &user_weights);

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
            _ => {
                SLACKER_ESSENCE_ACC.update(deps.storage, |x| -> StdResult<EssenceInfo> {
                    Ok(x.add(&elector_or_slacker_essence_info_after)
                        .sub(&elector_or_slacker_essence_info_before))
                })?;
            }
        };
    }

    DAO_ESSENCE_ACC.update(deps.storage, |x| -> StdResult<EssenceInfo> {
        Ok(x.add(&delegator_essence_info_after)
            .sub(&delegator_essence_info_before))
    })?;

    if weight.is_zero() {
        DELEGATOR_ESSENCE_FRACTIONS.remove(deps.storage, user);
    } else {
        DELEGATOR_ESSENCE_FRACTIONS.save(deps.storage, user, &weight)?;
    }

    Ok(Response::new().add_attribute("action", "try_set_delegation"))
}

pub fn try_place_vote(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    weight_allocation: Vec<WeightAllocationItem>,
) -> Result<Response, ContractError> {
    check_pause_state(deps.storage)?;
    check_rewards_claim_stage(deps.storage)?;
    verify_weight_allocation(deps.as_ref(), &weight_allocation)?;
    let block_time = env.block.time.seconds();
    let user = &info.sender;
    let user_types = get_user_types(deps.storage, user)?;
    let (_delegator_essence_info, elector_or_slacker_essence_info) =
        split_user_essence_info(deps.storage, user);

    // collect rewards
    let (is_updated, user_rewards) = get_accumulated_rewards(deps.storage, user, block_time)?;
    if is_updated {
        USER_REWARDS.save(deps.storage, user, &user_rewards)?;
        ELECTOR_WEIGHTS_REF.remove(deps.storage, user);
    }

    let user_type = user_types
        .iter()
        .find(|x| !matches!(x, UserType::Delegator))
        .ok_or(ContractError::DelegatorCanNotVote)?;

    if let UserType::Slacker = user_type {
        SLACKER_ESSENCE_ACC.update(deps.storage, |x| -> StdResult<EssenceInfo> {
            Ok(x.sub(&elector_or_slacker_essence_info))
        })?;
    }

    // update elector
    let user_weights_before = ELECTOR_WEIGHTS.load(deps.storage, user).unwrap_or_default();
    let user_essence_allocation_before =
        calc_essence_allocation(&elector_or_slacker_essence_info, &user_weights_before);
    let user_essence_allocation_after =
        calc_essence_allocation(&elector_or_slacker_essence_info, &weight_allocation);

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
    _env: Env,
    info: MessageInfo,
    weight_allocation: Vec<WeightAllocationItem>,
) -> Result<Response, ContractError> {
    check_pause_state(deps.storage)?;
    check_rewards_claim_stage(deps.storage)?;
    verify_weight_allocation(deps.as_ref(), &weight_allocation)?;
    let sender = &info.sender;
    let AddressConfig { eclipse_dao, .. } = ADDRESS_CONFIG.load(deps.storage)?;

    if sender != eclipse_dao {
        Err(ContractError::Unauthorized)?;
    }

    DAO_WEIGHTS_ACC.save(deps.storage, &weight_allocation)?;

    Ok(Response::new().add_attribute("action", "try_place_vote_as_dao"))
}

pub fn try_vote(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let block_time = env.block.time.seconds();
    let rewards_claim_stage = REWARDS_CLAIM_STAGE.load(deps.storage)?;
    // let AddressConfig {
    //     astroport_emission_controller,
    //     ..
    // } = ADDRESS_CONFIG.load(deps.storage)?;
    let DateConfig {
        epoch_length,
        vote_delay,
        ..
    } = DATE_CONFIG.load(deps.storage)?;
    let mut current_epoch = EPOCH_COUNTER.load(deps.storage)?;

    // final voting must be executed single time right before epoch end
    if block_time < current_epoch.start_date + vote_delay {
        Err(ContractError::VotingDelay)?;
    }

    // only swapped -> unclaimed transition is allowed
    if !matches!(rewards_claim_stage, RewardsClaimStage::Swapped) {
        Err(ContractError::WrongRewardsClaimStage)?;
    }
    REWARDS_CLAIM_STAGE.save(deps.storage, &RewardsClaimStage::Unclaimed)?;

    let elector_essence_acc_before = ELECTOR_ESSENCE_ACC.load(deps.storage)?;
    let elector_weights_acc_before = ELECTOR_WEIGHTS_ACC.load(deps.storage)?;
    let dao_essence_acc_before = DAO_ESSENCE_ACC.load(deps.storage)?;
    let dao_weights_acc_before = DAO_WEIGHTS_ACC.load(deps.storage)?;
    let slacker_essence = SLACKER_ESSENCE_ACC.load(deps.storage)?;
    let elector_base_essence_fraction = str_to_dec(ELECTOR_BASE_ESSENCE_FRACTION);
    let elector_additional_essence_fraction = str_to_dec(ELECTOR_ADDITIONAL_ESSENCE_FRACTION);
    let total_weights_allocation = get_total_votes(deps.storage, block_time)?.weight;

    // update vote results
    VOTE_RESULTS.update(deps.storage, |mut x| -> StdResult<Vec<VoteResults>> {
        x.push(VoteResults {
            epoch_id: current_epoch.id,
            end_date: current_epoch.start_date + epoch_length,

            elector_essence: elector_essence_acc_before
                .scale(elector_base_essence_fraction)
                .add(&slacker_essence.scale(elector_additional_essence_fraction))
                .capture(block_time),
            dao_essence: dao_essence_acc_before
                .add(&slacker_essence.scale(Decimal::one() - elector_additional_essence_fraction))
                .add(
                    &elector_essence_acc_before
                        .scale(Decimal::one() - elector_base_essence_fraction),
                )
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

    // // send vote msg
    // let msg = CosmosMsg::Wasm(WasmMsg::Execute {
    //     contract_addr: astroport_emission_controller.to_string(),
    //     msg: to_json_binary(
    //         &astroport_governance::emissions_controller::msg::ExecuteMsg::<Empty>::Vote {
    //             votes: total_weights_allocation,
    //         },
    //     )?,
    //     funds: vec![],
    // });

    Ok(Response::new()
        // .add_message(msg)
        .add_attribute("action", "try_vote"))
}

pub fn try_claim(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let sender = &env.contract.address;
    let _epoch = EPOCH_COUNTER.load(deps.storage)?;
    let rewards_claim_stage = REWARDS_CLAIM_STAGE.load(deps.storage)?;
    let AddressConfig {
        astroport_tribute_market,
        eclipsepad_tribute_market,
        astroport_emission_controller: _,
        astroport_voting_escrow: _,
        ..
    } = ADDRESS_CONFIG.load(deps.storage)?;
    let astroport_tribute_market =
        &unwrap_field(astroport_tribute_market, "astroport_tribute_market")?;

    // only unclaimed -> claimed transition is allowed
    if !matches!(rewards_claim_stage, RewardsClaimStage::Unclaimed) {
        Err(ContractError::WrongRewardsClaimStage)?;
    }
    REWARDS_CLAIM_STAGE.save(deps.storage, &RewardsClaimStage::Claimed)?;

    // check rewards
    let astroport_rewards = query_astroport_rewards(deps.as_ref(), sender)?;
    let eclipsepad_rewards = query_eclipsepad_rewards(deps.as_ref(), sender)?;

    if astroport_rewards.is_empty() && eclipsepad_rewards.is_empty() {
        Err(ContractError::RewardsAreNotFound)?;
    }

    // // get voter bribes allocation:
    // // 1) query tribute market bribes allocation
    // let astroport_bribe_allocation = query_astroport_bribe_allocation(deps.as_ref())?;
    // let eclipsepad_bribe_allocation = query_eclipsepad_bribe_allocation(deps.as_ref())?;

    // // 2) query voter voting power
    // let voter_voting_power = deps.querier.query_wasm_smart::<Uint128>(
    //     astroport_voting_escrow,
    //     &astroport_governance::voting_escrow::QueryMsg::UserVotingPower {
    //         user: env.contract.address.to_string(),
    //         timestamp: Some(epoch.start_date),
    //     },
    // )?;
    // let voter_voting_power_decimal = u128_to_dec(voter_voting_power);

    // // 3) get voter to tribute market voting power ratio allocation
    // let voter_to_tribute_voting_power_ratio_allocation = deps
    //     .querier
    //     .query_wasm_smart::<UserInfoResponse>(
    //         astroport_emission_controller.clone(),
    //         &astroport_governance::emissions_controller::hub::QueryMsg::UserInfo {
    //             user: env.contract.address.to_string(),
    //             timestamp: Some(epoch.start_date),
    //         },
    //     )?
    //     .applied_votes
    //     .iter()
    //     .map(|(lp_token, weight)| -> StdResult<(String, Decimal)> {
    //         let tribute_market_voting_power = deps
    //             .querier
    //             .query_wasm_smart::<VotedPoolInfo>(
    //                 astroport_emission_controller.clone(),
    //                 &astroport_governance::emissions_controller::hub::QueryMsg::VotedPool {
    //                     pool: lp_token.to_owned(),
    //                     timestamp: Some(epoch.start_date),
    //                 },
    //             )?
    //             .voting_power;

    //         let ratio = calc_voter_to_tribute_voting_power_ratio(
    //             weight,
    //             voter_voting_power_decimal,
    //             tribute_market_voting_power,
    //         );

    //         Ok((lp_token.to_owned(), ratio))
    //     })
    //     .collect::<StdResult<Vec<(String, Decimal)>>>()?;

    // // 4) update vote results
    // let mut vote_results = VOTE_RESULTS.load(deps.storage)?;

    // // compare pools from vote results and applied votes
    // let last_vote_results = &vote_results
    //     .iter()
    //     .last()
    //     .ok_or(ContractError::LastVoteResultsAreNotFound)?
    //     .pool_info_list;

    // let applied_votes_pool_list: Vec<String> = voter_to_tribute_voting_power_ratio_allocation
    //     .iter()
    //     .map(|(lp_token, _ratio)| lp_token.to_owned())
    //     .collect();

    // if !(last_vote_results.len() == applied_votes_pool_list.len()
    //     && last_vote_results
    //         .iter()
    //         .all(|x| applied_votes_pool_list.contains(&x.lp_token)))
    // {
    //     Err(ContractError::UnequalPools)?;
    // }

    // vote_results = vote_results
    //     .into_iter()
    //     .map(|mut x| {
    //         if x.epoch_id + 1 == epoch.id {
    //             let astroport_pool_info_list_with_rewards = calc_pool_info_list_with_rewards(
    //                 &x.pool_info_list,
    //                 &astroport_bribe_allocation,
    //                 &voter_to_tribute_voting_power_ratio_allocation,
    //             );

    //             let eclipsepad_pool_info_list_with_rewards = calc_pool_info_list_with_rewards(
    //                 &x.pool_info_list,
    //                 &eclipsepad_bribe_allocation,
    //                 &voter_to_tribute_voting_power_ratio_allocation,
    //             );

    //             x.pool_info_list = calc_merged_pool_info_list_with_rewards(
    //                 &astroport_pool_info_list_with_rewards,
    //                 &eclipsepad_pool_info_list_with_rewards,
    //             );
    //         }

    //         x
    //     })
    //     .collect();
    // VOTE_RESULTS.save(deps.storage, &vote_results)?;

    // claim rewards
    let mut msg_list: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: astroport_tribute_market.to_string(),
        msg: to_json_binary(&eclipse_base::tribute_market::msg::ExecuteMsg::ClaimRewards {})?,
        funds: vec![],
    })];

    if !eclipsepad_rewards.is_empty() {
        msg_list.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: eclipsepad_tribute_market.unwrap().to_string(),
            msg: to_json_binary(&eclipse_base::tribute_market::msg::ExecuteMsg::ClaimRewards {})?,
            funds: vec![],
        }));
    }

    Ok(Response::new()
        .add_messages(msg_list)
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
        .map_err(|_| ContractError::SubMsgResultError)?;

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

    // allow any essence allocation updates as bribes collection is completed
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
    check_pause_state(deps.storage)?;
    let user = &info.sender;
    let block_time = env.block.time.seconds();

    // collect rewards
    let (_is_updated, mut user_rewards) = get_accumulated_rewards(deps.storage, user, block_time)?;
    if user_rewards.value.is_empty() {
        Err(ContractError::RewardsAreNotFound)?;
    }

    let balance_list = deps.querier.query_all_balances(env.contract.address)?;

    // send rewards to user
    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: user.to_string(),
        amount: user_rewards
            .value
            .into_iter()
            .map(|(amount, denom)| {
                let balance = balance_list
                    .iter()
                    .find(|x| x.denom == denom)
                    .map(|x| x.amount)
                    .unwrap_or_default();

                coin(std::cmp::min(amount, balance).u128(), denom)
            })
            .filter(|x| !x.amount.is_zero())
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

pub fn try_unlock_xastro(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    check_pause_state(deps.storage)?;
    // don't allow unlock xastro when votes are in emissions_controller
    check_rewards_claim_stage(deps.storage)?;
    let (sender_address, ..) = check_funds(deps.as_ref(), &info, FundsType::Empty)?;
    let AddressConfig {
        astroport_voting_escrow: _,
        worker_list,
        ..
    } = ADDRESS_CONFIG.load(deps.storage)?;
    let TokenConfig { xastro, .. } = TOKEN_CONFIG.load(deps.storage)?;

    if !worker_list.contains(&sender_address) {
        Err(ContractError::Unauthorized)?;
    }

    if amount.is_zero() {
        Err(ContractError::ZeroAmount)?;
    }

    let max_xastro_amount = query_voter_xastro(deps.as_ref(), env)?;
    if amount > max_xastro_amount {
        Err(ContractError::ExceededMaxAmount)?;
    }

    let (astro_supply, xastro_supply) = get_astro_and_xastro_supply(deps.as_ref())?;
    let astro_amount = calc_eclip_astro_for_xastro(amount, astro_supply, xastro_supply);
    TOTAL_CONVERT_INFO.update(deps.storage, |mut x| -> StdResult<ConvertInfo> {
        x.total_xastro -= amount;
        x.total_astro_deposited -= astro_amount;
        Ok(x)
    })?;

    let msg_list = vec![
        // unlock part of xAstro instantly
        // CosmosMsg::Wasm(WasmMsg::Execute {
        //     contract_addr: astroport_voting_escrow.to_string(),
        //     msg: to_json_binary(
        //         &astroport_governance::voting_escrow::ExecuteMsg::InstantUnlock { amount },
        //     )?,
        //     funds: vec![],
        // }),
        // send xAstro to recipient
        CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient
                .map(|x| deps.api.addr_validate(&x))
                .transpose()?
                .unwrap_or(sender_address)
                .to_string(),
            amount: coins(amount.u128(), xastro),
        }),
    ];

    Ok(Response::new()
        .add_messages(msg_list)
        .add_attribute("action", "try_unlock_xastro"))
}
