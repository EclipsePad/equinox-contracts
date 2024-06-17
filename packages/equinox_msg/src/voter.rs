use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

pub const MAX_ESCROW_VOTING_LOCK_PERIOD: u64 = 2 * 365 * 24 * 3600;

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
    /// Converter contract
    pub converter_contract: String,
    /// contract owner for update
    pub owner: String,

    /// Astroport Voting Escrow contract
    pub astroport_voting_escrow_contract: String,
    /// Astroport generator controller contract
    pub astroport_generator_controller: String,
    /// Eclipsepad staking v3 contract
    pub eclipsepad_staking_contract: String,
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

    /// Astroport Voting Escrow contract
    pub astroport_voting_escrow_contract: Option<String>,
    /// Astroport generator controller contract
    pub astroport_generator_controller: Option<String>,
    /// Eclipsepad staking v3 contract
    pub eclipsepad_staking_contract: Option<String>,
}

#[cw_serde]
pub enum Cw20HookMsg {
    Stake {},

    /// a user can lock xASTRO for 2 years to get eclipASTRO and boost voting power for essence holders
    /// swap ASTRO -> xASTRO will be provided first if it's required
    SwapToEclipAstro {},
}

#[cw_serde]
pub enum ExecuteMsg {
    /// stake ASTRO from user
    Receive(Cw20ReceiveMsg),
    /// update config
    UpdateConfig {
        config: UpdateConfig,
    },
    /// update owner
    UpdateOwner {
        owner: String,
    },
    /// withdraw xASTRO
    Withdraw {
        amount: Uint128,
        recipient: String,
    },
    /// withdraw bribe rewards
    WithdrawBribeRewards {},
    /// gauge vote
    PlaceVote {
        gauge: u64,
        votes: Option<Vec<Vote>>,
    },

    Vote {
        voting_list: Vec<VotingListItem>,
    },
}

#[cw_serde]
pub struct VotingListItem {
    pub lp_token: String,
    pub voting_power: Decimal,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// query config
    #[returns(Config)]
    Config {},
    /// query owner
    #[returns(Addr)]
    Owner {},
    /// query total vxASTRO
    #[returns(Uint128)]
    VotingPower { address: String },
    /// query ASTRO/xASTRO ratio
    #[returns((Uint128, Uint128))]
    ConvertRatio {},

    #[returns(astroport_governance::generator_controller::UserInfoResponse)]
    VoterInfo { address: String },
}

#[cw_serde]
pub struct Vote {
    /// Option voted for.
    pub option: String,
    /// The weight of the power given to this vote
    pub weight: Decimal,
}

#[cw_serde]
pub struct Config {
    /// ASTRO token address
    pub base_token: Addr,
    /// xASTRO token address
    pub xtoken: Addr,
    /// vxASTRO contract
    pub vxtoken: Addr,
    /// Astroport Staking contract
    pub staking_contract: Addr,
    /// Converter contract
    pub converter_contract: Addr,
    /// Gauge contract
    pub gauge_contract: Addr,
    /// Astroport Gauge contract
    pub astroport_gauge_contract: Addr,

    /// Astroport voting escrow contract
    pub astroport_voting_escrow_contract: Addr,
    /// Astroport generator controller contract
    pub astroport_generator_controller: Addr,
    /// Eclipsepad staking v3 contract
    pub eclipsepad_staking_contract: Addr,
}
