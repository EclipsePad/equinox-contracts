use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};

use eclipse_base::converters::{str_to_dec, u128_to_dec};

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
pub struct RouteItem {
    pub denom_in: String,
    pub denom_out: String,
}

impl RouteItem {
    pub fn new(denom_in: impl ToString, denom_out: impl ToString) -> Self {
        Self {
            denom_in: denom_in.to_string(),
            denom_out: denom_out.to_string(),
        }
    }
}

#[cw_serde]
pub struct RouteListItem {
    pub denom: String,
    pub route: Vec<RouteItem>,
}

impl RouteListItem {
    pub fn new(denom: impl ToString, route: &[RouteItem]) -> Self {
        Self {
            denom: denom.to_string(),
            route: route.to_owned(),
        }
    }
}

#[cw_serde]
#[derive(Default)]
pub struct EssenceAllocationItem {
    pub lp_token: String,
    pub essence_info: EssenceInfo,
}

impl EssenceAllocationItem {
    pub fn new(lp_token: impl ToString, essence_info: &EssenceInfo) -> Self {
        Self {
            lp_token: lp_token.to_string(),
            essence_info: essence_info.to_owned(),
        }
    }
}

#[cw_serde]
#[derive(Default)]
pub struct RewardsInfo {
    pub amount: Uint128,
    pub last_update_epoch: u16,
}

#[cw_serde]
#[derive(Default)]
pub struct EssenceInfo {
    pub staking_components: (Uint128, Uint128),
    pub locking_amount: Uint128,
}

impl EssenceInfo {
    pub fn new<T>(a: T, b: T, le: T) -> Self
    where
        Uint128: From<T>,
    {
        Self {
            staking_components: (Uint128::from(a), Uint128::from(b)),
            locking_amount: Uint128::from(le),
        }
    }

    /// self + item
    pub fn add(&self, item: &Self) -> Self {
        let (a1, b1) = self.staking_components;
        let (a2, b2) = item.staking_components;

        Self {
            staking_components: (a1 + a2, b1 + b2),
            locking_amount: self.locking_amount + item.locking_amount,
        }
    }

    /// self - item
    pub fn sub(&self, item: &Self) -> Self {
        let (a1, b1) = self.staking_components;
        let (a2, b2) = item.staking_components;

        Self {
            staking_components: (a1 - a2, b1 - b2),
            locking_amount: self.locking_amount - item.locking_amount,
        }
    }

    /// k * self
    pub fn scale(&self, k: Decimal) -> Self {
        let (a, b) = self.staking_components;
        let le = self.locking_amount;

        Self {
            staking_components: (
                (k * u128_to_dec(a)).to_uint_floor(),
                (k * u128_to_dec(b)).to_uint_floor(),
            ),
            locking_amount: (k * u128_to_dec(le)).to_uint_floor(),
        }
    }

    pub fn is_zero(&self) -> bool {
        let (a, b) = self.staking_components;
        a.is_zero() && b.is_zero() && self.locking_amount.is_zero()
    }

    pub fn capture(&self, block_time: u64) -> Uint128 {
        let (a, b) = self.staking_components;
        let staking_amount = calc_staking_essence_from_components(a, b, block_time);
        staking_amount + self.locking_amount
    }
}

/// staking_essence_from_components = (a * block_time - b) / seconds_per_essence      \
/// where a = sum(staked_eclip_amount), b = sum(staked_eclip_amount * vault.creation_date)
fn calc_staking_essence_from_components(a: Uint128, b: Uint128, block_time: u64) -> Uint128 {
    const SECONDS_PER_ESSENCE: u128 = 31_536_000;
    const YEAR_IN_SECONDS: u64 = 31_536_000;

    std::cmp::min(
        a * Uint128::from(block_time) - b,
        a * Uint128::from(YEAR_IN_SECONDS),
    ) / Uint128::new(SECONDS_PER_ESSENCE)
}

#[cw_serde]
pub struct WeightAllocationItem {
    pub lp_token: String,
    pub weight: Decimal,
}

impl WeightAllocationItem {
    pub fn new(lp_token: impl ToString, weight: &str) -> Self {
        Self {
            lp_token: lp_token.to_string(),
            weight: str_to_dec(weight),
        }
    }
}

