use cosmwasm_std::{Addr, Uint128};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use equinox_msg::voter::Config;

/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "eclipse_equinox_voter";

pub const ASTRO_MAINNET: &str =
    "factory/neutron1ffus553eet978k024lmssw0czsxwr97mggyv85lpcsdkft8v9ufsz3sa07/astro";
pub const XASTRO_MAINNET: &str =
    "factory/neutron1zlf3hutsa4qnmue53lz2tfxrutp8y2e3rj4nkghg3rupgl4mqy8s5jgxsn/xASTRO";

pub const GAUGE_VOTING_PERIOD: u64 = 7 * 24 * 3600;

/// Owner of the contract who can update config or set new admin
pub const OWNER: Admin = Admin::new("owner");
pub const CONFIG: Item<Config> = Item::new("config");
/// temporary storage for eclipASTRO recipients
pub const RECIPIENT: Item<Addr> = Item::new("recipient");

/// (essence, update_date)
pub const TOTAL_ESSENCE: Item<(Uint128, u64)> = Item::new("total essence");
pub const USER_ESSENCE: Map<&Addr, Uint128> = Map::new("user essence");
