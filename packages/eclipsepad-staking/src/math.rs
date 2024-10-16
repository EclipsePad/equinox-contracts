use std::cmp::min;

use cosmwasm_std::{Decimal, Uint128};

use eclipse_base::{
    converters::u128_to_dec,
    staking::{
        state::{DECREASING_REWARDS_PERIOD, MAX_APR, YEAR_IN_SECONDS},
        types::{LockerInfo, Vault},
    },
};

// ------------------------- math v3 ----------------------------------------------------

/// Essence based model with rewards for staking
///
/// essence = staking_essence + locking_essence     \
///
/// staking_essence = sum_over_vaults(staking_essence_per_vault)     \
/// staking_essence_per_vault = staking_amount * staking_period / seconds_per_essence     \
/// staking_period = min(block_time - vault_creation_date, seconds_per_essence)     \
///
/// locking_essence = sum_over_tiers(locking_essence_per_tier)     \
/// locking_essence_per_tier = sum_over_vaults(locking_essence_per_vault)     \
/// locking_essence_per_vault = locking_amount * locking_period / seconds_per_essence     \
///
///
/// rewards = staking_rewards + locking_rewards     \
///
/// staking_rewards = sum_over_vaults(staking_rewards_per_vault)     \
/// staking_rewards_per_vault = accumulated_staking_rewards_per_vault + (eclip_per_second * accumulation_period) * (staking_essence_per_vault / total_essence)     \
/// accumulation_period = block_time - vault_claim_date     \
///
/// locking_rewards = sum_over_tiers(locking_rewards_per_tier)     \
/// locking_rewards_per_tier = sum_over_vaults(locking_rewards_per_vault)     \
/// locking_rewards_per_vault = accumulated_locking_rewards_per_vault + (eclip_per_second * accumulation_period) * (locking_essence_per_vault / total_essence)     \
///
/// total_essence = total_staking_essence + total_locking_essence

/// staking_essence = sum_over_vaults(staking_essence_per_vault)
pub fn calc_staking_essence(
    staking_vaults: &[Vault],
    block_time: u64,
    seconds_per_essence: Uint128,
) -> Uint128 {
    staking_vaults.iter().fold(Uint128::zero(), |acc, vault| {
        acc + calc_staking_essence_per_vault(
            vault.amount,
            vault.creation_date,
            block_time,
            seconds_per_essence,
        )
    })
}

/// staking_essence_per_vault = staking_amount * staking_period / seconds_per_essence
pub fn calc_staking_essence_per_vault(
    staking_amount: Uint128,
    vault_creation_date: u64,
    block_time: u64,
    seconds_per_essence: Uint128,
) -> Uint128 {
    staking_amount * calc_staking_period(vault_creation_date, block_time, seconds_per_essence)
        / seconds_per_essence
}

/// staking_period = min(block_time - vault_creation_date, seconds_per_essence)
fn calc_staking_period(
    vault_creation_date: u64,
    block_time: u64,
    seconds_per_essence: Uint128,
) -> Uint128 {
    min(
        Uint128::from(block_time - vault_creation_date),
        seconds_per_essence,
    )
}

/// locking_essence = sum_over_tiers(locking_essence_per_tier)
pub fn calc_locking_essence(
    locker_infos: &[LockerInfo],
    lock_schedule: &[(u64, u64)],
    seconds_per_essence: Uint128,
) -> Uint128 {
    locker_infos
        .iter()
        .fold(Uint128::zero(), |acc, locker_info| {
            let (locking_period, _) = lock_schedule[locker_info.lock_tier as usize];

            acc + calc_locking_essence_per_tier(
                &locker_info.vaults,
                locking_period,
                seconds_per_essence,
            )
        })
}

/// locking_essence_per_tier = sum_over_vaults(locking_essence_per_vault)
pub fn calc_locking_essence_per_tier(
    locking_vaults: &[Vault],
    locking_period: u64,
    seconds_per_essence: Uint128,
) -> Uint128 {
    locking_vaults.iter().fold(Uint128::zero(), |acc, vault| {
        acc + calc_locking_essence_per_vault(vault.amount, locking_period, seconds_per_essence)
    })
}

/// locking_essence_per_vault = locking_amount * locking_period / seconds_per_essence
pub fn calc_locking_essence_per_vault(
    locking_amount: Uint128,
    locking_period: u64,
    seconds_per_essence: Uint128,
) -> Uint128 {
    locking_amount * Uint128::from(locking_period) / seconds_per_essence
}

