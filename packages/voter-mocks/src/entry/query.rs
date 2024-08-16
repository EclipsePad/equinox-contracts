use cosmwasm_std::{Decimal, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

use equinox_msg::voter::{
    msg::{
        AstroStakingRewardResponse, DaoResponse, OperationStatusResponse, UserListResponse,
        UserListResponseItem, UserResponse, VoterInfoResponse,
    },
    types::{
        AddressConfig, BribesAllocationItem, DateConfig, EpochInfo, RouteListItem, TokenConfig,
        UserType,
    },
};

use crate::{
    helpers::{
        get_accumulated_rewards, get_astro_and_xastro_supply, get_total_votes, get_user_types,
        get_user_weights, query_astroport_bribe_allocation, query_astroport_rewards,
        query_eclipsepad_bribe_allocation, query_eclipsepad_rewards, split_user_essence_info,
    },
    math::{
        calc_essence_allocation, calc_merged_bribe_allocations, calc_merged_rewards,
        calc_splitted_user_essence_info, calc_voting_power, calc_xastro_price, calculate_claimable,
        calculate_eclipastro_amount,
    },
    state::{
        ADDRESS_CONFIG, ASTRO_PENDING_TREASURY_REWARD, ASTRO_STAKING_REWARD_CONFIG,
        DAO_ESSENCE_ACC, DAO_WEIGHTS_ACC, DATE_CONFIG, DELEGATOR_ESSENCE_FRACTIONS,
        ECLIP_ASTRO_MINTED_BY_VOTER, ELECTOR_ESSENCE_ACC, ELECTOR_WEIGHTS_ACC, EPOCH_COUNTER,
        IS_PAUSED, REWARDS_CLAIM_STAGE, ROUTE_CONFIG, SLACKER_ESSENCE_ACC, TOKEN_CONFIG,
        TOTAL_CONVERT_INFO, USER_ESSENCE, VOTE_RESULTS,
    },
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

// query from both tribute markets
pub fn query_rewards(deps: Deps, env: Env) -> StdResult<Vec<(Uint128, String)>> {
    let sender = &env.contract.address;
    let astroport_rewards = query_astroport_rewards(deps, sender)?;
    let eclipsepad_rewards = query_eclipsepad_rewards(deps, sender)?;

    Ok(calc_merged_rewards(&astroport_rewards, &eclipsepad_rewards))
}

pub fn query_xastro_price(deps: Deps, _env: Env) -> StdResult<Decimal> {
    let (astro_supply, xastro_supply) = get_astro_and_xastro_supply(deps)?;
    Ok(calc_xastro_price(astro_supply, xastro_supply))
}

pub fn query_eclip_astro_minted_by_voter(deps: Deps, _env: Env) -> StdResult<Uint128> {
    ECLIP_ASTRO_MINTED_BY_VOTER.load(deps.storage)
}

// query from both tribute markets
pub fn query_bribes_allocation(deps: Deps, _env: Env) -> StdResult<Vec<BribesAllocationItem>> {
    let astroport_bribe_allocation = query_astroport_bribe_allocation(deps)?;
    let eclipsepad_bribe_allocation = query_eclipsepad_bribe_allocation(deps)?;

    Ok(calc_merged_bribe_allocations(
        &astroport_bribe_allocation,
        &eclipsepad_bribe_allocation,
    ))
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

/// rewards are the same for different user types as we have common rewards storage
pub fn query_user(
    deps: Deps,
    env: Env,
    address: String,
    block_time: Option<u64>,
) -> StdResult<Vec<UserResponse>> {
    let block_time = block_time.unwrap_or(env.block.time.seconds());
    let user = &deps.api.addr_validate(&address)?;
    let (delegator_essence_info, elector_or_slacker_essence_info) =
        &split_user_essence_info(deps.storage, user);

    get_user_types(deps.storage, user)?
        .iter()
        .map(|user_type| -> StdResult<UserResponse> {
            let user_weights = get_user_weights(deps.storage, user, user_type);
            let essence_info = match user_type {
                UserType::Delegator => delegator_essence_info,
                _ => elector_or_slacker_essence_info,
            };
            let essence_value = essence_info.capture(block_time);
            let (_, user_rewards) = get_accumulated_rewards(deps.storage, user, block_time)?;

            Ok(UserResponse {
                user_type: user_type.to_owned(),
                essence_info: essence_info.to_owned(),
                essence_value,
                weights: user_weights,
                rewards: user_rewards,
            })
        })
        .collect()
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
            let delegator_essence_fraction = DELEGATOR_ESSENCE_FRACTIONS
                .load(deps.storage, &address)
                .unwrap_or_default();
            let (delegator_essence_info, elector_or_slacker_essence_info) =
                &calc_splitted_user_essence_info(&essence_info, delegator_essence_fraction);

            let user_info = get_user_types(deps.storage, &address)?
                .iter()
                .map(|user_type| -> StdResult<UserResponse> {
                    let user_weights = get_user_weights(deps.storage, &address, user_type);
                    let essence_info = match user_type {
                        UserType::Delegator => delegator_essence_info,
                        _ => elector_or_slacker_essence_info,
                    };
                    let essence_value = essence_info.capture(block_time);
                    let (_, user_rewards) =
                        get_accumulated_rewards(deps.storage, &address, block_time)?;

                    Ok(UserResponse {
                        user_type: user_type.to_owned(),
                        essence_info: essence_info.to_owned(),
                        essence_value,
                        weights: user_weights,
                        rewards: user_rewards,
                    })
                })
                .collect::<StdResult<Vec<UserResponse>>>()?;

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
    let total_votes = get_total_votes(deps.storage, block_time)?.essence;

    Ok(VoterInfoResponse {
        block_time,
        elector_votes,
        slacker_essence_acc,
        total_votes,
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

/// query reward
pub fn query_astro_staking_rewards(deps: Deps, env: Env) -> StdResult<AstroStakingRewardResponse> {
    let res: (AstroStakingRewardResponse, Uint128) = _query_astro_staking_rewards(deps, env)?;
    Ok(res.0)
}

pub fn query_astro_staking_treasury_rewards(deps: Deps, _env: Env) -> StdResult<Uint128> {
    let astro_staking_treasury_rewards = ASTRO_PENDING_TREASURY_REWARD
        .load(deps.storage)
        .unwrap_or_default();
    Ok(astro_staking_treasury_rewards)
}

pub fn _query_astro_staking_rewards(
    deps: Deps,
    _env: Env,
) -> StdResult<(AstroStakingRewardResponse, Uint128)> {
    let reward_config = ASTRO_STAKING_REWARD_CONFIG.load(deps.storage)?;
    let total_convert_info = TOTAL_CONVERT_INFO.load(deps.storage).unwrap_or_default();

    // ASTRO / xASTRO rate from voter contract
    let (astro_supply, xastro_supply) = get_astro_and_xastro_supply(deps)?;
    // calculate user rewards as xASTRO
    let claimable_xastro = calculate_claimable(
        total_convert_info.total_xastro,
        total_convert_info.total_astro_deposited,
        xastro_supply,
        astro_supply,
        total_convert_info.claimed_xastro,
    );

    let users_reward = claimable_xastro.multiply_ratio(reward_config.users, 10000u32);
    let treasury_reward = claimable_xastro.checked_sub(users_reward).unwrap();
    Ok((
        AstroStakingRewardResponse {
            users: calculate_eclipastro_amount(xastro_supply, astro_supply, users_reward),
            treasury: calculate_eclipastro_amount(xastro_supply, astro_supply, treasury_reward),
        },
        claimable_xastro,
    ))
}
