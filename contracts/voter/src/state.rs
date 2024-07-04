use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use equinox_msg::voter::{
    AddressConfig, DateConfig, EpochInfo, EssenceAllocationItem, EssenceInfo, RewardsInfo,
    TokenConfig, TransferAdminState, VoteResults, WeightAllocationItem,
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
/// historical data vector max length
pub const MAX_EPOCH_AMOUNT: u16 = 26;

/// electors will get 80 % of slacker essence
pub const ELECTOR_ADDITIONAL_ESSENCE_FRACTION: &str = "0.8";

pub const TRANSFER_ADMIN_TIMEOUT: u64 = 3600;

// TODO: add initial weights config

/// blocks the contract to prevent placing votes or voting after final voting at the epoch end
pub const IS_LOCKED: Item<bool> = Item::new("is_locked");

pub const ADDRESS_CONFIG: Item<AddressConfig> = Item::new("address_config");
pub const TOKEN_CONFIG: Item<TokenConfig> = Item::new("token_config");
pub const DATE_CONFIG: Item<DateConfig> = Item::new("date_config");

/// Stores the state of changing admin process
pub const TRANSFER_ADMIN_STATE: Item<TransferAdminState> = Item::new("transfer_admin_state");
/// temporary storage for eclipASTRO recipients
pub const RECIPIENT: Item<Addr> = Item::new("recipient");

/// list of pools with weight allocations by elector address
pub const ELECTOR_WEIGHTS: Map<&Addr, Vec<WeightAllocationItem>> = Map::new("elector_weights");
/// essence info by elector address, slakers are excluded
pub const ELECTOR_ESSENCE: Map<&Addr, EssenceInfo> = Map::new("elector_essence");
/// list of pools with essence allocations for all electors
pub const ELECTOR_VOTES: Item<Vec<EssenceAllocationItem>> = Item::new("elector_votes");

/// essence info by delegator address
pub const DELEGATOR_ESSENCE: Map<&Addr, EssenceInfo> = Map::new("delegator_essence");

/// essence info by slacker address
pub const SLACKER_ESSENCE: Map<&Addr, EssenceInfo> = Map::new("slacker_essence");
/// sum essence info over all slackers
pub const SLACKER_ESSENCE_ACC: Item<EssenceInfo> = Item::new("slacker_essence_acc");

/// dao list of pools with weight allocations
pub const DAO_WEIGHTS: Item<Vec<WeightAllocationItem>> = Item::new("dao_weights");
/// dao essence info, slakers are excluded
pub const DAO_ESSENCE: Item<EssenceInfo> = Item::new("dao_essence");

/// total list of pools with essence allocations, slakers are excluded
pub const TOTAL_VOTES: Item<Vec<EssenceAllocationItem>> = Item::new("total_votes");

/// bribe rewards info by user address
pub const BRIBE_REWARDS: Map<&Addr, RewardsInfo> = Map::new("bribe_rewards");
/// current epoch id and start date
pub const EPOCH_COUNTER: Item<EpochInfo> = Item::new("epoch_counter");
/// historical data, 26 epochs max
pub const VOTE_RESULTS: Item<Vec<VoteResults>> = Item::new("vote_results");
