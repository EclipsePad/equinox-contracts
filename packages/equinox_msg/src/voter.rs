use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};

pub const MAX_ESCROW_VOTING_LOCK_PERIOD: u64 = 2 * 365 * 24 * 3600;

#[cw_serde]
pub struct InstantiateMsg {
    /// ASTRO denom
    pub astro: String,
    /// xASTRO denom
    pub xastro: String,
    /// vxASTRO denom
    pub vxastro: String,

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
    /// ASTRO denom
    pub astro: Option<String>,
    /// xASTRO denom
    pub xastro: Option<String>,
    /// vxASTRO denom
    pub vxastro: Option<String>,

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
pub enum ExecuteMsg {
    /// update config
    UpdateConfig {
        config: UpdateConfig,
    },
    /// update owner
    UpdateOwner {
        owner: String,
    },
    /// withdraw bribe rewards
    WithdrawBribeRewards {},

    /// a user can lock xASTRO for 2 years to get eclipASTRO and boost voting power for essence holders
    /// swap ASTRO -> xASTRO will be provided first if it's required
    SwapToEclipAstro {},
    Vote {
        voting_list: Vec<VotingListItem>,
    },
    CaptureEssence {
        user_and_essence_list: Vec<(String, Uint128)>,
        total_essence: Uint128,
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
    // /// query total vxASTRO
    // #[returns(Uint128)]
    // VotingPower { address: String },
    /// query ASTRO/xASTRO ratio
    #[returns((Uint128, Uint128))]
    ConvertRatio {},
    // // #[returns(astroport_governance::emissions_controller::hub::UserInfoResponse)]
    // #[returns(astroport_governance::generator_controller::UserInfoResponse)]
    // VoterInfo { address: String },
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
    /// ASTRO denom
    pub astro: String,
    /// xASTRO denom
    pub xastro: String,
    /// vxASTRO denom
    pub vxastro: String,

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
