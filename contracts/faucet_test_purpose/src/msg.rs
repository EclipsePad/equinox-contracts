use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    pub astro_token: String,
    pub xastr_token: String,
    pub astro_generator: Addr,
    pub staking_contract: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateOwner { owner: Addr },
    Claim {},
    Mint {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},

    #[returns(LastClaimedResponse)]
    LastClaimed { addr: String },

    #[returns(Addr)]
    Owner {},
}

#[cw_serde]
pub struct MigrateMsg {
    pub version: String,
}

#[cw_serde]
pub struct LastClaimedResponse {
    pub last_claim_at: u64,
}

#[cw_serde]
pub struct ConfigResponse {
    pub astro_token: String,
    pub xastro_token: String,
    pub astro_generator: Addr,
    pub staking_contract: Addr,
}