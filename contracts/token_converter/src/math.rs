use cosmwasm_std::Uint128;

pub fn calculate_claimable(
    xtoken: Uint128,
    token: Uint128,
    total_xtoken: Uint128,
    total_token: Uint128,
    claimed: Uint128,
) -> Uint128 {
    xtoken
        .multiply_ratio(total_token, total_xtoken)
        .checked_sub(token)
        .unwrap_or_default()
        .multiply_ratio(total_xtoken, total_token)
        .checked_sub(claimed)
        .unwrap_or_default()
}

pub fn convert_token(a: Uint128, total_a: Uint128, total_b: Uint128) -> Uint128 {
    a.multiply_ratio(total_b, total_a)
}
