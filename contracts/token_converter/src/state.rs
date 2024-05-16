use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw_controllers::Admin;
use cw_storage_plus::Item;
use equinox_msg::token_converter::{Config, RewardConfig};

/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "token_converter";

/// Contract version that is used for migration.
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Owner of the contract who can update config or set new admin
pub const OWNER: Admin = Admin::new("owner");
pub const CONFIG: Item<Config> = Item::new("config");
pub const REWARD_CONFIG: Item<RewardConfig> = Item::new("reward_config");
/// Total staking data
pub const TOTAL_STAKE_INFO: Item<StakeInfo> = Item::new("total_stake_info");
/// Withdrawable xASTRO
pub const WITHDRAWABLE_BALANCE: Item<Uint128> = Item::new("withdrawable_balance");
/// withdrawable treasury reward
pub const TREASURY_REWARD: Item<Uint128> = Item::new("treasury_reward");

/// Staking user
pub const USER_STAKING: Item<UserStake> = Item::new("staking_user");

#[cw_serde]
pub struct StakeInfo {
    /// initial ASTRO stake
    pub astro: Uint128,
    /// user's xASTRO amount
    pub xastro: Uint128,
    /// claimed xASTRO amount
    pub claimed_xastro: Uint128,
}

impl Default for StakeInfo {
    fn default() -> Self {
        StakeInfo {
            astro: Uint128::zero(),
            xastro: Uint128::zero(),
            claimed_xastro: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct UserStake {
    pub user: String,
    pub stake: Uint128,
}
