use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};
use cw_controllers::Admin;
use cw_storage_plus::Item;

/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "eclipse_equinox_voter";
/// Owner of the contract who can update config or set new admin
pub const OWNER: Admin = Admin::new("owner");
pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct Config {
    /// ASTRO token address
    pub base_token: Addr,
    /// xASTRO token address
    pub xtoken: Addr,
    /// vxASTRO contract
    pub vxtoken: Addr,
    /// Astroport Staking contract
    pub staking_contract: Addr,
    /// Converter contract
    pub converter_contract: Addr,
    /// Gauge contract
    pub gauge_contract: Addr,
    /// Astroport Gauge contract
    pub astroport_gauge_contract: Addr,
}

#[cw_serde]
pub struct Vote {
    /// Option voted for.
    pub option: String,
    /// The weight of the power given to this vote
    pub weight: Decimal,
}
