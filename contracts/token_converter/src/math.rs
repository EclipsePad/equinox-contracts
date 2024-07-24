use cosmwasm_std::Uint128;

use crate::external_queriers::AstroStaking;

pub fn calculate_claimable(
    xastro: Uint128,
    astro: Uint128,
    total_shares: Uint128,
    total_deposit: Uint128,
    cliamed_xastro: Uint128,
) -> Uint128 {
    xastro // total xASTRO amount
        .multiply_ratio(total_deposit, total_shares) // total ASTRO amount when withdraw all
        .checked_sub(astro) // total deposited ASTRO amount
        .unwrap_or_default()
        .multiply_ratio(total_shares, total_deposit)
        .checked_sub(cliamed_xastro)
        .unwrap_or_default()
}

pub fn calculate_eclipastro_amount(rate: AstroStaking, xastro: Uint128) -> Uint128 {
    xastro.multiply_ratio(rate.total_deposit, rate.total_shares)
}
