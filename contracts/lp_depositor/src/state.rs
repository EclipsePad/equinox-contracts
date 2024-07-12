use cw_storage_plus::Item;
use equinox_msg::lp_depositor::Config;

/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "lp_depositor";

/// Contract version that is used for migration.
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const CONFIG: Item<Config> = Item::new("config");
