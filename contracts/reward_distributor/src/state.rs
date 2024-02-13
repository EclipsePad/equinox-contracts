use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use equinox_msg::reward_distributor::Config;


/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "eclipASTRO staking contract";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const OWNER: Admin = Admin::new("owner");
pub const CONFIG: Item<Config> = Item::new("config");
// user staking data, reward weight for eclipASTRO, reward weight for ECLIP
pub const TOTAL_STAKING: Item<TotalStakingData> = Item::new("total_staking");
pub const USER_STAKING: Map<&String, Vec<StakingData>> = Map::new("user_staking");
pub const USER_REWARDS: Map<&String, UserRewards> = Map::new("user_rewards");
pub const LAST_UPDATE_TIME: Item<u64> = Item::new("last_update_time");

#[cw_serde]
pub struct StakingData {
    pub duration: u64,
    pub amount: Uint128,
}

#[cw_serde]
pub struct TotalStakingData {
    pub staking_data: Vec<StakingData>,
    pub reward_weight_eclipastro: Decimal,
    pub reward_weight_astro: Decimal,
}

impl Default for TotalStakingData {
    fn default() -> Self {
        TotalStakingData {
            staking_data: vec![],
            reward_weight_eclipastro: Decimal::zero(),
            reward_weight_astro: Decimal::zero(),
        }
    }
}

#[cw_serde]
pub struct UserRewards {
    pub eclipastro: UserReward,
    pub eclip: UserReward,
}

#[cw_serde]
pub struct UserReward {
    pub reward_weight: Decimal,
    pub pending_reward: Uint128,
}

impl Default for UserReward {
    fn default() -> Self {
        UserReward {
            reward_weight: Decimal::zero(),
            pending_reward: Uint128::zero(),
        }
    }
}

impl Default for UserRewards {
    fn default() -> Self {
        UserRewards {
            eclipastro: UserReward::default(),
            eclip: UserReward::default(),
        }
    }
}
