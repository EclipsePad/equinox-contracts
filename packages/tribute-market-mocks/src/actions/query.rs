use cosmwasm_std::{Deps, Env, StdResult, Uint128};

use crate::state::{CLAIMABLE_REWARDS_PER_TX, INSTANTIATION_DATE, REWARDS_DISTRIBUTION_DELAY};

pub fn query_rewards(deps: Deps, env: Env, _user: String) -> StdResult<Vec<(String, Uint128)>> {
    let block_time = env.block.time.seconds();
    let instantiation_date = INSTANTIATION_DATE.load(deps.storage)?;

    if block_time < instantiation_date + REWARDS_DISTRIBUTION_DELAY {
        return Ok(vec![]);
    }

    CLAIMABLE_REWARDS_PER_TX.load(deps.storage)
}
