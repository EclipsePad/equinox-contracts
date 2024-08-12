use cosmwasm_std::{Decimal, Uint128};

/// user_single_side_vault_eclip_astro_rewards = 0.8 * (user_single_side_vault_eclip_astro / total_single_side_vault_eclip_astro) *     \
/// \* eclip_astro_minted_by_voter * (xastro_to_astro_price_after / xastro_to_astro_price_before - 1)                                   \
/// total_single_side_vault_eclip_astro = (total_flexible_vault_eclip_astro + total_time_lock_vault_eclip_astro)
pub fn calc_user_single_side_vault_eclip_astro_rewards(
    user_single_side_vault_eclip_astro: Uint128,
    total_single_side_vault_eclip_astro: Uint128,
    eclip_astro_minted_by_voter: Uint128,
    xastro_to_astro_price_before: Decimal,
    xastro_to_astro_price_after: Decimal,
) -> Uint128 {
    if total_single_side_vault_eclip_astro.is_zero()
        || xastro_to_astro_price_after == xastro_to_astro_price_before
    {
        return Uint128::zero();
    }

    (str_to_dec("0.8")
        * (u128_to_dec(user_single_side_vault_eclip_astro)
            / u128_to_dec(total_single_side_vault_eclip_astro))
        * u128_to_dec(eclip_astro_minted_by_voter)
        * (xastro_to_astro_price_after / xastro_to_astro_price_before - Decimal::one()))
    .to_uint_floor()
}

pub fn str_to_dec(s: &str) -> Decimal {
    <Decimal as std::str::FromStr>::from_str(s).unwrap()
}

pub fn u128_to_dec<T>(num: T) -> Decimal
where
    Uint128: From<T>,
{
    Decimal::from_ratio(Uint128::from(num), Uint128::one())
}
