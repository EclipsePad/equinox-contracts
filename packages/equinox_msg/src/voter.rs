use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};

pub const MAX_ESCROW_VOTING_LOCK_PERIOD: u64 = 2 * 365 * 24 * 3600;

#[cw_serde]
pub struct MigrateMsg {
    pub version: String,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// can execute permissioned actions
    pub worker_list: Option<Vec<String>>,

    /// to mint eclipASTRO
    pub eclipsepad_minter: Option<String>,
    /// to get cosmic essence info (staking v3)
    pub eclipsepad_staking: Option<String>,
    /// to get bribes for voting
    pub eclipsepad_tribute_market: Option<String>,

    /// to stake ASTRO and get xASTRO
    pub astroport_staking: Option<String>,
    /// to get proposal info
    pub astroport_assembly: Option<String>,
    /// to lock xASTRO and get voting power
    pub astroport_voting_escrow: Option<String>,
    /// TODO
    pub astroport_emission_controller: Option<String>,
    /// to get bribes for voting
    pub astroport_tribute_market: Option<String>,

    /// ASTRO denom
    pub astro: Option<String>,
    /// xASTRO denom
    pub xastro: Option<String>,
    /// vxASTRO address
    pub vxastro: Option<String>,
    /// eclipASTRO denom
    pub eclip_astro: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// accept admin role
    AcceptAdminRole {},

    /// update account/contract addresses related config
    UpdateAddressConfig {
        /// can update config
        admin: Option<String>,
        /// can execute permissioned actions
        worker_list: Option<Vec<String>>,

        /// to mint eclipASTRO
        eclipsepad_minter: Option<String>,
        /// to get cosmic essence info (staking v3)
        eclipsepad_staking: Option<String>,
        /// to get bribes for voting
        eclipsepad_tribute_market: Option<String>,

        /// to stake ASTRO and get xASTRO
        astroport_staking: Option<String>,
        /// to get proposal info
        astroport_assembly: Option<String>,
        /// to lock xASTRO and get voting power
        astroport_voting_escrow: Option<String>,
        /// TODO
        astroport_emission_controller: Option<String>,
        /// to get bribes for voting
        astroport_tribute_market: Option<String>,
    },

    /// update token related config
    UpdateTokenConfig {
        /// ASTRO denom
        astro: Option<String>,
        /// xASTRO denom
        xastro: Option<String>,
        /// vxASTRO address
        vxastro: Option<String>,
        /// eclipASTRO denom
        eclip_astro: Option<String>,
    },

    CaptureEssence {
        user_and_essence_list: Vec<(String, Uint128)>,
        total_essence: Uint128,
    },

    /// a user can lock xASTRO for 2 years to get eclipASTRO and boost voting power for essence holders
    /// swap ASTRO -> xASTRO will be provided first if it's required
    SwapToEclipAstro {},

    Vote {
        voting_list: Vec<VotingListItem>,
    },

    /// withdraw bribe rewards
    ClaimRewards {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// query account/contract addresses related config
    #[returns(AddressConfig)]
    AddressConfig {},

    /// query token related config
    #[returns(TokenConfig)]
    TokenConfig {},

    /// query bribe rewards
    #[returns(Vec<(Uint128, String)>)]
    Rewards {},

    /// query vxASTRO based voting power
    #[returns(Uint128)]
    VotingPower { address: String },

    /// query xASTRO/ASTRO ratio
    #[returns(Decimal)]
    XastroPrice {},

    #[returns(astroport_governance::emissions_controller::hub::UserInfoResponse)]
    VoterInfo { address: String },
}

#[cw_serde]
pub struct VotingListItem {
    pub lp_token: String,
    pub voting_power: Decimal,
}

#[cw_serde]
pub struct Vote {
    /// Option voted for.
    pub option: String,
    /// The weight of the power given to this vote
    pub weight: Decimal,
}

#[cw_serde]
pub struct AddressConfig {
    /// can update config
    pub admin: Addr,
    /// can execute permissioned actions
    pub worker_list: Vec<Addr>,

    /// to mint eclipASTRO
    pub eclipsepad_minter: Addr,
    /// to get cosmic essence info (staking v3)
    pub eclipsepad_staking: Addr,
    /// to get bribes for voting
    pub eclipsepad_tribute_market: Option<Addr>,

    /// to stake ASTRO and get xASTRO
    pub astroport_staking: Addr,
    /// to get proposal info
    pub astroport_assembly: Addr,
    /// to lock xASTRO and get voting power
    pub astroport_voting_escrow: Addr,
    /// TODO
    pub astroport_emission_controller: Addr,
    /// to get bribes for voting
    pub astroport_tribute_market: Option<Addr>,
}

#[cw_serde]
pub struct TokenConfig {
    /// ASTRO denom
    pub astro: String,
    /// xASTRO denom
    pub xastro: String,
    /// vxASTRO address
    pub vxastro: Addr,
    /// eclipASTRO denom
    pub eclip_astro: String,
}
