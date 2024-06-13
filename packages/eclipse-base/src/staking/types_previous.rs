use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    /// ECLIP
    pub staking_token: String,
    /// (tier, total rewards)
    /// Tiers: 1 month, 3 months, 6 months, 9 months, 365 days
    pub lock_schedule: Vec<(u64, u64)>,
    /// amount of time required to generate 1 essence for 1 ECLIP
    pub seconds_per_essence: Uint128,
    /// funds received from penalty will be sent to treasury
    pub dao_treasury_address: Addr,
    pub penalty_multiplier: Decimal,
}

#[cw_serde]
pub struct PaginationConfig {
    pagination_amount: u32,
    pub pagination_index: Option<Addr>,
}

impl PaginationConfig {
    pub fn new(pagination_amount: u32, pagination_index: &Option<Addr>) -> Self {
        Self {
            pagination_amount,
            pagination_index: pagination_index.to_owned(),
        }
    }

    pub fn get_amount(&self) -> u32 {
        let Self {
            pagination_amount, ..
        } = self.to_owned();

        pagination_amount
    }
}
