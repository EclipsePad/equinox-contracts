use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateOwner { owner: Addr },
    Mint { amount: Uint128 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(String)]
    Denom {},

    #[returns(Addr)]
    Owner {},
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct LastClaimedResponse {
    pub last_claim_at: u64,
}
