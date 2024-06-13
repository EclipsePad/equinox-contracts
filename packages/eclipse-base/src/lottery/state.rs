use cw_storage_plus::Item;

use cosmwasm_std::Addr;

use crate::lottery::types::Config;

pub const CONTRACT_NAME: &str = "eclipsepad-lottery";

pub const PAGINATION_MAX_LIMIT: u32 = 10_000;
pub const PAGINATION_DEFAULT_LIMIT: u32 = 1_000;

pub const CONFIG: Item<Config> = Item::new("config");
pub const JOB_ID: Item<String> = Item::new("job id");
pub const RANDOM_AMOUNT: Item<u32> = Item::new("random amount");
pub const WALLETS: Item<Vec<(Addr, u32)>> = Item::new("address and tickets list");
pub const RANDOM_WALLETS: Item<Vec<Addr>> = Item::new("random wallets list");
