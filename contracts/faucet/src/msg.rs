use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub astro: Addr,
    pub astro_staking: Addr,
    pub xastro: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    Claim {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(LastClaimedResponse)]
    LastClaimed { addr: String },
}

#[cw_serde]
pub struct MigrateMsg {
}

#[cw_serde]
pub struct Config {
    pub astro: Addr,
    pub astro_staking: Addr,
    pub xastro: Addr,
    pub daily_amount: Uint128,
}

#[cw_serde]
pub struct LastClaimedResponse {
    pub last_claim_at: u64,
}
