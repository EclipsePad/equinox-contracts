use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    pub astro_token: String,
    pub xastro_token: String,
    pub astro_generator: Addr,
    pub staking_contract: Addr,
    // pub lp_contract: Addr,
    // pub lp_token: Addr,
    // pub converter: Addr,
    // pub eclipastro_token: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateOwner { owner: Addr },
    UpdateConfig { config: UpdateConfig },
    Claim {},
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
pub struct MigrateMsg {}

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

#[cw_serde]
pub struct UpdateConfig {
    pub astro_token: Option<String>,
    pub xastro_token: Option<String>,
    // pub eclipastro_token: Option<Addr>,
    // pub lp_token: Option<Addr>,
    pub astro_generator: Option<Addr>,
    pub staking_contract: Option<Addr>,
    // pub lp_contract: Option<Addr>,
    // pub converter: Option<Addr>,
}
