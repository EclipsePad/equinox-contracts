use cw_controllers::Admin;
use cw_storage_plus::Item;
use equinox_msg::voter::Config;

/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "eclipse_equinox_voter";
/// Owner of the contract who can update config or set new admin
pub const OWNER: Admin = Admin::new("owner");
pub const CONFIG: Item<Config> = Item::new("config");
