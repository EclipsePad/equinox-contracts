use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {
    /// ASTRO token address
    pub base_token: String,
    /// xASTRO token address
    pub xtoken: String,
    /// vxASTRO contract
    pub vxtoken: String,
    /// Astroport Staking contract
    pub staking_contract: String,
    /// Converter contact
    pub converter_contact: String,
    /// contract owner for update
    pub owner: String,
}

#[cw_serde]
pub struct UpdateConfig {
    /// ASTRO token address
    pub base_token: Option<String>,
    /// xASTRO token address
    pub xtoken: Option<String>,
    /// vxASTRO contract
    pub vxtoken: Option<String>,
    /// Astroport Staking contract
    pub staking_contract: Option<String>,
    /// Converter contract
    pub converter_contract: Option<String>,
    /// Gauge contract
    pub gauge_contract: Option<String>,
    /// Astroport Gauge contract
    pub astroport_gauge_contract: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// stake ASTRO from user
    Receive(Cw20ReceiveMsg),
    /// update config
    UpdateConfig { config: UpdateConfig },
    /// update owner
    UpdateOwner { owner: String },
    /// withdraw xASTRO
    Withdraw { amount: Uint128, recipient: String },
    /// withdraw bribe rewards
    WithdrawBribeRewards {},
    /// gauge vote
    PlaceVote {
        gauge: u64,
        votes: Option<Vec<Vote>>,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum QueryMsg {
    /// query config
    Config {},
    /// query owner
    Owner {},
    /// query total vxASTRO
    VotingPower {},
    /// query ASTRO/xASTRO ratio
    ConvertRatio {},
}

#[cw_serde]
pub enum Cw20HookMsg {
    Stake {},
}

#[cw_serde]
pub struct Vote {
    /// Option voted for.
    pub option: String,
    /// The weight of the power given to this vote
    pub weight: Decimal,
}
