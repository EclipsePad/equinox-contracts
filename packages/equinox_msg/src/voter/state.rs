use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

use super::types::{
    AddressConfig, DateConfig, EpochInfo, EssenceInfo, RewardsClaimStage, RewardsInfo, RouteItem,
    TokenConfig, TransferAdminState, VoteResults, WeightAllocationItem,
};

/// Contract name that is used for migration
pub const CONTRACT_NAME: &str = "eclipse-equinox-voter";

pub const STAKE_ASTRO_REPLY_ID: u64 = 1;
pub const SWAP_REWARDS_REPLY_ID_MIN: u64 = 10;
pub const SWAP_REWARDS_REPLY_ID_MAX: u64 = SWAP_REWARDS_REPLY_ID_MIN + u8::MAX as u64;
pub const SWAP_REWARDS_REPLY_ID_CNT: Item<u8> = Item::new("swap_rewards_reply_cnt");

pub const ASTRO_MAINNET: &str =
    "factory/neutron1ffus553eet978k024lmssw0czsxwr97mggyv85lpcsdkft8v9ufsz3sa07/astro";
pub const XASTRO_MAINNET: &str =
    "factory/neutron1zlf3hutsa4qnmue53lz2tfxrutp8y2e3rj4nkghg3rupgl4mqy8s5jgxsn/xASTRO";

// https://github.com/astroport-fi/hidden_astroport_governance/blob/feat/revamped_vxastro/packages/astroport-governance/src/emissions_controller/consts.rs
/// vxASTRO voting epoch starts on Mon May 20 00:00:00 UTC 2024
pub const GENESIS_EPOCH_START_DATE: u64 = 1716163200;
pub const DAY: u64 = 86400;
/// vxASTRO voting epoch lasts 14 days
pub const EPOCH_LENGTH: u64 = DAY * 14;
/// User can vote once every 10 days
pub const VOTE_DELAY: u64 = DAY * 10;
/// historical data vector max length
pub const MAX_EPOCH_AMOUNT: u16 = 26;

/// electors will get 80 % of slacker essence
pub const ELECTOR_ADDITIONAL_ESSENCE_FRACTION: &str = "0.8";
// dao treasury will get 20 % of all dao rewards
pub const DAO_TREASURY_REWARDS_FRACTION: &str = "0.2";

pub const TRANSFER_ADMIN_TIMEOUT: u64 = 3600;

/// blocks the contract to prevent placing votes or voting after final voting at the epoch end
pub const IS_LOCKED: Item<bool> = Item::new("is_locked");

pub const ADDRESS_CONFIG: Item<AddressConfig> = Item::new("address_config");
pub const TOKEN_CONFIG: Item<TokenConfig> = Item::new("token_config");
pub const DATE_CONFIG: Item<DateConfig> = Item::new("date_config");

/// state machine to rotate actions executed by x/cron
pub const REWARDS_CLAIM_STAGE: Item<RewardsClaimStage> = Item::new("rewards_claim_stage");

/// Stores the state of changing admin process
pub const TRANSFER_ADMIN_STATE: Item<TransferAdminState> = Item::new("transfer_admin_state");
/// temporary storage for eclipASTRO recipients
pub const RECIPIENT: Item<Addr> = Item::new("recipient");

/// essence info by user address
pub const USER_ESSENCE: Map<&Addr, EssenceInfo> = Map::new("user_essence");
/// bribe rewards info by user address
pub const USER_REWARDS: Map<&Addr, RewardsInfo> = Map::new("user_rewards");

/// list of pools with weight allocations by elector address (to affect on total allocation)
pub const ELECTOR_WEIGHTS: Map<&Addr, Vec<WeightAllocationItem>> = Map::new("elector_weights");
/// list of pools with weight allocations by elector address (to calculate user rewards)
pub const ELECTOR_WEIGHTS_REF: Map<&Addr, Vec<WeightAllocationItem>> =
    Map::new("elector_weights_ref");
/// dao list of pools with weight allocations
pub const ELECTOR_WEIGHTS_ACC: Item<Vec<WeightAllocationItem>> = Item::new("elector_weights_acc");
/// sum essence info over all electors, slackers are excluded
pub const ELECTOR_ESSENCE_ACC: Item<EssenceInfo> = Item::new("elector_essence_acc");

/// delegator addresses
pub const DELEGATOR_ADDRESSES: Map<&Addr, bool> = Map::new("delegator_addresses");
/// dao list of pools with weight allocations
pub const DAO_WEIGHTS_ACC: Item<Vec<WeightAllocationItem>> = Item::new("dao_weights_acc");
/// dao essence info, slackers are excluded
pub const DAO_ESSENCE_ACC: Item<EssenceInfo> = Item::new("dao_essence_acc");

/// sum essence info over all slackers
pub const SLACKER_ESSENCE_ACC: Item<EssenceInfo> = Item::new("slacker_essence_acc");

/// historical data, 26 epochs max
pub const VOTE_RESULTS: Item<Vec<VoteResults>> = Item::new("vote_results");
/// temporary storage fro eclip bribe rewards
pub const TEMPORARY_REWARDS: Item<Uint128> = Item::new("temporary_rewards");
/// current epoch id and start date
pub const EPOCH_COUNTER: Item<EpochInfo> = Item::new("epoch_counter");
/// route by 1st denom_in, last denom_out is ECLIP
pub const ROUTE_CONFIG: Map<&str, Vec<RouteItem>> = Map::new("route_config");
