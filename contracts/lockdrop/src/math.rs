use cosmwasm_std::{Decimal, Decimal256, OverflowError, StdResult, Uint128, Uint256};
use equinox_msg::lockdrop::Config;

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

pub fn calculate_max_withdrawal_percent_allowed(
    current_timestamp: u64,
    config: &Config,
) -> Decimal {
    let withdrawal_cutoff_init_point = config.init_timestamp + config.deposit_window;

    // Deposit window :: 100% withdrawals allowed
    if current_timestamp < withdrawal_cutoff_init_point {
        return Decimal::from_ratio(100u32, 100u32);
    }

    let withdrawal_cutoff_second_point =
        withdrawal_cutoff_init_point + (config.withdrawal_window / 2u64);
    // Deposit window closed, 1st half of withdrawal window :: 50% withdrawals allowed
    if current_timestamp <= withdrawal_cutoff_second_point {
        return Decimal::from_ratio(50u32, 100u32);
    }

    // max withdrawal allowed decreasing linearly from 50% to 0% vs time elapsed
    let withdrawal_cutoff_final = withdrawal_cutoff_init_point + config.withdrawal_window;
    //  Deposit window closed, 2nd half of withdrawal window :: max withdrawal allowed decreases linearly from 50% to 0% vs time elapsed
    if current_timestamp < withdrawal_cutoff_final {
        let time_left = withdrawal_cutoff_final - current_timestamp;
        Decimal::from_ratio(
            50u64 * time_left,
            100u64 * (withdrawal_cutoff_final - withdrawal_cutoff_second_point),
        )
    }
    // Withdrawals not allowed
    else {
        Decimal::from_ratio(0u32, 100u32)
    }
}

pub fn calculate_eclipastro_staked(
    astro: Uint128,
    xastro: Uint128,
    conversion_rate: Decimal,
) -> Result<Uint128, OverflowError> {
    astro.checked_add(
        conversion_rate
            .checked_mul(Decimal::from_ratio(xastro, 1u128))
            .unwrap()
            .to_uint_floor(),
    )
}

pub fn calculate_eclipastro_amount_for_lp(
    astro_locked: Uint128,
    xastro_locked: Uint128,
    conversion_rate: Decimal,
) -> Result<Uint128, OverflowError> {
    astro_locked
        .checked_div(Uint128::from(2u128))
        .unwrap()
        .checked_add(
            conversion_rate
                .checked_mul(Decimal::from_ratio(xastro_locked, 2u128))
                .unwrap()
                .to_uint_floor(),
        )
}
