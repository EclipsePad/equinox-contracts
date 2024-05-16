use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub astro_token: String,
    pub xastro_token: String,
    pub astro_generator: Addr,
    pub staking_contract: Addr,
}

pub const CONTRACT_NAME: &str = "crates.io:eclipsepad-faucet";

pub const CONFIG: Item<Config> = Item::new("config");

pub const LAST_CLAIMED: Map<&Addr, u64> = Map::new("last claimed");

pub const OWNER: Admin = Admin::new("owner");
