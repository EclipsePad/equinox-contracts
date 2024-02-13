use cosmwasm_std::Uint128;
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};

use equinox_msg::timelock_staking::Config;

/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "eclipASTRO staking contract";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const OWNER: Admin = Admin::new("owner");
pub const CONFIG: Item<Config> = Item::new("config");
// user staking info (address, duration, start_time)
pub const STAKING: Map<(&String, u64, u64), Uint128> = Map::new("staking");
pub const TOTAL_STAKING: Item<Uint128> = Item::new("total_staking");
pub const TOTAL_STAKING_BY_DURATION: Map<u64, Uint128> = Map::new("total_staking_by_duration");
pub const PENALTIES: Item<Uint128> = Item::new("penalties");
