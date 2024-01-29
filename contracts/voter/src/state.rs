use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_controllers::Admin;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    /// ASTRO token address
    pub base_token: Addr,
    /// xASTRO token address
    pub xtoken: Addr,
    /// vxASTRO contract
    pub vxtoken: Addr,
    /// admin for claim rewards
    pub reward_distributor: Addr,
    /// admin for gauge vote
    pub gauge_voter: Addr,
    /// Astroport Staking contract
    pub staking_contract: Addr,
    /// Astroport Gauge contract
    pub gauge_contract: Addr,
}

/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "eclipse_equinox_voter";

/// Owner of the contract who can update config or set new admin
pub const OWNER: Admin = Admin::new("owner");

pub const CONFIG: Item<Config> = Item::new("config");