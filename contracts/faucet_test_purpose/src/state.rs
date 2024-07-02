use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub astro_token: String,
    pub xastro_token: String,
    // pub eclipastro_token: Addr,
    // pub lp_token: Addr,
    pub astro_generator: Addr,
    pub staking_contract: Addr,
    // pub lp_contract: Addr,
    // pub converter: Addr,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            astro_token: "".to_string(),
            xastro_token: "".to_string(),
            // eclipastro_token: Addr::unchecked(""),
            // lp_token: Addr::unchecked(""),
            astro_generator: Addr::unchecked(""),
            staking_contract: Addr::unchecked(""),
            // lp_contract: Addr::unchecked(""),
            // converter: Addr::unchecked(""),
        }
    }
}

pub const CONTRACT_NAME: &str = "crates.io:eclipsefi-faucet";

pub const CONFIG: Item<Config> = Item::new("config");

pub const LAST_CLAIMED: Map<&Addr, u64> = Map::new("last claimed");

pub const OWNER: Admin = Admin::new("owner");
