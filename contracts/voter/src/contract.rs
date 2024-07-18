use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult,
};

use equinox_msg::voter::{
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
    state::{STAKE_ASTRO_REPLY_ID, SWAP_REWARDS_REPLY_ID_MAX, SWAP_REWARDS_REPLY_ID_MIN},
};

use crate::{
    entry::{execute as e, instantiate::try_instantiate, migrate::migrate_contract, query as q},
    error::ContractError,
};

/// Creates a new contract with the specified parameters in the [`InstantiateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    try_instantiate(deps, env, info, msg)
}

/// Exposes execute functions available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AcceptAdminRole {} => e::try_accept_admin_role(deps, env, info),

        ExecuteMsg::UpdateAddressConfig {
            admin,
            worker_list,
            eclipse_dao,
            eclipsepad_foundry,
            eclipsepad_minter,
            eclipsepad_staking,
            eclipsepad_tribute_market,
            astroport_staking,
            astroport_assembly,
            astroport_voting_escrow,
            astroport_emission_controller,
            astroport_router,
            astroport_tribute_market,
        } => e::try_update_address_config(
            deps,
            env,
            info,
            admin,
            worker_list,
            eclipse_dao,
            eclipsepad_foundry,
            eclipsepad_minter,
            eclipsepad_staking,
            eclipsepad_tribute_market,
            astroport_staking,
            astroport_assembly,
            astroport_voting_escrow,
            astroport_emission_controller,
            astroport_router,
            astroport_tribute_market,
        ),

        ExecuteMsg::UpdateTokenConfig {
            eclip,
            astro,
            xastro,
            eclip_astro,
        } => e::try_update_token_config(deps, env, info, eclip, astro, xastro, eclip_astro),

        ExecuteMsg::UpdateDateConfig {
            genesis_epoch_start_date,
            epoch_length,
            vote_delay,
        } => e::try_update_date_config(
            deps,
            env,
            info,
            genesis_epoch_start_date,
            epoch_length,
            vote_delay,
        ),

        ExecuteMsg::UpdateEssenceAllocation {
            user_and_essence_list,
            total_essence,
        } => {
            e::try_update_essence_allocation(deps, env, info, user_and_essence_list, total_essence)
        }

        ExecuteMsg::SwapToEclipAstro {} => e::try_swap_to_eclip_astro(deps, env, info),

        ExecuteMsg::SwapXastroToAstro {} => unimplemented!(),

        ExecuteMsg::Delegate {} => e::try_delegate(deps, env, info),

        ExecuteMsg::Undelegate {} => e::try_undelegate(deps, env, info),

        ExecuteMsg::PlaceVote { weight_allocation } => {
            e::try_place_vote(deps, env, info, weight_allocation)
        }

        ExecuteMsg::PlaceVoteAsDao { weight_allocation } => {
            e::try_place_vote_as_dao(deps, env, info, weight_allocation)
        }

        ExecuteMsg::ClaimRewards {} => unimplemented!(),

        ExecuteMsg::UpdateRouteList { route_list } => {
            e::try_update_route_list(deps, env, info, route_list)
        }
    }
}

/// Exposes queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::AddressConfig {} => to_json_binary(&q::query_address_config(deps, env)?),

        QueryMsg::TokenConfig {} => to_json_binary(&q::query_token_config(deps, env)?),

        QueryMsg::DateConfig {} => to_json_binary(&q::query_date_config(deps, env)?),

        QueryMsg::Rewards {} => to_json_binary(&q::query_rewards(deps, env)?),

        QueryMsg::BribesAllocation {} => unimplemented!(),

        QueryMsg::VotingPower { address } => {
            to_json_binary(&q::query_voting_power(deps, env, address)?)
        }

        QueryMsg::XastroPrice {} => to_json_binary(&q::query_xastro_price(deps, env)?),

        QueryMsg::User {
            address,
            block_time,
        } => to_json_binary(&q::query_user(deps, env, address, block_time)?),

        QueryMsg::ElectorList { amount, start_from } => {
            to_json_binary(&q::query_elector_list(deps, env, amount, start_from)?)
        }

        QueryMsg::DelegatorList { amount, start_from } => {
            to_json_binary(&q::query_delegator_list(deps, env, amount, start_from)?)
        }

        QueryMsg::SlackerList { amount, start_from } => {
            to_json_binary(&q::query_slacker_list(deps, env, amount, start_from)?)
        }

        QueryMsg::DaoInfo { block_time } => {
            to_json_binary(&q::query_dao_info(deps, env, block_time)?)
        }

        QueryMsg::VoterInfo { block_time } => {
            to_json_binary(&q::query_voter_info(deps, env, block_time)?)
        }

        QueryMsg::EpochInfo {} => to_json_binary(&q::query_epoch_info(deps, env)?),

        QueryMsg::RouteList { amount, start_from } => {
            to_json_binary(&q::query_route_list(deps, env, amount, start_from)?)
        }
    }
}

/// Manages contract migration.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    migrate_contract(deps, env, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    let Reply { id, result } = reply;

    match id {
        STAKE_ASTRO_REPLY_ID => e::handle_stake_astro_reply(deps, env, &result),
        SWAP_REWARDS_REPLY_ID_MIN..=SWAP_REWARDS_REPLY_ID_MAX => {
            e::handle_swap_reply(deps, env, &result)
        }
        _ => Err(ContractError::UnknownReplyId(id)),
    }
}

/// Exposes all functions that can be called only by Cosmos SDK modules
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        // TODO: use Process
        SudoMsg::Vote {} => e::try_vote(deps, env),

        SudoMsg::Claim {} => e::try_claim(deps, env),

        SudoMsg::Swap {} => e::try_swap(deps, env),
    }
}
