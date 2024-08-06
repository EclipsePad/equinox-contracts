use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const CONTRACT_NAME: &str = "eclipsepad-faucet";

pub const CLAIMABLE_AMOUNT: u128 = 10_000_000_000;
pub const CLAIM_COOLDOWN: u64 = 3600;

pub const SUBDENOM: &str = "eclip";

pub const TOKEN: Item<String> = Item::new("token");

pub const LAST_CLAIMED: Map<&Addr, u64> = Map::new("last claimed");
