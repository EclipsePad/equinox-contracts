use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use equinox_msg::voter::{
    AddressConfig, DateConfig, EssenceAllocationItem, EssenceInfo, RewardsInfo, TokenConfig,
    TransferAdminState, VoteResults, WeightAllocationItem,
};

/// Contract name that is used for migration
pub const CONTRACT_NAME: &str = "eclipse-equinox-voter";

pub const STAKE_ASTRO_REPLY_ID: u64 = 1;

pub const ASTRO_MAINNET: &str =
    "factory/neutron1ffus553eet978k024lmssw0czsxwr97mggyv85lpcsdkft8v9ufsz3sa07/astro";
pub const XASTRO_MAINNET: &str =
    "factory/neutron1zlf3hutsa4qnmue53lz2tfxrutp8y2e3rj4nkghg3rupgl4mqy8s5jgxsn/xASTRO";

// https://github.com/astroport-fi/hidden_astroport_governance/blob/feat/revamped_vxastro/packages/astroport-governance/src/emissions_controller/consts.rs
/// vxASTRO voting epoch starts on Mon May 20 00:00:00 UTC 2024
pub const EPOCHS_START: u64 = 1716163200;
pub const DAY: u64 = 86400;
/// vxASTRO voting epoch lasts 14 days
pub const EPOCH_LENGTH: u64 = DAY * 14;
/// User can vote once every 10 days
pub const VOTE_COOLDOWN: u64 = DAY * 10;

pub const TRANSFER_ADMIN_TIMEOUT: u64 = 3600;

pub const ADDRESS_CONFIG: Item<AddressConfig> = Item::new("address_config");
pub const TOKEN_CONFIG: Item<TokenConfig> = Item::new("token_config");
pub const DATE_CONFIG: Item<DateConfig> = Item::new("date_config");

/// Stores the state of changing admin process
pub const TRANSFER_ADMIN_STATE: Item<TransferAdminState> = Item::new("transfer_admin_state");
/// temporary storage for eclipASTRO recipients
pub const RECIPIENT: Item<Addr> = Item::new("recipient");

// user storages
//
/// list of pools with weight allocations by elector address
pub const ELECTOR_WEIGHTS: Map<&Addr, Vec<WeightAllocationItem<Addr>>> =
    Map::new("elector_weights");
/// essence info by elector address, non-voters are excluded
pub const ELECTOR_ESSENCE: Map<&Addr, EssenceInfo> = Map::new("elector_essence");
/// dao list of pools with weight allocations
pub const DAO_WEIGHTS: Item<Vec<WeightAllocationItem<Addr>>> = Item::new("dao_weights");
/// dao essence info, non-voters are excluded
pub const DAO_ESSENCE: Item<EssenceInfo> = Item::new("dao_essence");
/// essence info by delegator address
pub const DELEGATOR_ESSENCE: Map<&Addr, EssenceInfo> = Map::new("delegator_essence");
// bribe rewards info by user address
pub const BRIBE_REWARDS: Map<&Addr, RewardsInfo> = Map::new("bribe_rewards");

// voter storages
//
/// list of pools with essence allocations for all electors
pub const ELECTOR_VOTES: Item<Vec<EssenceAllocationItem<Addr>>> = Item::new("elector_votes");
// /// list of pools with essence allocations for all delegators
// pub const DELEGATOR_VOTES: Item<Vec<EssenceAllocationItem<Addr>>> = Item::new("delegator_votes");
/// total list of pools with essence allocations, non-voters are excluded
pub const TOTAL_VOTES: Item<Vec<EssenceAllocationItem<Addr>>> = Item::new("total_votes");
/// current epoch
pub const EPOCH_ID: Item<u16> = Item::new("epoch_id");
/// historical data, 26 epochs max
pub const VOTE_RESULTS: Item<Vec<VoteResults>> = Item::new("vote_results");
