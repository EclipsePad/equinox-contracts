use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_asset::{AssetInfo, AssetInfoUnchecked};

use crate::vesting::types::UserInfo;

#[cw_serde]
pub struct MigrateMsg {
    pub version: String,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub worker: Option<String>,

    pub reward_token: Option<AssetInfoUnchecked>,
    pub initial_unlock: Option<Decimal>,
    pub distribution_amount: Option<Uint128>,

    pub start_time: Option<u64>,
    pub cliff: Option<u64>,
    pub vesting_period: Option<u64>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Set distribution amount
    UpdateDistributionAmount {
        amount: Uint128,
    },

    /// Update recipient info
    UpdateRecipient {
        recp: String,
        amount: Uint128,
    },

    /// Add in bulk
    BulkUpdateRecipients {
        list: Vec<(String, Uint128)>,
    },

    /// Update in bulk
    InplaceUpdate {
        list: Vec<(String, Uint128)>,
    },

    /// Withdraw unsold tokens
    WithdrawUnSoldToken {
        receiver: String,
    },

    // --------- new interfaces -------------------
    //
    ClaimBeforeVesting {},

    ClaimInVesting {},

    UpdateAddressConfig {
        owner: Option<String>,
        worker: Option<String>,
    },

    UpdateFundConfig {
        reward_token: Option<AssetInfoUnchecked>,
        initial_unlock: Option<Decimal>,
        distribution_amount: Option<Uint128>,
    },

    UpdateDateConfig {
        start_time: Option<u64>,
        cliff: Option<u64>,
        vesting_period: Option<u64>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(UsersCountResponse)]
    UsersCount {},

    #[returns(GetUsersResponse)]
    GetUsers {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(GetUserResponse)]
    GetUser { user: String },

    #[returns(ConfigResponse)]
    Config {},

    // --------- new interfaces -------------------
    //
    #[returns(QueryBalancesResponse)]
    QueryBalances { user: String },

    #[returns(QueryAddressConfigResponse)]
    QueryAddressConfig {},

    #[returns(QueryFundConfigResponse)]
    QueryFundConfig {},

    #[returns(QueryDateConfigResponse)]
    QueryDateConfig {},
}

#[cw_serde]
pub struct UsersCountResponse {
    pub count: u64,
}

#[cw_serde]
pub struct GetUsersResponse {
    pub users: Vec<(Addr, UserInfo)>,
}

#[cw_serde]
pub struct GetUserResponse {
    pub data: UserInfo,
}

#[cw_serde]
pub struct ConfigResponse {
    pub start_time: u64,
    pub release_interval: u64,
    pub release_rate: u64,
    pub initial_unlock: u64,
    pub lock_period: u64,
    pub vesting_period: u64,
    pub reward_token: AssetInfo,
    pub distribution_amount: Uint128,
    pub owner: Option<Addr>,
    pub worker: Option<Addr>,
    pub total_vesting_amount: Uint128,
}

#[cw_serde]
pub struct QueryBalancesResponse {
    pub total_amount: Uint128,
    // The amount that has been withdrawn from initially unlocked funds
    pub withdrawn_amount: Uint128,
    // The amount that has been withdrawn during vesting
    pub vesting_withdrawn_amount: Uint128,
    pub initially_unlocked_funds: Uint128,
    pub claimable_before_vesting: Uint128,
    pub claimable_in_vesting: Uint128,
    pub total_claimable: Uint128,
}

#[cw_serde]
pub struct QueryAddressConfigResponse {
    pub owner: Addr,
    pub worker: Addr,
}

#[cw_serde]
pub struct QueryFundConfigResponse {
    pub reward_token: AssetInfo,
    pub initial_unlock: Decimal,
    pub distribution_amount: Uint128,
}

#[cw_serde]
pub struct QueryDateConfigResponse {
    pub start_time: u64,
    pub cliff: u64,
    pub vesting_period: u64,
}
