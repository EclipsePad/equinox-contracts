use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use crate::presale::types::{
    AddressConfig, Allocation, ClaimStats, DateConfig, FundConfig, Participant, SaleStats,
};

pub const CONTRACT_NAME: &str = "eclipsepad-presale";

pub const DENOM_NOBLE_USDC: &str =
    "ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81";
pub const DENOM_AXELAR_USDC: &str =
    "ibc/F082B65C88E4B6D5EF1DB243CDA1D331D002759E938A0F5CD3FFDC5D53B3E349"; // axlUSDC on Neutron mainnet
pub const DENOM_ECLIP: &str = "factory/neutron10sr06r3qkhn7xzpw3339wuj77hu06mzna6uht0/eclip"; // ECLIP on Neutron mainnet

pub const FUND_CURRENCY_DECIMALS: u8 = 6;
pub const REWARD_CURRENCY_DECIMALS: u8 = 6;

pub const FUND_LOT: u128 = 1;
pub const EXCHANGE_RATE: &str = "1";
pub const CLIENT_FEE_RATE: &str = "0";

pub const PRIVATE_START_TIME: u64 = 9_000_000_000_000;
pub const PRIVATE_PRESALE_PERIOD: u64 = 24 * 3600;

pub const PUBLIC_START_TIME: u64 = 9_100_000_000_000;
pub const PUBLIC_PRESALE_PERIOD: u64 = 24 * 3600;

pub const MAX_PRIVATE_ALLOCATION: u128 = 2_000_000_000;
pub const MAX_PUBLIC_ALLOCATION: u128 = 1_000_000_000;

pub const TOTAL_REWARDS_AMOUNT: u128 = 1_000_000_000_000;

pub const PAGINATION_MAX_LIMIT: u32 = 10_000;
pub const PAGINATION_DEFAULT_LIMIT: u32 = 1_000;

/// contract config
pub const ADDRESS_CONFIG: Item<AddressConfig> = Item::new("address config");
pub const FUND_CONFIG: Item<FundConfig> = Item::new("fund config");
pub const DATE_CONFIG: Item<DateConfig> = Item::new("date config");

/// sale stats
pub const SALE_STATS: Item<SaleStats> = Item::new("sale stats");
pub const CLAIM_STATS: Item<ClaimStats> = Item::new("claim stats");

/// allocation by user address
pub const ALLOCATIONS: Map<&Addr, Allocation> = Map::new("allocations");
/// participant by user address
pub const PARTICIPANTS: Map<&Addr, Participant> = Map::new("participants");
