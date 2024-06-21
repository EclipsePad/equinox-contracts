use cosmwasm_std::Addr;
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};

pub const CONTRACT_NAME: &str = "crates.io:eclip-faucet";

pub const TOKEN: Item<String> = Item::new("token");

pub const LAST_CLAIMED: Map<&Addr, u64> = Map::new("last claimed");

pub const OWNER: Admin = Admin::new("owner");
