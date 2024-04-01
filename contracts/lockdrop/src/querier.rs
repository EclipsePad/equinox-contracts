use astroport::staking::QueryMsg;
use cosmwasm_std::{Deps, StdResult, Uint128};
use cw20::{BalanceResponse, Cw20QueryMsg};

pub fn query_token_balance(
    deps: Deps,
    token: String,
    address: String,
) -> StdResult<BalanceResponse> {
    deps.querier
        .query_wasm_smart(token, &Cw20QueryMsg::Balance { address })
}

pub fn query_total_deposit_astro_staking(deps: Deps, astro_staking: String) -> StdResult<Uint128> {
    deps.querier
        .query_wasm_smart(astro_staking, &QueryMsg::TotalDeposit {})
}

pub fn query_total_shares_astro_staking(deps: Deps, astro_staking: String) -> StdResult<Uint128> {
    deps.querier
        .query_wasm_smart(astro_staking, &QueryMsg::TotalShares {})
}
