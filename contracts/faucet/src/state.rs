use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use crate::msg::Config;

pub const CONTRACT_NAME: &str = "crates.io:astro-faucet";

pub const CONFIG: Item<Config> = Item::new("config");

pub const LAST_CLAIMED: Map<&Addr, u64> = Map::new("last claimed");
