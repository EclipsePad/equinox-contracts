use std::str::FromStr;

use cosmwasm_std::{
    coins, to_json_binary, Addr, CosmosMsg, Decimal, DepsMut, Empty, Env, MessageInfo, ReplyOn,
    Response, StdResult, SubMsg, SubMsgResult, Uint128, WasmMsg,
};

use eclipse_base::{
    assets::TokenUnverified,
    converters::u128_to_dec,
    utils::{check_funds, unwrap_field, FundsType},
};
use equinox_msg::voter::{AddressConfig, DateConfig, EssenceInfo, TokenConfig, TransferAdminState};

use crate::{
    error::ContractError,
    state::{
        ADDRESS_CONFIG, DATE_CONFIG, RECIPIENT, STAKE_ASTRO_REPLY_ID, TOKEN_CONFIG,
        TRANSFER_ADMIN_STATE, TRANSFER_ADMIN_TIMEOUT,
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
    eclipse_dao: String,
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
    epochs_start: Option<u64>,
    epoch_length: Option<u64>,
    vote_cooldown: Option<u64>,
    vote_delay: Option<u64>,
) -> Result<Response, ContractError> {
    let AddressConfig { admin, .. } = ADDRESS_CONFIG.load(deps.storage)?;
    let mut config = DATE_CONFIG.load(deps.storage)?;

    if info.sender != admin {
        Err(ContractError::Unauthorized)?;
    }

    if let Some(x) = epochs_start {
        config.epochs_start = x;
    }

    if let Some(x) = epoch_length {
        config.epoch_length = x;
    }

    if let Some(x) = vote_cooldown {
        config.vote_cooldown = x;
    }

    DATE_CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "try_update_date_config"))
}

// pub fn try_capture_essence(
//     deps: DepsMut,
//     _env: Env,
//     info: MessageInfo,
//     user_and_essence_list: Vec<(String, EssenceInfo)>,
//     total_essence: EssenceInfo,
// ) -> Result<Response, ContractError> {
//     let sender = &info.sender;
//     // let block_time = env.block.time.seconds();
//     let AddressConfig {
//         admin,
//         eclipsepad_staking,
//         ..
//     } = ADDRESS_CONFIG.load(deps.storage)?;
//     // let DateConfig {
//     //     epochs_start,
//     //     epoch_length,
//     //     ..
//     // } = DATE_CONFIG.load(deps.storage)?;

//     if sender != eclipsepad_staking && sender.to_string() != admin {
//         Err(ContractError::Unauthorized)?;
//     }

//     // if block_time > epochs_start + epoch_length {
//     TOTAL_STAKING_ESSENCE_COMPONENTS.save(deps.storage, &total_essence.staking_components)?;
//     TOTAL_LOCKING_ESSENCE.save(deps.storage, &total_essence.locking_amount)?;

//     for (user_address, user_essence) in user_and_essence_list {
//         let user = &Addr::unchecked(user_address);
//         STAKING_ESSENCE_COMPONENTS.save(deps.storage, user, &user_essence.staking_components)?;
//         LOCKING_ESSENCE.save(deps.storage, user, &user_essence.locking_amount)?;
//     }
//     // }

//     Ok(Response::new().add_attribute("action", "try_capture_essence"))
// }

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

// pub fn try_vote(
//     deps: DepsMut,
//     _env: Env,
//     info: MessageInfo,
//     voting_list: Vec<VotingListItem>,
// ) -> Result<Response, ContractError> {
//     let AddressConfig {
//         admin,
//         astroport_emission_controller,
//         ..
//     } = ADDRESS_CONFIG.load(deps.storage)?;

//     if info.sender != admin {
//         Err(ContractError::Unauthorized)?;
//     }

//     // check voting list

//     // empty
//     if voting_list.is_empty() {
//         Err(ContractError::EmptyVotingList)?;
//     }

//     // diplications
//     let mut pool_list: Vec<String> = voting_list.iter().map(|x| x.lp_token.to_string()).collect();
//     pool_list.sort_unstable();
//     pool_list.dedup();

//     if pool_list.len() != voting_list.len() {
//         Err(ContractError::VotingListDuplication)?;
//     }

//     // out of range
//     if voting_list
//         .iter()
//         .any(|x| x.voting_power.is_zero() || x.voting_power > Decimal::one())
//     {
//         Err(ContractError::WeightIsOutOfRange)?;
//     }

//     // wrong sum
//     let votes: Vec<(String, Decimal)> = voting_list
//         .into_iter()
//         .map(|x| (x.lp_token, x.voting_power))
//         .collect();

//     if (votes
//         .iter()
//         .fold(Decimal::zero(), |acc, (_, voting_power)| acc + voting_power))
//         != Decimal::one()
//     {
//         Err(ContractError::WeightsAreUnbalanced)?;
//     }

//     // send vote msg
//     let msg = CosmosMsg::Wasm(WasmMsg::Execute {
//         contract_addr: astroport_emission_controller.to_string(),
//         msg: to_json_binary(
//             &astroport_governance::emissions_controller::msg::ExecuteMsg::<Empty>::Vote { votes },
//         )?,
//         funds: vec![],
//     });

//     Ok(Response::new()
//         .add_message(msg)
//         .add_attribute("action", "try_vote"))
// }
