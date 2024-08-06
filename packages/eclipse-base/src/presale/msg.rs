use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Addr, Decimal, Uint128};

use crate::{
    assets::{Currency, TokenUnverified},
    presale::types::{AllocationItem, Participant},
};

#[cw_serde]
pub struct MigrateMsg {
    pub version: String,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub vesting: Option<String>,
    pub client: Option<String>,

    pub fund_currency_list: Option<Vec<Currency<TokenUnverified>>>,
    pub reward_currency: Option<Currency<TokenUnverified>>,
    pub client_fee_rate: Option<Decimal>,
    pub exchange_rate: Option<Decimal>,
    pub fund_lot: Option<Uint128>,
    pub max_private_allocation: Option<Uint128>,
    pub max_public_allocation: Option<Uint128>,
    pub total_rewards_amount: Option<Uint128>,

    pub private_start_time: Option<u64>,
    pub private_presale_period: Option<u64>,
    pub public_start_time: Option<u64>,
    pub public_presale_period: Option<u64>,
}

#[cw_serde]
pub enum ExecuteMsg {
    // user
    DepositPrivateSale {
        evm_address: Option<String>,
    },

    DepositPublicSale {
        evm_address: Option<String>,
    },

    // admin
    SetAllocations {
        address_and_allocation_list: Vec<AllocationItem<String, Option<Uint128>>>,
    },

    WithdrawFunds {
        receiver: String,
    },

    UpdateAddressConfig {
        admin: Option<String>,
        vesting: Option<String>,
        client: Option<String>,
    },

    UpdateFundConfig {
        fund_currency_list: Option<Vec<Currency<TokenUnverified>>>,
        reward_currency: Option<Currency<TokenUnverified>>,
        client_fee_rate: Option<Decimal>,
        exchange_rate: Option<Decimal>,
        fund_lot: Option<Uint128>,
        max_private_allocation: Option<Uint128>,
        max_public_allocation: Option<Uint128>,
        total_rewards_amount: Option<Uint128>,
    },

    UpdateDateConfig {
        private_start_time: Option<u64>,
        private_presale_period: Option<u64>,
        public_start_time: Option<u64>,
        public_presale_period: Option<u64>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::presale::types::SaleStats)]
    QuerySaleStats {},

    #[returns(crate::presale::types::ClaimStats)]
    QueryClaimStats {},

    #[returns(Vec<crate::presale::types::AllocationItem<Addr, Uint128>>)]
    QueryAllocationList {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(crate::presale::types::Allocation)]
    QueryAllocation { user: String },

    #[returns(Vec<QueryParticipantsResponseItem>)]
    QueryParticipantList {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(crate::presale::types::Participant)]
    QueryParticipant { user: String },

    #[returns(QueryTimeResponse)]
    QueryTime {},

    #[returns(crate::presale::types::AddressConfig)]
    QueryAddressConfig {},

    #[returns(crate::presale::types::FundConfig)]
    QueryFundConfig {},

    #[returns(crate::presale::types::DateConfig)]
    QueryDateConfig {},
}

#[cw_serde]
pub struct QueryParticipantsResponseItem {
    pub address: Addr,
    pub participant: Participant,
}

#[cw_serde]
pub struct QueryTimeResponse {
    pub block_time: u64,
    pub private_presale_start: u64,
    pub private_presale_end: u64,
    pub public_presale_start: u64,
    pub public_presale_end: u64,
}
