use astroport::staking::QueryMsg as StakingQueryMsg;
use cosmwasm_std::{Deps, StdResult, Uint128};

pub fn query_rates_astro_staking(deps: Deps, staking_contract: String) -> StdResult<AstroStaking> {
    let total_shares: Uint128 = deps.querier
        .query_wasm_smart(staking_contract.clone(), &StakingQueryMsg::TotalShares {}).unwrap();
    let total_deposit: Uint128 = deps.querier
        .query_wasm_smart(staking_contract, &StakingQueryMsg::TotalDeposit {}).unwrap();
    Ok(AstroStaking {
        total_shares,
        total_deposit
    })
}

pub struct AstroStaking {
    pub total_shares: Uint128,
    pub total_deposit: Uint128,
}