use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

use crate::staking::types::{
    Config, LockerInfo, PaginationConfig, StakerInfo, State, TransferAdminState,
};

pub const CONTRACT_NAME: &str = "eclipsepad-staking";

pub const DAO_TREASURY_ADDRESS: &str =
    "neutron1sfw29y5mu5cuwml56m5jq6a32redhnc2ns7mgltfxl424wad7fcsr2pzdp";

pub const ECLIP_MAINNET: &str = "factory/neutron10sr06r3qkhn7xzpw3339wuj77hu06mzna6uht0/eclip";
pub const ECLIP_TESTNET: &str =
    "factory/neutron182rjdvv4q82rctk7cfdl423e8vhf54tu2c75fplp37p0njnh5znq4asklm/eclip";

pub const DAY_IN_SECONDS: u64 = 24 * 3600;
pub const YEAR_IN_SECONDS: u64 = 31_536_000;

pub const PERIOD_TIER_0: u64 = 30 * DAY_IN_SECONDS; // 30 days
pub const PERIOD_TIER_1: u64 = 90 * DAY_IN_SECONDS; // 90 days
pub const PERIOD_TIER_2: u64 = 180 * DAY_IN_SECONDS; // 180 days
pub const PERIOD_TIER_3: u64 = 270 * DAY_IN_SECONDS; // 270 days
pub const PERIOD_TIER_4: u64 = 365 * DAY_IN_SECONDS; // 365 days

pub const REWARDS_TIER_0: u64 = 50_400_000_000; // 30 days, $ECLIP in micro units
pub const REWARDS_TIER_1: u64 = 201_600_000_000; // 90 days, $ECLIP in micro units
pub const REWARDS_TIER_2: u64 = 403_200_000_000; // 180 days, $ECLIP in micro units
pub const REWARDS_TIER_3: u64 = 1_360_800_000_000; // 270 days, $ECLIP in micro units
pub const REWARDS_TIER_4: u64 = 3_024_000_000_000; // 365 days, $ECLIP in micro units

pub const MAX_REWARDS_TIER_0: u64 = 12_600_000_000; // 30 days, $ECLIP in micro units
pub const MAX_REWARDS_TIER_1: u64 = 50_400_000_000; // 90 days, $ECLIP in micro units
pub const MAX_REWARDS_TIER_2: u64 = 201_600_000_000; // 180 days, $ECLIP in micro units
pub const MAX_REWARDS_TIER_3: u64 = 1_012_200_000_000; // 270 days, $ECLIP in micro units
pub const MAX_REWARDS_TIER_4: u64 = 3_024_000_000_000; // 365 days, $ECLIP in micro units

pub const TIER_4: usize = 4;

// TODO: set on migration
pub const ECLIP_PER_SECOND: u64 = 136_889; // calc for mainnet

// pub const ECLIP_PER_SECOND: u64 = 245; // testnet

pub const ECLIP_PER_SECOND_MULTIPLIER: &str = "0.99";
pub const DECREASING_REWARDS_PERIOD: u64 = 7 * DAY_IN_SECONDS;

pub const SECONDS_PER_ESSENCE: u128 = 31_536_000;

pub const PENALTY_MULTIPLIER: &str = "0.7";

pub const VAULTS_LIMIT: usize = 25;

pub const TRANSFER_ADMIN_TIMEOUT: u64 = 3600;

pub const MAX_APR: u128 = 10_000;

pub const PAGINATION_AMOUNT: u32 = 10;
pub const PAGINATION_INDEX: Option<Addr> = None;

/// Stores the configuration of this contract
pub const CONFIG: Item<Config> = Item::new("config");

/// Stores user functions pause flag
pub const IS_PAUSED: Item<bool> = Item::new("is_paused");

/// Stores the state of changing admin process
pub const TRANSFER_ADMIN_STATE: Item<TransferAdminState> = Item::new("transfer_admin_state");

/// Stores bECLIP vault creation date
pub const BONDED_VAULT_CREATION_DATE: Map<&Addr, u64> = Map::new("bonded_vault_creation_date");
/// Stores minted bECLIP amount
pub const BECLIP_SUPPLY: Item<Uint128> = Item::new("beclip_supply");

/// Stores time dependent total staking essence components (a,b)
/// to reduce calculations amount during rewards accumulation
/// total_staking_essence = (a * block_time - b) / seconds_per_essence
/// a = sum(staked_eclip_amount), b = sum(staked_eclip_amount * vault.creation_date)
pub const TOTAL_STAKING_ESSENCE_COMPONENTS: Item<(Uint128, Uint128)> =
    Item::new("total_staking_essence_components");

/// Stores time independent total locking essence amount
/// to reduce calculations amount during rewards accumulation
pub const TOTAL_LOCKING_ESSENCE: Item<Uint128> = Item::new("total_locking_essence");

/// Stores the current state of this contract
pub const STAKE_STATE: Item<State> = Item::new("stake_state");

/// Stores all the different locked states
pub const LOCK_STATES: Item<Vec<State>> = Item::new("lock_state");

/// Stores the configuration of pagination
pub const PAGINATION_CONFIG: Item<PaginationConfig> = Item::new("pagination_config");

/// Stores the nearest date when locking rewards will be decreased
pub const DECREASING_REWARDS_DATE: Item<u64> = Item::new("decreasing_rewards_date");

/// Stores staker info
pub const STAKER_INFO: Map<&Addr, StakerInfo> = Map::new("staker_info");

/// Stores locker info
pub const LOCKER_INFO: Map<&Addr, Vec<LockerInfo>> = Map::new("locker_info");

/// Stores time dependent staking essence components (a,b)
/// to reduce calculations amount during rewards accumulation
/// staking_essence = (a * block_time - b) / seconds_per_essence
/// a = sum(staked_eclip_amount), b = sum(staked_eclip_amount * vault.creation_date)
pub const STAKING_ESSENCE_COMPONENTS: Map<&Addr, (Uint128, Uint128)> =
    Map::new("staking_essence_components");

/// Stores time independent locking essence amount
/// to reduce calculations amount during rewards accumulation
pub const LOCKING_ESSENCE: Map<&Addr, Uint128> = Map::new("locking_essence");
