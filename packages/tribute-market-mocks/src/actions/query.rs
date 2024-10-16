use cosmwasm_std::{Addr, Deps, Env, StdResult, Uint128};

use eclipse_base::{
    tribute_market::state::{
        BRIBES_ALLOCATION, INSTANTIATION_DATE, REWARDS, REWARDS_DISTRIBUTION_DELAY,
    },
    voter::types::BribesAllocationItem,
};

pub fn query_rewards(deps: Deps, env: Env, user: String) -> StdResult<Vec<(Uint128, String)>> {
    let block_time = env.block.time.seconds();
    let instantiation_date = INSTANTIATION_DATE.load(deps.storage)?;

    if block_time < instantiation_date + REWARDS_DISTRIBUTION_DELAY {
        return Ok(vec![]);
    }

    REWARDS.load(deps.storage, &Addr::unchecked(user))
}

pub fn query_bribes_allocation(deps: Deps, _env: Env) -> StdResult<Vec<BribesAllocationItem>> {
    Ok(BRIBES_ALLOCATION.load(deps.storage).unwrap_or_default())
}
