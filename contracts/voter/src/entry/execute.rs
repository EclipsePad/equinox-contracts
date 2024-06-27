use std::str::FromStr;

use cosmwasm_std::{
    coins, to_json_binary, Addr, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, ReplyOn, Response,
    SubMsg, SubMsgResult, Uint128, WasmMsg,
};

use eclipse_base::{
    assets::TokenUnverified,
    converters::u128_to_dec,
    utils::{check_funds, unwrap_field, FundsType},
};
use equinox_msg::voter::{AddressConfig, DateConfig, EssenceInfo, TokenConfig, VotingListItem};

use crate::{
    error::ContractError,
    state::{
        ADDRESS_CONFIG, DATE_CONFIG, LOCKING_ESSENCE, RECIPIENT, STAKE_ASTRO_REPLY_ID,
        STAKING_ESSENCE_COMPONENTS, TOKEN_CONFIG, TOTAL_LOCKING_ESSENCE,
        TOTAL_STAKING_ESSENCE_COMPONENTS,
    },
};

pub fn try_update_date_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    epochs_start: Option<u64>,
    epoch_length: Option<u64>,
    vote_cooldown: Option<u64>,
) -> Result<Response, ContractError> {
    let AddressConfig { admin, .. } = ADDRESS_CONFIG.load(deps.storage)?;
    let mut config = DATE_CONFIG.load(deps.storage)?;

    if info.sender != admin {
        Err(ContractError::Unauthorized {})?;
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

// /// Update config
// pub fn update_config(
//     deps: DepsMut,
//     _env: Env,
//     info: MessageInfo,
//     new_config: UpdateConfig,
// ) -> Result<Response, ContractError> {
//     OWNER.assert_admin(deps.as_ref(), &info.sender)?;
//     let mut config = CONFIG.load(deps.storage)?;
//     let mut res: Response = Response::new().add_attribute("action", "update config");
//     if let Some(astro) = new_config.astro {
//         res = res.add_attribute("astro", &astro);
//         config.astro = astro;
//     }
//     if let Some(xastro) = new_config.xastro {
//         res = res.add_attribute("xastro", &xastro);
//         config.xastro = xastro;
//     }
//     if let Some(vxastro) = new_config.vxastro {
//         res = res.add_attribute("vxastro", &vxastro);
//         config.vxastro = vxastro;
//     }
//     if let Some(staking_contract) = new_config.staking_contract {
//         config.staking_contract = deps.api.addr_validate(&staking_contract)?;
//         res = res.add_attribute("staking_contract", staking_contract);
//     }
//     if let Some(converter_contract) = new_config.converter_contract {
//         config.converter_contract = deps.api.addr_validate(&converter_contract)?;
//         res = res.add_attribute("converter_contract", converter_contract);
//     }
//     if let Some(gauge_contract) = new_config.gauge_contract {
//         config.gauge_contract = deps.api.addr_validate(&gauge_contract)?;
//         res = res.add_attribute("gauge_contract", gauge_contract);
//     }
//     if let Some(astroport_gauge_contract) = new_config.astroport_gauge_contract {
//         config.astroport_gauge_contract = deps.api.addr_validate(&astroport_gauge_contract)?;
//         res = res.add_attribute("astroport_gauge_contract", astroport_gauge_contract);
//     }
//     if let Some(astroport_voting_escrow_contract) = new_config.astroport_voting_escrow_contract {
//         config.astroport_voting_escrow_contract =
//             deps.api.addr_validate(&astroport_voting_escrow_contract)?;
//         res = res.add_attribute(
//             "astroport_voting_escrow_contract",
//             astroport_voting_escrow_contract,
//         );
//     }
//     if let Some(astroport_generator_controller) = new_config.astroport_generator_controller {
//         config.astroport_generator_controller =
//             deps.api.addr_validate(&astroport_generator_controller)?;
//         res = res.add_attribute(
//             "astroport_generator_controller",
//             astroport_generator_controller,
//         );
//     }
//     if let Some(eclipsepad_staking_contract) = new_config.eclipsepad_staking_contract {
//         config.eclipsepad_staking_contract =
//             deps.api.addr_validate(&eclipsepad_staking_contract)?;
//         res = res.add_attribute("eclipsepad_staking_contract", eclipsepad_staking_contract);
//     }
//     CONFIG.save(deps.storage, &config)?;
//     Ok(res)
// }

// /// Update owner
// pub fn update_owner(
//     mut deps: DepsMut,
//     _env: Env,
//     info: MessageInfo,
//     new_owner: String,
// ) -> Result<Response, ContractError> {
//     OWNER.assert_admin(deps.as_ref(), &info.sender)?;
//     let new_owner_addr = deps.api.addr_validate(&new_owner)?;
//     OWNER.set(deps.branch(), Some(new_owner_addr))?;
//     Ok(Response::new()
//         .add_attribute("action", "update owner")
//         .add_attribute("to", new_owner))
// }

// /// Withdraw bribe rewards
// pub fn withdraw_bribe_rewards(
//     _deps: DepsMut,
//     _env: Env,
//     _info: MessageInfo,
// ) -> Result<Response, ContractError> {
//     // to do
//     Ok(Response::new())
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
        Err(ContractError::ZeroAmount {})?;
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
        .map_err(|_| ContractError::StakeError {})?;

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
//     // 100 % = 10_000 BP
//     const BP_MULTIPLIER: u128 = 10_000;

//     OWNER.assert_admin(deps.as_ref(), &info.sender)?;
//     let Config {
//         astroport_generator_controller,
//         ..
//     } = CONFIG.load(deps.storage)?;

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
//     let votes: Vec<(String, u16)> = voting_list
//         .into_iter()
//         .map(|x| {
//             (
//                 x.lp_token,
//                 (x.voting_power * u128_to_dec(BP_MULTIPLIER))
//                     .to_uint_floor()
//                     .u128() as u16,
//             )
//         })
//         .collect();

//     if (votes
//         .iter()
//         .fold(0, |acc, (_, voting_power)| acc + voting_power)) as u128
//         != BP_MULTIPLIER
//     {
//         Err(ContractError::WeightsAreUnbalanced)?;
//     }

//     // // send vote msg
//     // let msg = CosmosMsg::Wasm(WasmMsg::Execute {
//     //     contract_addr: astroport_generator_controller.to_string(),
//     //     msg: to_json_binary(
//     //         &astroport_governance::generator_controller::ExecuteMsg::Vote { votes },
//     //     )?,
//     //     funds: vec![],
//     // });

//     Ok(Response::new()
//         //     .add_message(msg)
//         .add_attribute("action", "try_vote"))
// }

pub fn try_capture_essence(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    user_and_essence_list: Vec<(String, EssenceInfo)>,
    total_essence: EssenceInfo,
) -> Result<Response, ContractError> {
    let sender = &info.sender;
    // let block_time = env.block.time.seconds();
    let AddressConfig {
        admin,
        eclipsepad_staking,
        ..
    } = ADDRESS_CONFIG.load(deps.storage)?;
    // let DateConfig {
    //     epochs_start,
    //     epoch_length,
    //     ..
    // } = DATE_CONFIG.load(deps.storage)?;

    if sender != eclipsepad_staking && sender.to_string() != admin {
        Err(ContractError::Unauthorized {})?;
    }

    // if block_time > epochs_start + epoch_length {
    TOTAL_STAKING_ESSENCE_COMPONENTS.save(deps.storage, &total_essence.staking_components)?;
    TOTAL_LOCKING_ESSENCE.save(deps.storage, &total_essence.locking_amount)?;

    for (user_address, user_essence) in user_and_essence_list {
        let user = &Addr::unchecked(user_address);
        STAKING_ESSENCE_COMPONENTS.save(deps.storage, user, &user_essence.staking_components)?;
        LOCKING_ESSENCE.save(deps.storage, user, &user_essence.locking_amount)?;
    }
    // }

    Ok(Response::new().add_attribute("action", "try_capture_essence"))
}
