use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};

use super::types::{
    EssenceAllocationItem, EssenceInfo, RewardsInfo, RouteListItem, VoteResults,
    WeightAllocationItem,
};

#[cw_serde]
pub struct MigrateMsg {
    pub version: String,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// can execute permissioned actions
    pub worker_list: Option<Vec<String>>,

    /// to allocate delegated voting power
    pub eclipse_dao: String,
    /// to query darkECLIP holders essence info
    pub eclipsepad_foundry: Option<String>,
    /// to mint eclipASTRO
    pub eclipsepad_minter: String,
    /// to get cosmic essence info (staking v3)
    pub eclipsepad_staking: String,
    /// to get bribes for voting
    pub eclipsepad_tribute_market: Option<String>,

    /// to stake ASTRO and get xASTRO
    pub astroport_staking: String,
    /// to get proposal info
    pub astroport_assembly: String,
    /// to lock xASTRO and get voting power
    pub astroport_voting_escrow: String,
    /// TODO
    pub astroport_emission_controller: String,
    /// to sell rewards
    pub astroport_router: String,
    /// to get bribes for voting
    pub astroport_tribute_market: Option<String>,

    /// ECLIP denom
    pub eclip: String,
    /// ASTRO denom
    pub astro: String,
    /// xASTRO denom
    pub xastro: String,
    /// eclipASTRO denom
    pub eclip_astro: String,

    /// start date of 1st epoch
    pub genesis_epoch_start_date: u64,
    /// epoch duration
    pub epoch_length: u64,
    /// votes will be sent to astroport emissions controller by x/cron right after this delay
    pub vote_delay: u64,
}

#[cw_serde]
pub enum SudoMsg {
    // x/cron
    Vote {},

    Claim {},

    Swap {},
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

        /// to allocate delegated voting power
        eclipse_dao: Option<String>,
        /// to query darkECLIP holders essence info
        eclipsepad_foundry: Option<String>,
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
        /// to sell rewards
        astroport_router: Option<String>,
        /// to get bribes for voting
        astroport_tribute_market: Option<String>,
    },

    /// update token related config
    UpdateTokenConfig {
        /// ECLIP denom
        eclip: Option<String>,
        /// ASTRO denom
        astro: Option<String>,
        /// xASTRO denom
        xastro: Option<String>,
        /// eclipASTRO denom
        eclip_astro: Option<String>,
    },

    /// update date related config
    UpdateDateConfig {
        /// start date of 1st epoch
        genesis_epoch_start_date: Option<u64>,
        /// epoch duration
        epoch_length: Option<u64>,
        /// votes will be sent to astroport emissions controller by x/cron right after this delay
        vote_delay: Option<u64>,
    },

    UpdateEssenceAllocation {
        user_and_essence_list: Vec<(String, EssenceInfo)>,
        total_essence: EssenceInfo,
    },

    /// a user can lock xASTRO to get eclipASTRO and boost voting power for essence holders
    /// swap ASTRO -> xASTRO will be provided first if it's required
    SwapToEclipAstro {},

    SwapXastroToAstro {},

    Delegate {},

    Undelegate {},

    PlaceVote {
        weight_allocation: Vec<WeightAllocationItem>,
    },

    PlaceVoteAsDao {
        weight_allocation: Vec<WeightAllocationItem>,
    },

    /// withdraw bribe rewards
    ClaimRewards {},

    UpdateRouteList {
        route_list: Vec<RouteListItem>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// query account/contract addresses related config
    #[returns(super::types::AddressConfig)]
    AddressConfig {},

    /// query token related config
    #[returns(super::types::TokenConfig)]
    TokenConfig {},

    /// query date related config
    #[returns(super::types::DateConfig)]
    DateConfig {},

    /// query bribe rewards as [(amount, denom)]
    #[returns(Vec<(Uint128, String)>)]
    Rewards {},

    #[returns(Vec<super::types::BribesAllocationItem>)]
    BribesAllocation {},

    /// query vxASTRO based voting power
    #[returns(Uint128)]
    VotingPower { address: String },

    /// query xASTRO/ASTRO ratio
    #[returns(Decimal)]
    XastroPrice {},

    // #[returns(astroport_governance::emissions_controller::hub::UserInfoResponse)]
    // VoterInfo {
    //     address: String,
    // },

    // /// get user essence or total essence
    // #[returns(EssenceInfo)]
    // Essence { address: String },

    // #[returns(QueryEssenceListResponse)]
    // EssenceList {
    //     amount: u32,
    //     start_from: Option<String>,
    // },
    //
    #[returns(UserResponse)]
    User {
        address: String,
        block_time: Option<u64>,
    },

    #[returns(UserListResponse)]
    ElectorList {
        amount: u32,
        start_from: Option<String>,
    },

    #[returns(UserListResponse)]
    DelegatorList {
        amount: u32,
        start_from: Option<String>,
    },

    #[returns(UserListResponse)]
    SlackerList {
        amount: u32,
        start_from: Option<String>,
    },

    #[returns(DaoResponse)]
    DaoInfo { block_time: Option<u64> },

    #[returns(VoterInfoResponse)]
    VoterInfo { block_time: Option<u64> },

    #[returns(super::types::EpochInfo)]
    EpochInfo {},

    #[returns(Vec<super::types::RouteListItem>)]
    RouteList {
        amount: u32,
        start_from: Option<String>,
    },
}

#[cw_serde]
pub struct UserResponse {
    pub user_type: UserType,
    /// essence by user address
    pub essence_info: EssenceInfo,
    pub essence_value: Uint128,
    /// list of pools with weight allocations by user address
    pub weights: Vec<WeightAllocationItem>,
    /// rewards available to claim
    pub rewards: RewardsInfo,
}

#[cw_serde]
pub enum UserType {
    Elector,
    Delegator,
    Slacker,
}

#[cw_serde]
pub struct UserListResponse {
    pub block_time: u64,
    pub list: Vec<UserListResponseItem>,
}

#[cw_serde]
pub struct UserListResponseItem {
    pub address: Addr,
    /// essence info by user address
    pub essence_info: EssenceInfo,
    /// list of pools with weight allocations by user address, Some for Elector
    pub weights: Option<Vec<WeightAllocationItem>>,
}

#[cw_serde]
pub struct DaoResponse {
    /// essence by dao address, slakers are excluded
    pub essence_info: EssenceInfo,
    pub essence_value: Uint128,
    /// list of pools with weight allocations by dao address
    pub weights: Vec<WeightAllocationItem>,
}

#[cw_serde]
pub struct VoterInfoResponse {
    pub block_time: u64,
    /// list of pools with essence allocations for all electors
    pub elector_votes: Vec<EssenceAllocationItem>,
    /// sum essence info over all slackers
    pub slacker_essence_acc: EssenceInfo,
    /// total list of pools with essence allocations, slakers are excluded
    pub total_votes: Vec<EssenceAllocationItem>,
    /// historical data, 26 epochs max
    pub vote_results: Vec<VoteResults>,
}

#[cw_serde]
pub struct QueryEssenceListResponse {
    pub user_and_essence_list: Vec<(String, EssenceInfo)>,
    pub total_essence: EssenceInfo,
}
