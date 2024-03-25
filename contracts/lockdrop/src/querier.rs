use cosmwasm_std::{Deps, StdResult};
use cw20::{BalanceResponse, Cw20QueryMsg};

pub fn query_token_balance(
    deps: Deps,
    token: String,
    address: String,
) -> StdResult<BalanceResponse> {
    deps.querier
        .query_wasm_smart(token, &Cw20QueryMsg::Balance { address })
}
