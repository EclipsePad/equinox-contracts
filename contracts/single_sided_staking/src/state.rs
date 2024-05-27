use cosmwasm_std::Uint128;
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};

use equinox_msg::single_sided_staking::{Config, RewardWeights, UserStaked};

/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "single sided staking contract";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const OWNER: Admin = Admin::new("owner");
pub const CONFIG: Item<Config> = Item::new("config");
// user staking info (address, duration, start_time)
pub const USER_STAKED: Map<(&String, u64, u64), UserStaked> = Map::new("user_staking");
pub const TOTAL_STAKING: Item<Uint128> = Item::new("total_staking");
pub const TOTAL_STAKING_BY_DURATION: Map<u64, Uint128> = Map::new("total_staking_by_duration");
// only allowed users can set amount when withdraw and relock
pub const ALLOWED_USERS: Map<&String, bool> = Map::new("allowed_users");

pub const LAST_CLAIM_TIME: Item<u64> = Item::new("last_claim_time");
pub const REWARD_WEIGHTS: Item<RewardWeights> = Item::new("reward_weights");
pub const PENDING_ECLIPASTRO_REWARDS: Map<u64, Uint128> = Map::new("pending_eclipastro_rewards");
