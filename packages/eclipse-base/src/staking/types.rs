use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub equinox_voter: Option<Addr>,
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

#[cw_serde]
pub struct TransferAdminState {
    pub new_admin: Addr,
    pub deadline: u64,
}

#[cw_serde]
#[derive(Default)]
pub struct State {
    pub total_bond_amount: Uint128, // or total_locked_per_tier
    pub distributed_rewards_per_tier: u64,
}

#[cw_serde]
#[derive(Default)]
pub struct Vault {
    pub amount: Uint128,
    pub creation_date: u64, // for essence rewards
    pub claim_date: u64,    // for locking rewards
    pub accumulated_rewards: Uint128,
}

#[cw_serde]
#[derive(Default)]
pub struct LockerInfo {
    pub lock_tier: u64,
    pub vaults: Vec<Vault>,
}

#[cw_serde]
#[derive(Default)]
pub struct StakerInfo {
    pub vaults: Vec<Vault>,
}

#[cw_serde]
pub struct PaginationConfig {
    pagination_amount: u32,
    pub staking_pagination_index: Option<Addr>,
    pub locking_pagination_index: Option<Addr>,
}

impl PaginationConfig {
    pub fn new(
        pagination_amount: u32,
        staking_pagination_index: &Option<Addr>,
        locking_pagination_index: &Option<Addr>,
    ) -> Self {
        Self {
            pagination_amount,
            staking_pagination_index: staking_pagination_index.to_owned(),
            locking_pagination_index: locking_pagination_index.to_owned(),
        }
    }

    pub fn get_amount(&self) -> u32 {
        let Self {
            pagination_amount, ..
        } = self.to_owned();

        pagination_amount
    }
}

#[cw_serde]
pub struct AprInfoItem {
    pub staking_apr: Decimal,
    pub locking_apr_list: Vec<LockingAprItem>,
}

#[cw_serde]
pub struct LockingAprItem {
    pub tier: u64,
    pub apr: Decimal,
}