#[cw_serde]
pub struct PoolInfoItem {
    pub lp_token: String,
    pub weight: Decimal,
    pub rewards: Vec<(Uint128, String)>,
}

impl PoolInfoItem {
    pub fn new(lp_token: impl ToString, weight: &str, rewards: &[(u128, &str)]) -> Self {
        Self {
            lp_token: lp_token.to_string(),
            weight: str_to_dec(weight),
            rewards: rewards
                .into_iter()
                .map(|(amount, denom)| (Uint128::new(amount.to_owned()), denom.to_string()))
                .collect(),
        }
    }
}

#[cw_serde]
#[derive(Default)]
pub struct VoteResults {
    pub epoch_id: u16,
    pub end_date: u64,
    /// essence used in voting                                              \
    /// dao w/o electors: delegated_essence + 0.2 * slacker_essence         \
    /// dao w/o delegators: elector_essence + 0.8 * slacker_essence         \
    /// dao w/o both electors and delegators: 0.2 * slacker_essence
    pub essence: Uint128,
    pub dao_essence: Uint128,
    /// 20 % self + 80 % delegators
    pub dao_eclip_rewards: Uint128,
    pub pool_info_list: Vec<PoolInfoItem>,
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

    /// query date related config
    #[returns(DateConfig)]
    DateConfig {},

    /// query bribe rewards as [(amount, denom)]
    #[returns(Vec<(Uint128, String)>)]
    Rewards {},

    #[returns(Vec<BribesAllocationItem>)]
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

    #[returns(EpochInfo)]
    EpochInfo {},

    #[returns(Vec<RouteListItem>)]
    RouteList {
        amount: u32,
        start_from: Option<String>,
    },
}

#[cw_serde]
pub enum UserResponse {
    Elector {
        /// essence by elector address, slakers are excluded
        essence_info: EssenceInfo,
        essence_value: Uint128,
        /// list of pools with weight allocations by elector address
        weights: Vec<WeightAllocationItem>,
    },
    Delegator {
        /// essence by delegator address
        essence_info: EssenceInfo,
        essence_value: Uint128,
    },
    Slacker {
        /// essence by slacker address
        essence_info: EssenceInfo,
        essence_value: Uint128,
    },
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

#[cw_serde]
pub struct BribesAllocationItem {
    pub lp_token: Addr,
    pub rewards: Vec<(Uint128, String)>,
}

impl BribesAllocationItem {
    pub fn new(lp_token: impl ToString, rewards: &[(u128, impl ToString)]) -> Self {
        Self {
            lp_token: Addr::unchecked(lp_token.to_string()),
            rewards: rewards
                .into_iter()
                .map(|(amount, denom)| (Uint128::new(amount.to_owned()), denom.to_string()))
                .collect(),
        }
    }
}

#[cw_serde]
pub struct AddressConfig {
    /// can update config
    pub admin: Addr,
    /// can execute permissioned actions
    pub worker_list: Vec<Addr>,

    /// to allocate delegated voting power
    pub eclipse_dao: Addr,
    /// to query darkECLIP holders essence info
    pub eclipsepad_foundry: Option<Addr>,
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
    /// to sell rewards
    pub astroport_router: Addr,
    /// to get bribes for voting
    pub astroport_tribute_market: Option<Addr>,
}

#[cw_serde]
pub struct TokenConfig {
    /// ASTRO denom
    pub astro: String,
    /// xASTRO denom
    pub xastro: String,
    /// eclipASTRO denom
    pub eclip_astro: String,
}

#[cw_serde]
pub struct DateConfig {
    /// start date of 1st epoch
    pub genesis_epoch_start_date: u64,
    /// epoch duration
    pub epoch_length: u64,
    /// votes will be sent to astroport emissions controller by x/cron right after this delay
    pub vote_delay: u64,
}

#[cw_serde]
pub struct EpochInfo {
    pub id: u16,
    pub start_date: u64,
}

#[cw_serde]
pub struct TransferAdminState {
    pub new_admin: Addr,
    pub deadline: u64,
}
