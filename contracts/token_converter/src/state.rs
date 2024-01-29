use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    /// ASTRO token
    pub base_token: Addr,
    /// eclipASTRO token
    pub token: Addr,
    /// Eclipse vxASTRO holder contract
    pub voter: Addr,
    /// Eclipse treasury
    pub treasury: Addr,
    /// eclipASTRO / xASTRO lp staking vault
    pub lp_staking_vault: Addr,
    /// eclipASTRO staking reward distributor
    pub staking_reward_distributor: Addr,
    /// cosmic essence reward distributor
    pub pos_reward_distributor: Addr,
}

#[cw_serde]
pub struct StakeInfo {
    /// initial ASTRO stake
    pub stake: Uint128,
    /// user's xASTRO amount
    pub xtoken: Uint128,
    /// claimed
    pub claimed: Uint128,
}

#[cw_serde]
pub struct RewardConfig {
    /// users' reward in basis point
    pub users: u32,
    /// treasury reward in basis point
    pub treasury: u32,
    /// cosmic essence holders' reward in basis point
    pub voters: u32,
    /// stability pool reward in basis point
    pub stability_pool: u32,
}

/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "token_converter";

/// Owner of the contract who can update config or set new admin
pub const OWNER: Admin = Admin::new("owner");

pub const CONFIG: Item<Config> = Item::new("config");

pub const REWARD_CONFIG: Item<RewardConfig> = Item::new("reward_config");

/// User data
pub const STAKE_INFO: Map<Addr, StakeInfo> = Map::new("stake_info");