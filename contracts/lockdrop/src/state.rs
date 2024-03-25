use cosmwasm_std::Addr;
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use equinox_msg::{
    common::OwnershipProposal,
    lockdrop::{
        Config, LockupInfo, LpLockupState, LpStakingAssetRewardWeights, LpUserLockupInfo,
        SingleLockupState, SingleStakingAssetRewardWeights, SingleUserLockupInfo,
    },
};

/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "eclipASTRO staking contract";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const OWNER: Admin = Admin::new("owner");
pub const CONFIG: Item<Config> = Item::new("config");
pub const OWNERSHIP_PROPOSAL: Item<OwnershipProposal> = Item::new("ownership_proposal");
pub const SINGLE_LOCKUP_STATE: Item<SingleLockupState> = Item::new("single_lockup_state");
pub const LP_LOCKUP_STATE: Item<LpLockupState> = Item::new("lp_lockup_state");
/// Map of lockup info according to asset address, duration
pub const SINGLE_LOCKUP_INFO: Map<u64, LockupInfo> = Map::new("single_lockup_info");
pub const LP_LOCKUP_INFO: Map<u64, LockupInfo> = Map::new("lp_lockup_info");
/// Map of lockup info according to user address, duration
pub const SINGLE_USER_LOCKUP_INFO: Map<(&Addr, u64), SingleUserLockupInfo> =
    Map::new("single_user_lockup_info");
pub const LP_USER_LOCKUP_INFO: Map<(&Addr, u64), LpUserLockupInfo> =
    Map::new("lp_user_lockup_info");

/// Reward weights for asset rewards
pub const SINGLE_STAKING_ASSET_REWARD_WEIGHTS: Item<SingleStakingAssetRewardWeights> =
    Item::new("single_staking_asset_reward_weights");
pub const LP_STAKING_ASSET_REWARD_WEIGHTS: Item<LpStakingAssetRewardWeights> =
    Item::new("lp_staking_asset_reward_weights");
