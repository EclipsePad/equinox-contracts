use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use equinox_msg::voter::types::BribesAllocationItem;

use crate::types::Config;

pub const CONTRACT_NAME: &str = "tribute-market-mocks";

pub const DAY: u64 = 86400;

// > VOTE_DELAY
pub const REWARDS_DISTRIBUTION_DELAY: u64 = DAY * 11;

pub const CONFIG: Item<Config> = Item::new("config");
pub const BRIBES_ALLOCATION: Item<Vec<BribesAllocationItem>> = Item::new("bribes_allocation");
pub const INSTANTIATION_DATE: Item<u64> = Item::new("date");
// rewards as (amount, denom) by user
pub const REWARDS: Map<&Addr, Vec<(Uint128, String)>> = Map::new("rewards");