/// staking_rewards = sum_over_vaults(staking_rewards_per_vault)
pub fn calc_staking_rewards(
    staking_vaults: &[Vault],
    eclip_per_second_update_date: u64,
    block_time: u64,
    seconds_per_essence: Uint128,
    eclip_per_second_before: u64,
    eclip_per_second_after: u64,
    total_essence: Uint128,
) -> Uint128 {
    if total_essence.is_zero() {
        return Uint128::zero();
    }

    staking_vaults.iter().fold(Uint128::zero(), |acc, vault| {
        acc + calc_staking_rewards_per_vault(
            vault.accumulated_rewards,
            calc_staking_essence_per_vault(
                vault.amount,
                vault.creation_date,
                block_time,
                seconds_per_essence,
            ),
            vault.claim_date,
            eclip_per_second_update_date,
            block_time,
            eclip_per_second_before,
            eclip_per_second_after,
            total_essence,
        )
    })
}

/// staking_rewards_per_vault = accumulated_staking_rewards_per_vault + (eclip_per_second * accumulation_period) * (staking_essence_per_vault / total_essence)
#[allow(clippy::too_many_arguments)]
pub fn calc_staking_rewards_per_vault(
    accumulated_staking_rewards_per_vault: Uint128,
    staking_essence_per_vault: Uint128,
    vault_claim_date: u64,
    eclip_per_second_update_date: u64,
    block_time: u64,
    eclip_per_second_before: u64,
    eclip_per_second_after: u64,
    total_essence: Uint128,
) -> Uint128 {
    if total_essence.is_zero() {
        return Uint128::zero();
    }

    accumulated_staking_rewards_per_vault
        + calc_eclip_multiplier(
            vault_claim_date,
            eclip_per_second_update_date,
            block_time,
            eclip_per_second_before,
            eclip_per_second_after,
        ) * staking_essence_per_vault
            / total_essence
}

pub fn calc_eclip_multiplier(
    vault_claim_date: u64,
    eclip_per_second_update_date: u64,
    block_time: u64,
    eclip_per_second_before: u64,
    eclip_per_second_after: u64,
) -> Uint128 {
    if eclip_per_second_update_date >= vault_claim_date
        && eclip_per_second_update_date <= block_time
    {
        let accumulation_period_1 = Uint128::from(eclip_per_second_update_date - vault_claim_date);
        let accumulation_period_2 = Uint128::from(block_time - eclip_per_second_update_date);

        Uint128::from(eclip_per_second_before) * accumulation_period_1
            + Uint128::from(eclip_per_second_after) * accumulation_period_2
    } else {
        let accumulation_period = Uint128::from(block_time - vault_claim_date);

        Uint128::from(eclip_per_second_before) * accumulation_period
    }
}

/// locking_rewards = sum_over_tiers(locking_rewards_per_tier)
#[allow(clippy::too_many_arguments)]
pub fn calc_locking_rewards(
    locker_infos: &[LockerInfo],
    lock_schedule: &[(u64, u64)],
    eclip_per_second_update_date: u64,
    block_time: u64,
    seconds_per_essence: Uint128,
    eclip_per_second_before: u64,
    eclip_per_second_after: u64,
    total_essence: Uint128,
) -> Uint128 {
    locker_infos
        .iter()
        .fold(Uint128::zero(), |acc, locker_info| {
            let (locking_period, _) = lock_schedule[locker_info.lock_tier as usize];

            acc + calc_locking_rewards_per_tier(
                &locker_info.vaults,
                locking_period,
                eclip_per_second_update_date,
                block_time,
                seconds_per_essence,
                eclip_per_second_before,
                eclip_per_second_after,
                total_essence,
            )
        })
}

/// locking_rewards = sum_over_vaults(locking_rewards_per_vault)
#[allow(clippy::too_many_arguments)]
pub fn calc_locking_rewards_per_tier(
    locking_vaults: &[Vault],
    locking_period: u64,
    eclip_per_second_update_date: u64,
    block_time: u64,
    seconds_per_essence: Uint128,
    eclip_per_second_before: u64,
    eclip_per_second_after: u64,
    total_essence: Uint128,
) -> Uint128 {
    locking_vaults.iter().fold(Uint128::zero(), |acc, vault| {
        acc + calc_locking_rewards_per_vault(
            vault.accumulated_rewards,
            calc_locking_essence_per_vault(vault.amount, locking_period, seconds_per_essence),
            vault.claim_date,
            eclip_per_second_update_date,
            block_time,
            eclip_per_second_before,
            eclip_per_second_after,
            total_essence,
        )
    })
}

