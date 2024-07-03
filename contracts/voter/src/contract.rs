use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult,
};

use equinox_msg::voter::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

use crate::{
    entry::{execute as e, instantiate::try_instantiate, migrate::migrate_contract, query as q},
    error::ContractError,
    state::STAKE_ASTRO_REPLY_ID,
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
            astroport_tribute_market,
        ),

        ExecuteMsg::UpdateTokenConfig {
            astro,
            xastro,
            eclip_astro,
        } => e::try_update_token_config(deps, env, info, astro, xastro, eclip_astro),

        ExecuteMsg::UpdateDateConfig {
            epochs_start,
            epoch_length,
            vote_cooldown,
            vote_delay,
        } => e::try_update_date_config(
            deps,
            env,
            info,
            epochs_start,
            epoch_length,
            vote_cooldown,
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

        ExecuteMsg::Delegate {} => unimplemented!(),

        ExecuteMsg::Undelegate {} => unimplemented!(),

        ExecuteMsg::PlaceVote { weight_allocation } => unimplemented!(),

        ExecuteMsg::PlaceVoteAsDao { weight_allocation } => unimplemented!(),

        ExecuteMsg::Vote {} => unimplemented!(),

        ExecuteMsg::ClaimRewards {} => unimplemented!(),
    }
}

/// Exposes queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::AddressConfig {} => to_json_binary(&q::query_address_config(deps, env)?),

        QueryMsg::TokenConfig {} => to_json_binary(&q::query_token_config(deps, env)?),

        QueryMsg::DateConfig {} => to_json_binary(&q::query_date_config(deps, env)?),

        QueryMsg::Rewards {} => unimplemented!(),

        QueryMsg::BribesAllocation {} => unimplemented!(),

        QueryMsg::VotingPower { address } => {
            to_json_binary(&q::query_voting_power(deps, env, address)?)
        }

        QueryMsg::XastroPrice {} => to_json_binary(&q::query_xastro_price(deps, env)?),

        QueryMsg::VoterInfo { address } => unimplemented!(),

        QueryMsg::Essence { address } => unimplemented!(),

        QueryMsg::EssenceList { amount, start_from } => unimplemented!(),
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
        _ => Err(ContractError::UnknownReplyId(id)),
    }
}
