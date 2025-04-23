use std::{collections::HashSet, hash::Hash};

use cosmwasm_std::{StdError, StdResult};

pub const SECONDS_PER_DAY: u64 = 24 * 3_600;
pub const UNBONDING_PERIOD_0: u64 = 14 * SECONDS_PER_DAY;
pub const UNBONDING_PERIOD_1: u64 = 28 * SECONDS_PER_DAY;
pub const UNBONDING_FEE_RATE: &str = "0.05";

pub fn check_unbonding_period(unbonding_period: u64, err: impl ToString) -> StdResult<()> {
    if ![UNBONDING_PERIOD_0, UNBONDING_PERIOD_1].contains(&unbonding_period) {
        Err(StdError::generic_err(err.to_string()))?;
    }

    Ok(())
}

pub fn has_unique_elements<T>(iter: T) -> bool
where
    T: IntoIterator,
    T::Item: Eq + Hash,
{
    let mut uniq = HashSet::new();
    iter.into_iter().all(move |x| uniq.insert(x))
}
