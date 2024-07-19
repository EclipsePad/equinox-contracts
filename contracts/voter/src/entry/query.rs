use cosmwasm_std::{Decimal, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

use eclipse_base::{converters::u128_to_dec, utils::unwrap_field};
use equinox_msg::voter::{
    msg::{
        DaoResponse, UserListResponse, UserListResponseItem, UserResponse, UserType,
        VoterInfoResponse,
    },
    state::{
        ADDRESS_CONFIG, DAO_ESSENCE_ACC, DAO_WEIGHTS_ACC, DATE_CONFIG, ELECTOR_ESSENCE_ACC,
        ELECTOR_WEIGHTS, ELECTOR_WEIGHTS_ACC, ELECTOR_WEIGHTS_REF, EPOCH_COUNTER, ROUTE_CONFIG,
        SLACKER_ESSENCE_ACC, TOKEN_CONFIG, USER_ESSENCE, VOTE_RESULTS,
    },
    types::{AddressConfig, DateConfig, EpochInfo, RouteListItem, TokenConfig},
};

use crate::{
    helpers::{get_total_votes, get_user_type, get_user_weights},
    math::calc_essence_allocation,
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

/// query voting power
pub fn query_voting_power(deps: Deps, env: Env, address: String) -> StdResult<Uint128> {
    let voter_address = &env.contract.address;
    let address = &deps.api.addr_validate(&address)?;
    let AddressConfig {
        astroport_voting_escrow,
        eclipsepad_staking,
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

    // TODO: calculate essence at the epoch start
    // query essence from eclipsepad-staking v3
    let eclipse_base::staking::msg::QueryEssenceResponse { essence, .. } =
        deps.querier.query_wasm_smart(
            eclipsepad_staking.clone(),
            &eclipse_base::staking::msg::QueryMsg::QueryEssence {
                user: address.to_string(),
            },
        )?;

    let eclipse_base::staking::msg::QueryEssenceResponse {
        essence: total_essence,
        ..
    } = deps.querier.query_wasm_smart(
        eclipsepad_staking,
        &eclipse_base::staking::msg::QueryMsg::QueryTotalEssence {},
    )?;

    let voting_power = vxastro_amount * essence / total_essence;

    Ok(voting_power)
}

// pub fn query_voter_info(
//     deps: Deps,
//     _env: Env,
//     address: String,
// ) -> StdResult<astroport_governance::generator_controller::UserInfoResponse> {
//     let address = &deps.api.addr_validate(&address)?;
//     let Config {
//         astroport_generator_controller,
//         ..
//     } = CONFIG.load(deps.storage)?;

//     deps.querier.query_wasm_smart(
//         astroport_generator_controller,
//         &astroport_governance::generator_controller::QueryMsg::UserInfo {
//             user: address.to_string(),
//         },
//     )
// }

pub fn query_user(
    deps: Deps,
    env: Env,
    address: String,
    block_time: Option<u64>,
) -> StdResult<UserResponse> {
    let block_time = block_time.unwrap_or(env.block.time.seconds());
    let user = &deps.api.addr_validate(&address)?;
    let essence_info = USER_ESSENCE.load(deps.storage, user).unwrap_or_default();
    let essence_value = essence_info.capture(block_time);

    Ok(UserResponse {
        user_type: get_user_type(deps.storage, user)?,
        essence_info,
        essence_value,
        weights: get_user_weights(deps.storage, user)?,
    })
}

// pub fn query_elector_list(
//     deps: Deps,
//     env: Env,
//     amount: u32,
//     start_from: Option<String>,
// ) -> StdResult<UserListResponse> {
//     let block_time = env.block.time.seconds();
//     let address;
//     let start_bound = match start_from {
//         None => None,
//         Some(x) => {
//             address = deps.api.addr_validate(&x)?;
//             Some(Bound::exclusive(&address))
//         }
//     };

//     let list = ELECTOR_ESSENCE
//         .range(deps.storage, start_bound.clone(), None, Order::Ascending)
//         .take(amount as usize)
//         .map(|x| {
//             let (address, essence_info) = x.unwrap();
//             let weights = ELECTOR_WEIGHTS
//                 .load(deps.storage, &address)
//                 .unwrap_or_default();

//             Ok(UserListResponseItem {
//                 address,
//                 essence_info,
//                 weights: Some(weights),
//             })
//         })
//         .collect::<StdResult<Vec<UserListResponseItem>>>()?;

//     Ok(UserListResponse { block_time, list })
// }

// pub fn query_delegator_list(
//     deps: Deps,
//     env: Env,
//     amount: u32,
//     start_from: Option<String>,
// ) -> StdResult<UserListResponse> {
//     let block_time = env.block.time.seconds();
//     let address;
//     let start_bound = match start_from {
//         None => None,
//         Some(x) => {
//             address = deps.api.addr_validate(&x)?;
//             Some(Bound::exclusive(&address))
//         }
//     };

//     let list = DELEGATOR_ESSENCE
//         .range(deps.storage, start_bound.clone(), None, Order::Ascending)
//         .take(amount as usize)
//         .map(|x| {
//             let (address, essence_info) = x.unwrap();

//             UserListResponseItem {
//                 address,
//                 essence_info,
//                 weights: None,
//             }
//         })
//         .collect::<Vec<UserListResponseItem>>();

//     Ok(UserListResponse { block_time, list })
// }

// pub fn query_slacker_list(
//     deps: Deps,
//     env: Env,
//     amount: u32,
//     start_from: Option<String>,
// ) -> StdResult<UserListResponse> {
//     let block_time = env.block.time.seconds();
//     let address;
//     let start_bound = match start_from {
//         None => None,
//         Some(x) => {
//             address = deps.api.addr_validate(&x)?;
//             Some(Bound::exclusive(&address))
//         }
//     };

//     let list = SLACKER_ESSENCE
//         .range(deps.storage, start_bound.clone(), None, Order::Ascending)
//         .take(amount as usize)
//         .map(|x| {
//             let (address, essence_info) = x.unwrap();

//             UserListResponseItem {
//                 address,
//                 essence_info,
//                 weights: None,
//             }
//         })
//         .collect::<Vec<UserListResponseItem>>();

//     Ok(UserListResponse { block_time, list })
// }

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
