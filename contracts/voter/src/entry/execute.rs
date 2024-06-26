use std::str::FromStr;

use cosmwasm_std::{
    coins, to_json_binary, Addr, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, ReplyOn, Response,
    SubMsg, SubMsgResult, Uint128, WasmMsg,
};

use eclipse_base::{
    converters::u128_to_dec,
    utils::{check_funds, unwrap_field, FundsType},
};
use equinox_msg::voter::{AddressConfig, VotingListItem};

use crate::{
    error::ContractError,
    state::{ADDRESS_CONFIG, DATE_CONFIG},
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

// pub fn try_swap_to_eclip_astro(
//     deps: DepsMut,
//     env: Env,
//     info: MessageInfo,
// ) -> Result<Response, ContractError> {
//     let (sender_address, asset_amount, asset_info) = check_funds(
//         deps.as_ref(),
//         &info,
//         FundsType::Single {
//             sender: None,
//             amount: None,
//         },
//     )?;
//     let token_in = asset_info.try_get_native()?;
//     let Config {
//         astro,
//         xastro,
//         staking_contract,
//         ..
//     } = CONFIG.load(deps.storage)?;

//     // check if ASTRO or xASTRO was sent
//     if token_in != astro && token_in != xastro {
//         Err(ContractError::UnknownToken(token_in.to_string()))?;
//     }

//     // check if amount isn't zero
//     if asset_amount.is_zero() {
//         Err(ContractError::ZeroAmount {})?;
//     }

//     // get xastro first
//     if token_in == astro {
//         RECIPIENT.save(deps.storage, &sender_address)?;

//         let msg = SubMsg {
//             id: STAKE_ASTRO_REPLY_ID,
//             msg: WasmMsg::Execute {
//                 contract_addr: staking_contract.to_string(),
//                 msg: to_json_binary(&astroport::staking::ExecuteMsg::Enter { receiver: None })?,
//                 funds: coins(asset_amount.u128(), astro),
//             }
//             .into(),
//             gas_limit: None,
//             reply_on: ReplyOn::Success,
//         };

//         return Ok(Response::new().add_submessage(msg));
//     }

//     lock_xastro(deps, env, asset_amount, &sender_address)
// }

// pub fn handle_stake_astro_reply(
//     deps: DepsMut,
//     env: Env,
//     result: &SubMsgResult,
// ) -> Result<Response, ContractError> {
//     let res = result
//         .to_owned()
//         .into_result()
//         .map_err(|_| ContractError::StakeError {})?;

//     let mut xastro_amount = Uint128::zero();
//     for event in res.events.iter() {
//         for attr in event.attributes.iter() {
//             if attr.key == "xastro_amount" {
//                 xastro_amount = Uint128::from_str(&attr.value).unwrap();
//             }
//         }
//     }

//     let recipient = &RECIPIENT.load(deps.storage)?;
//     lock_xastro(deps, env, xastro_amount, recipient)
// }

// fn lock_xastro(
//     deps: DepsMut,
//     env: Env,
//     xastro_amount: Uint128,
//     recipient: &Addr,
// ) -> Result<Response, ContractError> {
//     let Config {
//         xastro,
//         astroport_voting_escrow_contract,
//         converter_contract,
//         staking_contract,
//         ..
//     } = CONFIG.load(deps.storage)?;

//     // calculate eclipASTRO amount
//     let total_xastro_amount: Uint128 = deps.querier.query_wasm_smart(
//         staking_contract.to_string(),
//         &astroport::staking::QueryMsg::TotalShares {},
//     )?;
//     let total_astro_amount: Uint128 = deps.querier.query_wasm_smart(
//         staking_contract.to_string(),
//         &astroport::staking::QueryMsg::TotalDeposit {},
//     )?;
//     let eclip_astro_amount = total_astro_amount * xastro_amount / total_xastro_amount;

//     // check lock position
//     let lock_info = deps
//         .querier
//         .query_wasm_smart::<astroport_governance::voting_escrow::LockInfoResponse>(
//             astroport_voting_escrow_contract.to_string(),
//             &astroport_governance::voting_escrow::QueryMsg::LockInfo {
//                 user: env.contract.address.to_string(),
//             },
//         );

//     // let hook_msg = match lock_info {
//     //     Ok(_) => {
//     //         to_json_binary(&astroport_governance::voting_escrow::Cw20HookMsg::ExtendLockAmount {})
//     //     }
//     //     Err(_) => to_json_binary(
//     //         &astroport_governance::voting_escrow::Cw20HookMsg::CreateLock {
//     //             time: MAX_ESCROW_VOTING_LOCK_PERIOD,
//     //         },
//     //     ),
//     // };

//     let msg_list = vec![
//         // replenish existent lock or create new one for 2 years
//         // CosmosMsg::Wasm(WasmMsg::Execute {
//         //     contract_addr: xastro.to_string(),
//         //     msg: to_json_binary(&Cw20ExecuteMsg::Send {
//         //         contract: astroport_voting_escrow_contract.to_string(),
//         //         amount: xastro_amount,
//         //         msg: hook_msg?,
//         //     })?,
//         //     funds: vec![],
//         // }),
//         // mint eclipAstro to user
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             contract_addr: converter_contract.to_string(),
//             msg: to_json_binary(&equinox_msg::token_converter::ExecuteMsg::MintEclipAstro {
//                 amount: eclip_astro_amount,
//                 recipient: recipient.to_string(),
//             })?,
//             funds: vec![],
//         }),
//     ];

//     Ok(Response::new()
//         .add_messages(msg_list)
//         .add_attribute("action", "try_swap_to_eclip_astro")
//         .add_attribute("eclip_astro_amount", eclip_astro_amount))
// }

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

// pub fn try_capture_essence(
//     deps: DepsMut,
//     env: Env,
//     info: MessageInfo,
//     user_and_essence_list: Vec<(String, Uint128)>,
//     total_essence: Uint128,
// ) -> Result<Response, ContractError> {
//     let sender = &info.sender;
//     let admin = unwrap_field(OWNER.query_admin(deps.as_ref())?.admin, "admin")?;
//     let block_time = env.block.time.seconds();
//     let Config {
//         eclipsepad_staking_contract,
//         ..
//     } = CONFIG.load(deps.storage)?;

//     if sender != eclipsepad_staking_contract && sender.to_string() != admin {
//         Err(ContractError::Unauthorized {})?;
//     }

//     // TODO: query gauge voting start date
//     let gauge_voting_start_date = 10000000000000000u64;

//     if block_time > gauge_voting_start_date + GAUGE_VOTING_PERIOD {
//         TOTAL_ESSENCE.save(deps.storage, &(total_essence, block_time))?;
//         for (user_address, user_essence) in user_and_essence_list {
//             USER_ESSENCE.save(deps.storage, &Addr::unchecked(user_address), &user_essence)?;
//         }
//     }

//     Ok(Response::new().add_attribute("action", "try_capture_essence"))
// }
