use cosmwasm_std::Addr;

use cw_storage_plus::{Item, Map};

use crate::whitelist::types::{Config, WhitelistEntry};

pub const CONTRACT_NAME: &str = "eclipsepad-whitelist";

pub const DEFAULT_NETWORK: &str = "neutron";
pub const WALLET_LIMIT: usize = 50;

pub const PAGINATION_MAX_LIMIT: u32 = 10_000;
pub const PAGINATION_DEFAULT_LIMIT: u32 = 1_000;

pub const CONFIG: Item<Config> = Item::new("config");

pub const WHITELIST: Map<&Addr, WhitelistEntry> = Map::new("whitelist entry by neutron address");
