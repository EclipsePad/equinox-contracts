use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Decimal, Uint128};

use crate::assets::{Currency, Token};

#[cw_serde]
pub struct AddressConfig {
    // Owner of the contract who can update config or set new admin
    pub admin: Addr,
    // Vesting Contract
    pub vesting: Option<Addr>,
    // Owner of the IDO project who can claim funds
    pub client: Option<Addr>,
}

#[cw_serde]
pub struct FundConfig {
    // Tokens for fundraise, native supported
    pub fund_currency_list: Vec<Currency<Token>>,
    // Asset for distribution, native supported
    pub reward_currency: Currency<Token>,
    // Percentage of total fee client can claim
    pub client_fee_rate: Decimal,

    /************** Presale Params *************/
    // exchange_rate = fund_token_price / reward_token_price
    pub exchange_rate: Decimal,
    // fund tokens can be accpted only as round lots
    pub fund_lot: Uint128,
    // Max allocation for private round
    pub max_private_allocation: Uint128,
    // Max allocation for public round
    pub max_public_allocation: Uint128,
    // Total reward token amount, limits distributed amount
    pub total_rewards_amount: Uint128,
}

#[cw_serde]
pub struct DateConfig {
    // Private Presale Start Time
    pub private_start_time: u64,
    // Private Presale Period
    pub private_presale_period: u64,
    // Public Presale Start Time
    pub public_start_time: u64,
    // Public Presale Period
    pub public_presale_period: u64,
}

#[derive(Default)]
#[cw_serde]
pub struct SaleStats {
    /************** Status Info *************/
    // Reward token amount sold by private sale
    pub private_sold_amount: Uint128,
    // Reward token amount sold by public sale
    pub public_sold_amount: Uint128,
    // Participants count
    pub user_count: u64,
}

#[derive(Default)]
#[cw_serde]
pub struct ClaimStats {
    pub claimed_by_admin: Uint128,
    pub claimed_by_client: Uint128,
}

// TODO: remove evm
#[cw_serde]
pub struct AllocationItem<A: ToString, U> {
    pub neutron_address: A,
    pub evm_address: Option<String>,
    // Max allocation for this user in private presale
    pub private_allocation: U,
    // Max allocation for this user in public presale
    pub public_allocation: U,
}

#[derive(Default)]
#[cw_serde]
pub struct Allocation {
    pub evm_address: Option<String>,
    // Max allocation for this user in private presale
    pub private_allocation: Uint128,
    // Max allocation for this user in public presale
    pub public_allocation: Uint128,
}

#[derive(Default)]
#[cw_serde]
pub struct Participant {
    pub evm_address: Option<String>,
    pub funded_private: Uint128,
    pub funded_public: Uint128,
    pub vested_private: Uint128,
    pub vested_public: Uint128,
}

// ---------------- for migration ------------------------------
//
#[cw_serde]
pub struct FundConfigPrev {
    // Tokens for fundraise, native supported
    pub fund_currency_list: Vec<Currency<Token>>,
    // Asset for distribution, native supported
    pub reward_currency: Currency<Token>,
    // Percentage of total fee client can claim
    pub client_fee_rate: Decimal,

    /************** Presale Params *************/
    // exchange_rate = fund_token_price / reward_token_price
    pub exchange_rate: Decimal,
    // Max allocation for private round
    pub max_private_allocation: Uint128,
    // Max allocation for public round
    pub max_public_allocation: Uint128,
    // Total reward token amount, limits distributed amount
    pub total_rewards_amount: Uint128,
}