/// locking_rewards_per_vault = accumulated_locking_rewards_per_vault + (eclip_per_second * accumulation_period) * (locking_essence_per_vault / total_essence)
#[allow(clippy::too_many_arguments)]
pub fn calc_locking_rewards_per_vault(
    accumulated_locking_rewards_per_vault: Uint128,
    locking_essence_per_vault: Uint128,
    vault_claim_date: u64,
    eclip_per_second_update_date: u64,
    block_time: u64,
    eclip_per_second_before: u64,
    eclip_per_second_after: u64,
    total_essence: Uint128,
) -> Uint128 {
    if total_essence.is_zero() {
        return Uint128::zero();
    }

    accumulated_locking_rewards_per_vault
        + calc_eclip_multiplier(
            vault_claim_date,
            eclip_per_second_update_date,
            block_time,
            eclip_per_second_before,
            eclip_per_second_after,
        ) * locking_essence_per_vault
            / total_essence
}

/// staking_essence_from_components = (a * block_time - b) / seconds_per_essence      \
/// where a = sum(staked_eclip_amount), b = sum(staked_eclip_amount * vault.creation_date)
pub fn calc_staking_essence_from_components(
    a: Uint128,
    b: Uint128,
    block_time: u64,
    seconds_per_essence: Uint128,
) -> Uint128 {
    let at = a * Uint128::from(block_time);

    min(at - min(b, at), a * Uint128::from(YEAR_IN_SECONDS)) / seconds_per_essence
}

/// a = sum(staked_eclip_amount), b = sum(staked_eclip_amount * vault.creation_date)
pub fn calc_components_from_staking_vaults(staking_vaults: &[Vault]) -> (Uint128, Uint128) {
    staking_vaults
        .iter()
        .fold((Uint128::zero(), Uint128::zero()), |(acc_a, acc_b), cur| {
            (
                acc_a + cur.amount,
                acc_b + cur.amount * Uint128::from(cur.creation_date),
            )
        })
}

/// penalty = penalty_multiplier * sum_over_vaults(locked_eclip_amount * (1 - min(locking_duration, locking_period) / locking_period))
pub fn calc_penalty_per_tier(
    locking_vaults: &[Vault],
    locking_period: u64,
    block_time: u64,
    penalty_multiplier: Decimal,
) -> Uint128 {
    (penalty_multiplier
        * u128_to_dec(locking_vaults.iter().fold(Uint128::zero(), |acc, vault| {
            let locking_duration = min(block_time - vault.creation_date, locking_period);

            acc + vault.amount
                - vault.amount * Uint128::from(locking_duration) / Uint128::from(locking_period)
        })))
    .to_uint_floor()
}

/// TODO: add description
#[allow(clippy::too_many_arguments)]
pub fn calc_bonded_vault(
    tier_4_vaults: &[Vault],
    lock_schedule: &[(u64, u64)],
    eclip_per_second_update_date: u64,
    block_time: u64,
    seconds_per_essence: Uint128,
    eclip_per_second_before: u64,
    eclip_per_second_after: u64,
    total_essence: Uint128,
) -> Vault {
    const TIER_4: usize = 4;
    let (locking_period, _) = lock_schedule[TIER_4];

    let amount = tier_4_vaults
        .iter()
        .fold(Uint128::zero(), |acc, cur| acc + cur.amount);

    let accumulated_rewards = calc_locking_rewards_per_tier(
        tier_4_vaults,
        locking_period,
        eclip_per_second_update_date,
        block_time,
        seconds_per_essence,
        eclip_per_second_before,
        eclip_per_second_after,
        total_essence,
    );

    Vault {
        amount,
        accumulated_rewards,
        creation_date: block_time,
        claim_date: block_time,
    }
}

