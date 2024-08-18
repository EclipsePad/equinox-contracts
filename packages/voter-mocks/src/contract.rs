use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
};

use equinox_msg::voter::{
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
    state::{
        REWARDS_CLAIM_STAGE, STAKE_ASTRO_REPLY_ID, SWAP_REWARDS_REPLY_ID_MAX,
        SWAP_REWARDS_REPLY_ID_MIN,
    },
    types::RewardsClaimStage,
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
        ExecuteMsg::Pause {} => e::try_pause(deps, env, info),

        ExecuteMsg::Unpause {} => e::try_unpause(deps, env, info),

        ExecuteMsg::AcceptAdminRole {} => e::try_accept_admin_role(deps, env, info),

        ExecuteMsg::UpdateAddressConfig {
            admin,
            worker_list,
            eclipse_dao,
            eclipsepad_foundry,
            eclipsepad_minter,
            eclipsepad_staking,
            eclipsepad_tribute_market,
            eclipse_single_sided_vault,
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
            eclipse_single_sided_vault,
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
        } => e::try_update_essence_allocation(deps, env, info, user_and_essence_list),

        ExecuteMsg::SwapToEclipAstro {} => e::try_swap_to_eclip_astro(deps, env, info),

        ExecuteMsg::UpdateAstroStakingRewardConfig { config } => {
            e::try_update_astro_staking_reward_config(deps, env, info, config)
        }

        ExecuteMsg::ClaimAstroRewards {} => e::try_claim_astro_staking_rewards(deps, env, info),

        ExecuteMsg::ClaimTreasuryRewards {} => {
            e::try_claim_astro_staking_treasury_rewards(deps, env, info)
        }

        ExecuteMsg::SetDelegation { weight } => e::try_set_delegation(deps, env, info, weight),

        ExecuteMsg::PlaceVote { weight_allocation } => {
            e::try_place_vote(deps, env, info, weight_allocation)
        }

        ExecuteMsg::PlaceVoteAsDao { weight_allocation } => {
            e::try_place_vote_as_dao(deps, env, info, weight_allocation)
        }

        ExecuteMsg::ClaimRewards {} => e::try_claim_rewards(deps, env, info),

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

        QueryMsg::BribesAllocation {} => to_json_binary(&q::query_bribes_allocation(deps, env)?),

        QueryMsg::VotingPower { address } => {
            to_json_binary(&q::query_voting_power(deps, env, address)?)
        }

        QueryMsg::XastroPrice {} => to_json_binary(&q::query_xastro_price(deps, env)?),

        QueryMsg::EclipAstroMintedByVoter {} => {
            to_json_binary(&q::query_eclip_astro_minted_by_voter(deps, env)?)
        }

        QueryMsg::User {
            address,
            block_time,
        } => to_json_binary(&q::query_user(deps, env, address, block_time)?),

        QueryMsg::UserList {
            block_time,
            amount,
            start_from,
        } => to_json_binary(&q::query_user_list(
            deps, env, block_time, amount, start_from,
        )?),

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

        QueryMsg::OperationStatus {} => to_json_binary(&q::query_operation_status(deps, env)?),

        QueryMsg::AstroStakingRewards {} => {
            to_json_binary(&q::query_astro_staking_rewards(deps, env)?)
        }
        QueryMsg::AstroStakingTreasuryRewards {} => {
            to_json_binary(&q::query_astro_staking_treasury_rewards(deps, env)?)
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
        SudoMsg::Push {} => match REWARDS_CLAIM_STAGE.load(deps.storage)? {
            RewardsClaimStage::Swapped => e::try_vote(deps, env),
            RewardsClaimStage::Unclaimed => e::try_claim(deps, env),
            RewardsClaimStage::Claimed => e::try_swap(deps, env),
        },
    }
}
