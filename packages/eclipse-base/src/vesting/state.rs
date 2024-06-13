use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use crate::vesting::types::{State, UserInfo};

pub const CONTRACT_NAME: &str = "eclipsepad-vesting";

/// Pagination settings
pub const PAGINATION_MAX_LIMIT: u32 = 10_000;
pub const PAGINATION_DEFAULT_LIMIT: u32 = 1_000;

pub const ACCURACY: u64 = 1_000;

pub const REWARD_TOKEN: &str = "untrn";
pub const INITIAL_UNLOCK: &str = "0";
pub const DISTRIBUTION_AMOUNT: u128 = 0;

pub const START_TIME: u64 = 9_000_000_000;
pub const CLIFF: u64 = 0;
pub const VESTING_PERIOD: u64 = 0;

/// Owner address(presale or the vesting runner)
pub const OWNER: Item<Addr> = Item::new("owner");

/// Worker address(presale or the vesting runner)
pub const WORKER: Item<Addr> = Item::new("worker");

/// Global vesting state
pub const STATE: Item<State> = Item::new("state");

/// Stores recipients vesting info
pub const RECIPIENTS: Map<&Addr, UserInfo> = Map::new("recipients");
