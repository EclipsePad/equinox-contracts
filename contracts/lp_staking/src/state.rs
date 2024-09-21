use cosmwasm_std::Uint128;
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use equinox_msg::lp_staking::{
    Config, OwnershipProposal, Reward, RewardDistribution, RewardWeight, UserStaking,
};

/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "lp staking contract";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const OWNER: Admin = Admin::new("owner");
pub const CONFIG: Item<Config> = Item::new("config");
pub const STAKING: Map<&String, UserStaking> = Map::new("staking");
pub const TOTAL_STAKING: Item<Uint128> = Item::new("total_staking");
pub const REWARD_WEIGHTS: Item<Vec<RewardWeight>> = Item::new("reward_weights");
pub const REWARD_DISTRIBUTION: Item<RewardDistribution> = Item::new("reward_distribution");
pub const REWARD: Map<(u64, u64), Reward> = Map::new("reward");

pub const LAST_CLAIMED: Item<u64> = Item::new("last_claimed");
/// Stores the latest contract ownership transfer proposal
pub const OWNERSHIP_PROPOSAL: Item<OwnershipProposal> = Item::new("ownership_proposal");
