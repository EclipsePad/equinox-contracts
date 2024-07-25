use cosmwasm_std::{Decimal, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

use eclipse_base::{converters::u128_to_dec, utils::unwrap_field};
use equinox_msg::voter::{
    msg::{
        DaoResponse, OperationStatusResponse, UserListResponse, UserListResponseItem, UserResponse,
        VoterInfoResponse,
    },
    state::{
        ADDRESS_CONFIG, DAO_ESSENCE_ACC, DAO_WEIGHTS_ACC, DATE_CONFIG, ELECTOR_ESSENCE_ACC,
        ELECTOR_WEIGHTS_ACC, EPOCH_COUNTER, IS_PAUSED, REWARDS_CLAIM_STAGE, ROUTE_CONFIG,
        SLACKER_ESSENCE_ACC, TOKEN_CONFIG, USER_ESSENCE, VOTE_RESULTS,
    },
    types::{
        AddressConfig, BribesAllocationItem, DateConfig, EpochInfo, RouteListItem, TokenConfig,
    },
};

use crate::{
    helpers::{get_accumulated_rewards, get_total_votes, get_user_type, get_user_weights},
    math::{calc_essence_allocation, calc_voting_power},
};

pub fn query_address_config(deps: Deps, _env: Env) -> StdResult<AddressConfig> {
    ADDRESS_CONFIG.load(deps.storage)
}

pub fn query_token_config(deps: Deps, _env: Env) -> StdResult<TokenConfig> {
    TOKEN_CONFIG.load(deps.storage)
}

pub fn query_date_config(deps: Deps, _env: Env) -> StdResult<DateConfig> {
    DATE_CONFIG.load(deps.storage)
}

pub fn query_rewards(deps: Deps, env: Env) -> StdResult<Vec<(Uint128, String)>> {
    let address_config = ADDRESS_CONFIG.load(deps.storage)?;
    let astroport_tribute_market = &unwrap_field(
        address_config.astroport_tribute_market,
        "astroport_tribute_market",
    )?;

    deps.querier.query_wasm_smart::<Vec<(Uint128, String)>>(
        astroport_tribute_market,
        &tribute_market_mocks::msg::QueryMsg::Rewards {
            user: env.contract.address.to_string(),
        },
    )
}

pub fn query_xastro_price(deps: Deps, _env: Env) -> StdResult<Decimal> {
    let AddressConfig {
        eclipsepad_staking, ..
    } = ADDRESS_CONFIG.load(deps.storage)?;

    let xastro_amount: Uint128 = deps.querier.query_wasm_smart(
        eclipsepad_staking.to_string(),
        &astroport::staking::QueryMsg::TotalShares {},
    )?;

    let astro_amount: Uint128 = deps.querier.query_wasm_smart(
        eclipsepad_staking.to_string(),
        &astroport::staking::QueryMsg::TotalDeposit {},
    )?;

    Ok(u128_to_dec(astro_amount) / u128_to_dec(xastro_amount))
}

// TODO: query from both tribute markets
pub fn query_bribes_allocation(deps: Deps, _env: Env) -> StdResult<Vec<BribesAllocationItem>> {
    let address_config = ADDRESS_CONFIG.load(deps.storage)?;
    let astroport_tribute_market = &unwrap_field(
        address_config.astroport_tribute_market,
        "astroport_tribute_market",
    )?;

    deps.querier.query_wasm_smart::<Vec<BribesAllocationItem>>(
        astroport_tribute_market,
        &tribute_market_mocks::msg::QueryMsg::BribesAllocation {},
    )
}

/// query voting power
pub fn query_voting_power(deps: Deps, env: Env, address: String) -> StdResult<Uint128> {
    let block_time = env.block.time.seconds();
    let voter_address = &env.contract.address;
    let address = &deps.api.addr_validate(&address)?;
    let AddressConfig {
        astroport_voting_escrow,
        ..
    } = ADDRESS_CONFIG.load(deps.storage)?;

    // query total vxASTRO owned by voter contract
    let vxastro_amount: Uint128 = deps.querier.query_wasm_smart(
        astroport_voting_escrow,
        &astroport_governance::voting_escrow::QueryMsg::UserVotingPower {
            user: voter_address.to_string(),
            timestamp: None,
        },
    )?;

    // voter contract has full voting power
    if address == voter_address {
        return Ok(vxastro_amount);
    }

    let user_essence = USER_ESSENCE
        .load(deps.storage, address)
        .unwrap_or_default()
        .capture(block_time);
    let elector_essence_acc = ELECTOR_ESSENCE_ACC
        .load(deps.storage)
        .unwrap_or_default()
        .capture(block_time);
    let dao_essence_acc = DAO_ESSENCE_ACC
        .load(deps.storage)
        .unwrap_or_default()
        .capture(block_time);
    let slacker_essence_acc = SLACKER_ESSENCE_ACC
        .load(deps.storage)
        .unwrap_or_default()
        .capture(block_time);

    Ok(calc_voting_power(
        vxastro_amount,
        user_essence,
        elector_essence_acc,
        dao_essence_acc,
        slacker_essence_acc,
    ))
}

pub fn query_user(
    deps: Deps,
    env: Env,
    address: String,
    block_time: Option<u64>,
) -> StdResult<UserResponse> {
    let block_time = block_time.unwrap_or(env.block.time.seconds());
    let user = &deps.api.addr_validate(&address)?;
    let user_type = get_user_type(deps.storage, user)?;
    let user_weights = get_user_weights(deps.storage, user, &user_type);
    let essence_info = USER_ESSENCE.load(deps.storage, user).unwrap_or_default();
    let essence_value = essence_info.capture(block_time);
    let (_, user_rewards) = get_accumulated_rewards(deps.storage, user, block_time)?;

    Ok(UserResponse {
        user_type,
        essence_info,
        essence_value,
        weights: user_weights,
        rewards: user_rewards,
    })
}

pub fn query_user_list(
    deps: Deps,
    env: Env,
    block_time: Option<u64>,
    amount: u32,
    start_from: Option<String>,
) -> StdResult<UserListResponse> {
    let block_time = block_time.unwrap_or(env.block.time.seconds());
    let address;
    let start_bound = match start_from {
        None => None,
        Some(x) => {
            address = deps.api.addr_validate(&x)?;
            Some(Bound::exclusive(&address))
        }
    };

    let list = USER_ESSENCE
        .range(deps.storage, start_bound.clone(), None, Order::Ascending)
        .take(amount as usize)
        .map(|x| {
            let (address, essence_info) = x.unwrap();
            let essence_value = essence_info.capture(block_time);
            let user_type = get_user_type(deps.storage, &address)?;
            let user_weights = get_user_weights(deps.storage, &address, &user_type);
            let (_, user_rewards) = get_accumulated_rewards(deps.storage, &address, block_time)?;
            let user_info = UserResponse {
                user_type,
                essence_info,
                essence_value,
                weights: user_weights,
                rewards: user_rewards,
            };

            Ok(UserListResponseItem { address, user_info })
        })
        .collect::<StdResult<Vec<UserListResponseItem>>>()?;

    Ok(UserListResponse { block_time, list })
}

pub fn query_dao_info(deps: Deps, env: Env, block_time: Option<u64>) -> StdResult<DaoResponse> {
    let block_time = block_time.unwrap_or(env.block.time.seconds());
    let essence_info = DAO_ESSENCE_ACC.load(deps.storage)?;
    let essence_value = essence_info.capture(block_time);
    let weights = DAO_WEIGHTS_ACC.load(deps.storage).unwrap_or_default();

    Ok(DaoResponse {
        essence_info,
        essence_value,
        weights,
    })
}

pub fn query_voter_info(
    deps: Deps,
    env: Env,
    block_time: Option<u64>,
) -> StdResult<VoterInfoResponse> {
    let block_time = block_time.unwrap_or(env.block.time.seconds());
    let elector_essence_acc = ELECTOR_ESSENCE_ACC.load(deps.storage)?;
    let elector_weights_acc = ELECTOR_WEIGHTS_ACC.load(deps.storage)?;
    let elector_votes = calc_essence_allocation(&elector_essence_acc, &elector_weights_acc);
    let slacker_essence_acc = SLACKER_ESSENCE_ACC.load(deps.storage)?;
    let vote_results = VOTE_RESULTS.load(deps.storage)?;

    let (total_essence_allocation, _total_weights_allocation) =
        get_total_votes(deps.storage, block_time)?;

    Ok(VoterInfoResponse {
        block_time,
        elector_votes,
        slacker_essence_acc,
        total_votes: total_essence_allocation,
        vote_results,
    })
}

pub fn query_epoch_info(deps: Deps, _env: Env) -> StdResult<EpochInfo> {
    EPOCH_COUNTER.load(deps.storage)
}

pub fn query_route_list(
    deps: Deps,
    _env: Env,
    amount: u32,
    start_from: Option<String>,
) -> StdResult<Vec<RouteListItem>> {
    let denom;
    let start_bound = match start_from {
        None => None,
        Some(x) => {
            denom = x;
            Some(Bound::exclusive(&*denom))
        }
    };

    ROUTE_CONFIG
        .range(deps.storage, start_bound.clone(), None, Order::Ascending)
        .take(amount as usize)
        .map(|x| {
            let (denom, route) = x.unwrap();

            Ok(RouteListItem { denom, route })
        })
        .collect::<StdResult<Vec<RouteListItem>>>()
}

pub fn query_operation_status(deps: Deps, _env: Env) -> StdResult<OperationStatusResponse> {
    Ok(OperationStatusResponse {
        is_paused: IS_PAUSED.load(deps.storage)?,
        rewards_claim_stage: REWARDS_CLAIM_STAGE.load(deps.storage)?,
    })
}
