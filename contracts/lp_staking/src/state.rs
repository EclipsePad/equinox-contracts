use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use equinox_msg::lp_staking::{Config, RewardConfig, TotalStaking, UserStaking};

/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "lp staking contract";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const OWNER: Admin = Admin::new("owner");
pub const CONFIG: Item<Config> = Item::new("config");
pub const STAKING: Map<&String, UserStaking> = Map::new("staking");
pub const TOTAL_STAKING: Item<TotalStaking> = Item::new("total_staking");
pub const REWARD_CONFIG: Item<RewardConfig> = Item::new("reward_config");

pub const LAST_CLAIMED: Item<u64> = Item::new("last_claimed");
