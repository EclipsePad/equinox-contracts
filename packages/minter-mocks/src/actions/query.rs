use cosmwasm_std::{Deps, Env, Order, StdResult};

use eclipse_base::minter::{
    msg::QueryCurrenciesFromCreatorResponse,
    state::{CONFIG, OWNERS},
    types::Config,
};

pub fn query_currencies_by_creator(
    deps: Deps,
    _env: Env,
    creator: String,
) -> StdResult<QueryCurrenciesFromCreatorResponse> {
    let creator = deps.api.addr_validate(&creator)?;
    let currencies = OWNERS
        .range(deps.storage, None, None, Order::Ascending)
        .flatten()
        .filter(|(_denom, (_currency, owner))| owner == creator)
        .map(|(_denom, (currency, _owner))| currency)
        .collect();

    Ok(QueryCurrenciesFromCreatorResponse { currencies })
}

pub fn query_config(deps: Deps, _env: Env) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}