/// TODO: add description
#[allow(clippy::too_many_arguments)]
pub fn split_bonded_vault(
    bonded_vault: &Vault,
    beclip_to_burn: Uint128,
    lock_schedule: &[(u64, u64)],
    eclip_per_second_update_date: u64,
    block_time: u64,
    seconds_per_essence: Uint128,
    eclip_per_second_before: u64,
    eclip_per_second_after: u64,
    total_essence: Uint128,
) -> (Option<Vault>, Vault) {
    let mut bonded_vault = calc_bonded_vault(
        &[bonded_vault.to_owned()],
        lock_schedule,
        eclip_per_second_update_date,
        block_time,
        seconds_per_essence,
        eclip_per_second_before,
        eclip_per_second_after,
        total_essence,
    );

    // bonded vault consumed partially
    if beclip_to_burn < bonded_vault.amount {
        bonded_vault.amount -= beclip_to_burn;

        let tier_4_vault = Vault {
            amount: beclip_to_burn,
            accumulated_rewards: Uint128::zero(),
            creation_date: block_time,
            claim_date: block_time,
        };

        return (Some(bonded_vault), tier_4_vault);
    }

    // bonded vault consumed completely
    let tier_4_vault = Vault {
        amount: bonded_vault.amount,
        accumulated_rewards: bonded_vault.accumulated_rewards,
        creation_date: block_time,
        claim_date: block_time,
    };

    (None, tier_4_vault)
}

/// vaults aggregation allows to merge several vaults into single one with new parameters:                                              \
/// amount_new = sum_over_vaults(amount_per_vault)                                                                                      \
/// accumulated_rewards_new = sum_over_vaults(accumulated_rewards_per_vault + staking_rewards_per_vault | locking_rewards_per_vault)    \
/// claim_date_new = block_time                                                                                                         \
/// creation_date_new = sum_over_vaults(amount_per_vault * (creation_date + locking_period)) / amount_new - locking_period              \
/// note: only staking vaults or single tier locking vaults can be merged                                                               \
#[allow(clippy::too_many_arguments)]
pub fn calc_aggregated_vault(
    vaults: &[Vault],
    locking_period: u64,
    eclip_per_second_update_date: u64,
    block_time: u64,
    seconds_per_essence: Uint128,
    eclip_per_second_before: u64,
    eclip_per_second_after: u64,
    total_essence: Uint128,
) -> Vault {
    let amount_new = vaults
        .iter()
        .fold(Uint128::zero(), |acc, cur| acc + cur.amount);

    let accumulated_rewards_new = if locking_period == 0 {
        calc_staking_rewards(
            vaults,
            eclip_per_second_update_date,
            block_time,
            seconds_per_essence,
            eclip_per_second_before,
            eclip_per_second_after,
            total_essence,
        )
    } else {
        calc_locking_rewards_per_tier(
            vaults,
            locking_period,
            eclip_per_second_update_date,
            block_time,
            seconds_per_essence,
            eclip_per_second_before,
            eclip_per_second_after,
            total_essence,
        )
    };

    let creation_date_new = vaults.iter().fold(Uint128::zero(), |acc, cur| {
        acc + cur.amount * Uint128::from(cur.creation_date + locking_period)
    }) / amount_new
        - Uint128::from(locking_period);

    Vault {
        amount: amount_new,
        accumulated_rewards: accumulated_rewards_new,
        creation_date: creation_date_new.u128() as u64,
        claim_date: block_time,
    }
}

/// current_staking_apr_per_vault = 100 % * eclip_per_second * (year_in_seconds / seconds_per_essence) *           \
/// * (staking_duration / total_essence)
pub fn calc_current_staking_apr_per_vault(
    vault_creation_date: u64,
    block_time: u64,
    eclip_per_second: u64,
    current_total_essence: Uint128,
    seconds_per_essence: Uint128,
) -> Decimal {
    if current_total_essence.is_zero() {
        return u128_to_dec(MAX_APR);
    }

    let staking_duration =
        calc_staking_period(vault_creation_date, block_time, seconds_per_essence);

    u128_to_dec(100 * eclip_per_second as u128)
        * (u128_to_dec(YEAR_IN_SECONDS as u128) / u128_to_dec(seconds_per_essence))
        * (u128_to_dec(staking_duration) / u128_to_dec(current_total_essence))
}

