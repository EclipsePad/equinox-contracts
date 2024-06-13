use cosmwasm_schema::{cw_serde, QueryResponses};

use crate::whitelist::types::{WalletEntry, WhitelistEntry};

#[cw_serde]
pub struct InstantiateMsg {
    pub staking_contract: Option<String>,
    pub is_registration_enabled: Option<bool>,
}

#[cw_serde]
pub struct MigrateMsg {
    pub version: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    // user when registration is enabled
    AddUser {
        wallets: Vec<WalletEntry>,
        is_kyc_passed: bool,
        user_info: String,
    },

    // user when registration is enabled
    UpdateWalletEntry {
        network: String,
        address: String,
    },

    // admin at any time
    UpdateUser {
        default_address: String,
        wallets: Vec<WalletEntry>,
        is_kyc_passed: bool,
    },

    RemoveUser {
        default_address: String,
    },

    // add new users or rewrite user data completely
    AddUserList {
        users: Vec<WhitelistEntry>,
    },

    // to update only kyc status and wallets
    UpdateUserList {
        users: Vec<WhitelistEntry>,
    },

    RemoveUserList {
        default_address_list: Vec<String>,
    },

    UpdateConfig {
        admin: Option<String>,
        staking_contract: Option<String>,
        is_registration_enabled: Option<bool>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::whitelist::types::Config)]
    QueryConfig {},

    #[returns(Vec<crate::whitelist::types::WhitelistEntry>)]
    QueryUserList {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(crate::whitelist::types::WhitelistEntry)]
    QueryUser { address: String },
}
