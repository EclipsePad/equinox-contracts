use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

use crate::staking::types::{AprInfoItem, LockerInfo, PaginationConfig, StakerInfo, State};

#[cw_serde]
pub struct MigrateMsg {
    pub version: String,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub equinox_voter: Option<String>,
    pub beclip_minter: Option<String>,
    /// ECLIP
    pub staking_token: Option<String>,
    /// bECLIP
    pub beclip_address: Option<String>,
    /// whitelisted contracts that be able to convert eclip to beclip directly
    pub beclip_whitelist: Option<Vec<String>>,
    /// (tier, total rewards)
    /// Tiers: 3 months, 6 months, 9 months, 365 days
    pub lock_schedule: Option<Vec<(u64, u64)>>,
    /// amount of time required to generate 1 essence for 1 ECLIP
    pub seconds_per_essence: Option<Uint128>,
    /// funds received from penalty will be sent to treasury
    pub dao_treasury_address: Option<String>,
    pub penalty_multiplier: Option<Decimal>,
    pub pagintaion_config: Option<PaginationConfig>,
    pub eclip_per_second: Option<u64>,
    pub eclip_per_second_multiplier: Option<Decimal>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),

    Stake {},

    Unstake {},

    Lock {
        amount: Uint128,
        lock_tier: u64,
    },

    Unlock {},

    Relock {
        vault_creation_date: u64,
        from_tier: u64,
        to_tier: u64,
    },

    Withdraw {
        vault_creation_date: u64,
    },

    Bond {
        vault_creation_date_list: Vec<u64>,
    },

    BondFor {
        address_and_amount_list: Vec<(String, Uint128)>,
    },

    Unbond {},

    Claim {},

    AggregateVaults {
        tier: Option<u64>,
        vault_creation_date_list: Vec<u64>,
    },

    AcceptAdminRole {},

    UpdateConfig {
        admin: Option<String>,
        equinox_voter: Option<String>,
        beclip_minter: Option<String>,
        beclip_address: Option<String>,
        beclip_whitelist: Option<Vec<String>>,
        lock_schedule: Option<Vec<(u64, u64)>>,
        dao_treasury_address: Option<String>,
        penalty_multiplier: Option<Decimal>,
        eclip_per_second_multiplier: Option<Decimal>,
    },

    UpdatePaginationConfig {
        pagination_amount: Option<u32>,
    },

    DecreaseBalance {
        amount: Uint128,
    },

    Pause {},

    Unpause {},

    // temporary
    UpdateStakingEssenceStorages {
        amount: u32,
        start_from: Option<String>,
    },

    UpdateLockingEssenceStorages {
        amount: u32,
        start_from: Option<String>,
    },

    FilterStakers {
        amount: u32,
        start_from: Option<String>,
    },

    FilterLockers {
        amount: u32,
        start_from: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::staking::types::Config)]
    QueryConfig {},

    #[returns(crate::staking::types::PaginationConfig)]
    QueryPaginationConfig {},

    #[returns(StateResponse)]
    QueryState {},

    #[returns(StakerInfoResponse)]
    QueryStakerInfo { staker: String },

    #[returns(UsersAmountResponse)]
    QueryUsersAmount {},

    #[returns(crate::staking::types::Vault)]
    QueryAggregatedVault {
        user: String,
        tier: Option<u64>,
        vault_creation_date_list: Vec<u64>,
    },

    #[returns(QueryBalancesResponse)]
    QueryBalances {},

    #[returns(QueryEssenceResponse)]
    QueryEssence { user: String },

    #[returns(QueryEssenceResponse)]
    QueryTotalEssence {},

    #[returns(Vec<Uint128>)]
    QueryWalletsPerTier {},

    #[returns(Vec<QueryEssenceListResponseItem>)]
    QueryStakingEssenceList {
        amount: u32,
        start_from: Option<String>,
        block_time: u64,
    },

    #[returns(Vec<QueryEssenceListResponseItem>)]
    QueryLockingEssenceList {
        amount: u32,
        start_from: Option<String>,
    },

    #[returns(QueryStorageVolumesResponse)]
    QueryStorageVolumes {},

    #[returns(QueryAprInfoResponse)]
    QueryAprInfo {
        amount_to_add: Option<Uint128>,
        staker_address: Option<String>,
    },

    #[returns(Vec<(Addr, StakerInfo)>)]
    QueryStakerInfoList {
        amount: u32,
        start_from: Option<String>,
    },

    #[returns(Vec<(Addr, Vec<LockerInfo>)>)]
    QueryLockerInfoList {
        amount: u32,
        start_from: Option<String>,
    },

    #[returns(u64)]
    QueryTimeUntilDecreasingRewards {},

    #[returns(QueryRewardsReductionInfoResponse)]
    QueryRewardsReductionInfo {},

    #[returns(bool)]
    QueryPauseState {},

    #[returns(u64)]
    QueryBondedVaultCreationDate { user: String },

    #[returns(Uint128)]
    QueryBeclipSupply {},
}

#[cw_serde]
pub struct StateResponse {
    pub stake_state: State,
    pub lock_states: Vec<State>,
}

#[cw_serde]
pub struct EssenceAndRewardsInfo {
    pub staking_essence: Uint128,
    pub locking_essence: Uint128,
    pub essence: Uint128,
    pub staking_rewards: Uint128,
    pub locking_rewards: Uint128,
    pub rewards: Uint128,
    pub penalty: Uint128,
}

#[cw_serde]
pub struct StakerInfoResponse {
    pub staker: Addr,
    pub staker_info: StakerInfo,
    pub locker_infos: Vec<LockerInfo>,
    pub staking_vaults_info: Vec<EssenceAndRewardsInfo>,
    pub locking_vaults_info: Vec<(u64, Vec<EssenceAndRewardsInfo>)>,
    pub essence_and_rewards_info: EssenceAndRewardsInfo,
    pub funds_to_unstake: Uint128,
    pub funds_to_unlock: Uint128, // without penalty
    pub block_time: u64,
}

#[cw_serde]
pub struct UsersAmountResponse {
    pub stakers_only: Uint128,
    pub lockers_only: Uint128,
    pub stakers_and_lockers: Uint128,
    pub total: Uint128,
}

#[cw_serde]
pub struct QueryBalancesResponse {
    /// actual size of rewards pool
    pub rewards_pool: Uint128,
    /// sum of accumulated rewards over all users, should be provided on request only
    pub unclaimed: Uint128,
}

#[cw_serde]
pub struct QueryEssenceResponse {
    pub staking_essence_components: (Uint128, Uint128),
    pub staking_essence: Uint128,
    pub locking_essence: Uint128,
    pub essence: Uint128,
}

#[cw_serde]
pub struct QueryEssenceListResponseItem {
    pub user: Addr,
    pub essence: Uint128,
}

#[cw_serde]
pub struct QueryStorageVolumesResponse {
    pub staker_info: u16,
    pub locker_info: u16,
}

#[cw_serde]
pub struct QueryRewardsReductionInfoResponse {
    pub eclip_per_second: u64,
    pub decreasing_rewards_date: u64,
}

#[cw_serde]
pub struct QueryAprInfoResponse {
    pub current: AprInfoItem,
    pub expected: AprInfoItem,
}
