use cosmwasm_std::{Decimal256, StdResult, Uint128, Uint256};
use equinox_msg::lockdrop::Config;
use std::cmp::min;

use crate::config::MAX_WITHDRAW_BPS;

pub fn calculate_weight(amount: Uint128, duration: u64, config: &Config) -> StdResult<Uint256> {
    let lock_config = config
        .lock_configs
        .iter()
        .find(|c| c.duration == duration)
        .unwrap();
    let lock_weight = Decimal256::from_ratio(lock_config.multiplier, 1u128);
    Ok(lock_weight
        .checked_mul(Decimal256::from_ratio(amount, 1u128))
        .unwrap()
        .to_uint_floor())
}

pub fn calculate_max_withdrawal_amount_allowed(
    current_timestamp: u64,
    config: &Config,
    amount: Uint128,
) -> Uint128 {
    let withdrawal_cutoff_init_point = config.init_timestamp + config.deposit_window;

    // Deposit window :: 100% withdrawals allowed
    if current_timestamp < withdrawal_cutoff_init_point {
        return amount;
    }

    // max withdrawal allowed decreasing linearly from 50% to 0% vs time elapsed
    let withdrawal_cutoff_final = withdrawal_cutoff_init_point + config.withdrawal_window;
    //  Deposit window closed, 2nd half of withdrawal window :: max withdrawal allowed decreases linearly from 50% to 0% vs time elapsed
    if current_timestamp < withdrawal_cutoff_final {
        let time_left = withdrawal_cutoff_final - current_timestamp;
        min(
            amount.multiply_ratio(MAX_WITHDRAW_BPS, 10000u64),
            amount.multiply_ratio(time_left, config.withdrawal_window),
        )
    }
    // Withdrawals not allowed
    else {
        Uint128::zero()
    }
}