/// current_staking_apr = sum_over_vaults(vault_amount * current_staking_apr_per_vault) / sum_over_vaults(vault_amount)
pub fn calc_current_staking_apr(
    vaults: &[Vault],
    block_time: u64,
    eclip_per_second: u64,
    current_total_essence: Uint128,
    seconds_per_essence: Uint128,
) -> Decimal {
    let (amount_and_apr_product, amount) = vaults.iter().fold(
        (Decimal::zero(), Uint128::zero()),
        |(product_acc, amount_acc), vault| {
            let apr = calc_current_staking_apr_per_vault(
                vault.creation_date,
                block_time,
                eclip_per_second,
                current_total_essence,
                seconds_per_essence,
            );

            (
                product_acc + u128_to_dec(vault.amount) * apr,
                amount_acc + vault.amount,
            )
        },
    );

    if amount.is_zero() {
        return Decimal::zero();
    }

    amount_and_apr_product / u128_to_dec(amount)
}

/// expected_staking_apr = 100 % * eclip_per_second                                     \
/// * (eclip_per_second_multiplier ^ (year_in_seconds / decreasing_rewards_period))     \
/// * (year_in_seconds / seconds_per_essence) *                                         \
/// * (year_in_seconds / (total_essence + added_essence))
pub fn calc_expected_staking_apr(
    amount_to_add: Uint128,
    eclip_per_second: u64,
    eclip_per_second_multiplier: Decimal,
    expected_total_essence: Uint128,
    seconds_per_essence: Uint128,
) -> Decimal {
    let total_essence = expected_total_essence
        + amount_to_add * Uint128::from(YEAR_IN_SECONDS) / seconds_per_essence;

    if total_essence.is_zero() {
        return u128_to_dec(MAX_APR);
    }

    let periods = YEAR_IN_SECONDS / DECREASING_REWARDS_PERIOD;
    let eclip_per_second =
        u128_to_dec(eclip_per_second) * eclip_per_second_multiplier.pow(periods as u32);

    u128_to_dec(100u128)
        * eclip_per_second
        * (u128_to_dec(YEAR_IN_SECONDS as u128) / u128_to_dec(seconds_per_essence))
        * (u128_to_dec(YEAR_IN_SECONDS as u128) / u128_to_dec(total_essence))
}

/// current_locking_apr_per_tier = 100 % * eclip_per_second * (year_in_seconds / seconds_per_essence) *           \
/// * (lock_period_per_tier / (total_essence + added_essence))
pub fn calc_current_locking_apr_per_tier(
    amount_to_add: Uint128,
    eclip_per_second: u64,
    current_total_essence: Uint128,
    lock_period_per_tier: u64,
    seconds_per_essence: Uint128,
) -> Decimal {
    let total_essence = current_total_essence
        + amount_to_add * Uint128::from(lock_period_per_tier) / seconds_per_essence;

    if total_essence.is_zero() {
        return u128_to_dec(MAX_APR);
    }

    // basically, locking_apr = 100 * eclip_per_second * lock_period_per_tier / total_essence
    u128_to_dec(100 * eclip_per_second as u128)
        * (u128_to_dec(YEAR_IN_SECONDS as u128) / u128_to_dec(seconds_per_essence))
        * (u128_to_dec(lock_period_per_tier as u128) / u128_to_dec(total_essence))
}

/// expected_locking_apr_per_tier = 100 % * eclip_per_second                            \
/// * (eclip_per_second_multiplier ^ (year_in_seconds / decreasing_rewards_period))     \
/// * (year_in_seconds / seconds_per_essence) *                                         \
/// * (lock_period_per_tier / (total_essence + added_essence))
pub fn calc_expected_locking_apr_per_tier(
    amount_to_add: Uint128,
    eclip_per_second: u64,
    eclip_per_second_multiplier: Decimal,
    expected_total_essence: Uint128,
    lock_period_per_tier: u64,
    seconds_per_essence: Uint128,
) -> Decimal {
    let total_essence = expected_total_essence
        + amount_to_add * Uint128::from(lock_period_per_tier) / seconds_per_essence;

    if total_essence.is_zero() {
        return u128_to_dec(MAX_APR);
    }

    let periods = YEAR_IN_SECONDS / DECREASING_REWARDS_PERIOD;
    let eclip_per_second =
        u128_to_dec(eclip_per_second) * eclip_per_second_multiplier.pow(periods as u32);

    u128_to_dec(100u128)
        * eclip_per_second
        * (u128_to_dec(YEAR_IN_SECONDS as u128) / u128_to_dec(seconds_per_essence))
        * (u128_to_dec(lock_period_per_tier as u128) / u128_to_dec(total_essence))
}
