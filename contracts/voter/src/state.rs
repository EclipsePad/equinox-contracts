use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use equinox_msg::voter::{AddressConfig, DateConfig, TokenConfig, TransferAdminState};

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

/// Stores time dependent total staking essence components (a,b)
/// to reduce calculations amount during rewards accumulation
/// total_staking_essence = (a * block_time - b) / seconds_per_essence
/// a = sum(staked_eclip_amount), b = sum(staked_eclip_amount * vault.creation_date)
pub const TOTAL_STAKING_ESSENCE_COMPONENTS: Item<(Uint128, Uint128)> =
    Item::new("total_staking_essence_components");

/// Stores time independent total locking essence amount
/// to reduce calculations amount during rewards accumulation
pub const TOTAL_LOCKING_ESSENCE: Item<Uint128> = Item::new("total_locking_essence");

/// Stores time dependent staking essence components (a,b)
/// to reduce calculations amount during rewards accumulation
/// staking_essence = (a * block_time - b) / seconds_per_essence
/// a = sum(staked_eclip_amount), b = sum(staked_eclip_amount * vault.creation_date)
pub const STAKING_ESSENCE_COMPONENTS: Map<&Addr, (Uint128, Uint128)> =
    Map::new("staking_essence_components");

/// Stores time independent locking essence amount
/// to reduce calculations amount during rewards accumulation
pub const LOCKING_ESSENCE: Map<&Addr, Uint128> = Map::new("locking_essence");
