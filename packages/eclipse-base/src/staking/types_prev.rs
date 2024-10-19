use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub beclip_minter: Option<Addr>,
    /// ECLIP
    pub staking_token: String,
    /// bECLIP
    pub beclip_address: Option<Addr>,
    /// whitelisted contracts that be able to convert eclip to beclip directly
    pub beclip_whitelist: Vec<Addr>,
    /// (tier, total rewards)
    /// Tiers: 1 month, 3 months, 6 months, 9 months, 365 days
    pub lock_schedule: Vec<(u64, u64)>,
    /// amount of time required to generate 1 essence for 1 ECLIP
    pub seconds_per_essence: Uint128,
    /// funds received from penalty will be sent to treasury
    pub dao_treasury_address: Addr,
    pub penalty_multiplier: Decimal,
    pub eclip_per_second: u64,
    pub eclip_per_second_multiplier: Decimal,
}
