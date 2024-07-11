use cosmwasm_std::Uint128;
use cw_storage_plus::Item;

pub const CONTRACT_NAME: &str = "tribute-market-mocks";

pub const DAY: u64 = 86400;

// > VOTE_DELAY
pub const REWARDS_DISTRIBUTION_DELAY: u64 = DAY * 11;
pub const REWARDS_DIVIDER: u128 = 10;

pub const INSTANTIATION_DATE: Item<u64> = Item::new("date");
// (denom, amount)
pub const REWARDS: Item<Vec<(String, Uint128)>> = Item::new("rewards");
pub const CLAIMABLE_REWARDS_PER_TX: Item<Vec<(String, Uint128)>> = Item::new("claimable rewards");
