use cosmwasm_std::Uint128;
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};

use equinox_msg::flexible_staking::Config;

/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "eclipASTRO staking contract";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const OWNER: Admin = Admin::new("owner");
pub const CONFIG: Item<Config> = Item::new("config");
pub const STAKING: Map<&String, Uint128> = Map::new("staking");
pub const TOTAL_STAKING: Item<Uint128> = Item::new("total_staking");

pub const ALLOWED_USERS: Map<&String, bool> = Map::new("alllowed_users");
