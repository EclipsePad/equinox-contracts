use cosmwasm_schema::{cw_serde, QueryResponses};

use nois::NoisCallback;

#[cw_serde]
pub struct MigrateMsg {
    pub version: String,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub proxy: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    // proxy
    NoisReceive {
        callback: NoisCallback,
    },

    // admin
    RequestRandomNumberList {
        length: u32,
    },

    StoreWallets {
        address_and_tickets_list: Vec<(String, u32)>,
    },

    UpdateConfig {
        admin: Option<String>,
        proxy: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::lottery::types::Config)]
    QueryConfig {},

    #[returns(String)]
    QueryJobId {},

    #[returns(Vec<(cosmwasm_std::Addr, u32)>)]
    QueryStoredWallets {
        start_after: Option<u32>,
        limit: Option<u32>,
    },

    #[returns(Vec<cosmwasm_std::Addr>)]
    QueryRandomWallets {
        start_after: Option<u32>,
        limit: Option<u32>,
    },

    #[returns(Vec<(cosmwasm_std::Addr, cosmwasm_std::Uint128)>)]
    QueryAllocations {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}
