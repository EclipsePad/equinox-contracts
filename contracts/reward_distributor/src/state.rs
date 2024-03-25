use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal256, Uint128};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use equinox_msg::reward_distributor::{Config, TotalStakingData};

// reward_weight = rewards / total_staking
// Decimal256      Uint128   Uin128
// ~ 10^(77-18)       3 * 10^38
// 59 - 38 = 21 / 2 = 10
pub const REWARD_WEIGHT_MULTIPLIER: u128 = 10_000_000_000;
pub const REWARD_DISTRIBUTION_PERIOD: u64 = 8 * 86_400; // 8 days
pub const REWARD_DISTRIBUTION_TIME_DIFF: u64 = 6 * 3_600; // 6 hours

/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "eclipASTRO staking contract";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const OWNER: Admin = Admin::new("owner");
pub const CONFIG: Item<Config> = Item::new("config");
// user staking data, reward weight for eclipASTRO, reward weight for ECLIP
pub const TOTAL_STAKING: Item<TotalStakingData> = Item::new("total_staking");
pub const TIMELOCK_USER_STAKING: Map<(&String, u64, u64), UserStakingData> =
    Map::new("timelock_user_staking");
pub const FLEXIBLE_USER_STAKING: Map<&String, UserStakingData> = Map::new("flexible_user_staking");
pub const LAST_UPDATE_TIME: Item<u64> = Item::new("last_update_time");
// start time, end time
pub const PENDING_REWARDS: Map<u64, Uint128> = Map::new("pending_rewards");

#[cw_serde]
#[derive(Default)]
pub struct UserRewards {
    pub eclipastro: UserReward,
    pub eclip: UserReward,
}

#[cw_serde]
pub struct UserReward {
    pub reward_weight: Decimal256,
    pub pending_reward: Uint128,
}

impl Default for UserReward {
    fn default() -> Self {
        UserReward {
            reward_weight: Decimal256::zero(),
            pending_reward: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct UserStakingData {
    pub amount: Uint128,
    pub rewards: UserRewards,
}

impl Default for UserStakingData {
    fn default() -> Self {
        UserStakingData {
            amount: Uint128::zero(),
            rewards: UserRewards::default(),
        }
    }
}
