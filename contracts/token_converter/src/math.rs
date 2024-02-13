use cosmwasm_std::{Uint128, Uint256};

pub fn calculate_claimable(
    xtoken: Uint128,
    token: Uint128,
    total_xtoken: Uint128,
    total_token: Uint128,
    claimed: Uint128,
) -> Uint128 {
    let result = Uint256::from_uint128(token).multiply_ratio(total_xtoken, total_token);
    xtoken
        .checked_sub(Uint128::try_from(result).unwrap())
        .unwrap()
        .checked_sub(claimed)
        .unwrap()
}

pub fn convert_token(a: Uint128, total_a: Uint128, total_b: Uint128) -> Uint128 {
    let result = Uint256::from_uint128(a).multiply_ratio(total_b, total_a);
    Uint128::try_from(result).unwrap()
}
