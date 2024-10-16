use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

use crate::{
    assets::{Funds, Token},
    splitter::types::{AddressConfig, RouteItem, TokenConfig},
    staking::types::{TransferAdminState, Vault},
};

pub const CONTRACT_NAME: &str = "eclipse-splitter";

pub const PROVIDE_LIQUIDITY_REPLY_ID: u64 = 1;
pub const WITHDRAW_LIQUIDITY_REPLY_ID: u64 = 2;
pub const SWAP_DARKECLIP_OUT_REPLY_ID: u64 = 3;

pub const TRANSFER_ADMIN_TIMEOUT: u64 = 3600;

// structure of splitter vault system
//
// TOTAL_DARKECLIP_VAULT (darkeclip amount, darkeclip rewards)
//      USER_DARKECLIP_VAULT (darkeclip amount, darkeclip rewards - regular darkeclip holders)
//      TOTAL_LP_VAULT_WITH_DARKECLIP (darkeclip amount, darkeclip rewards)
//
// TOTAL_LP_VAULT (lp amount, lp rewards)
// 		USER_LP_VAULT (lp amount, lp rewards - splitter admin can provide liquidity as well)
//
// TOTAL_LP_VAULT_WITH_DARKECLIP darkeclip rewards depends on eclip/darkeclip ratio in liquidity pool
// therefore must be accumulated on each swap, provide/withdraw liquidity
// USER_LP_VAULT rewards can't be accumulated same way. That's why these rewards are defined as lp rewards
// while user darkeclip rewards will be calculated via lp rewards shares:
// user_darkeclip_rewards = total_lp_vault_darkeclip_rewards * user_lp_rewards / total_lp_vault_lp_rewards

/// Stores the state of changing admin process
pub const TRANSFER_ADMIN_STATE: Item<TransferAdminState> = Item::new("transfer_admin_state");
/// Stores user functions pause flag
pub const IS_PAUSED: Item<bool> = Item::new("is_paused");
pub const ADDRESS_CONFIG: Item<AddressConfig> = Item::new("address_config");
pub const TOKEN_CONFIG: Item<TokenConfig> = Item::new("token_config");

/// ECLIP claimed from splitter bonded vault
pub const CLAIMED_BONDED_VAULT_REWARDS: Item<Uint128> = Item::new("claimed_bonded_vault_rewards");
/// incentives claimed for providing eclip-darkeclip liquidity
pub const CLAIMED_LP_INCENTIVES: Item<Vec<Funds<Token>>> = Item::new("claimed_lp_incentives");

/// contains darkeclip amount, darkeclip rewards for sum of users darkeclip vaults
pub const TOTAL_DARKECLIP_VAULT: Item<Vault> = Item::new("total_darkeclip_vault");
/// contains darkeclip amount, darkeclip rewards for sum of users lp vaults
pub const TOTAL_LP_VAULT_WITH_DARKECLIP: Item<Vault> = Item::new("total_lp_vault_with_darkeclip");
/// contains darkeclip amount, darkeclip rewards for user darkeclip vault
pub const USER_DARKECLIP_VAULT: Map<&Addr, Vault> = Map::new("user_darkeclip_vault");

/// contains lp amount, lp rewards for sum of users lp vaults
pub const TOTAL_LP_VAULT: Item<Vault> = Item::new("total_lp_vault");
/// contains lp amount, lp rewards for user lp vault
pub const USER_LP_VAULT: Map<&Addr, Vault> = Map::new("user_lp_vault");

/// route by 1st denom_or_address_in, last denom_or_address_out is DARKECLIP
// possible options for mainnet:
// [TOKEN-ECLIP, ECLIP-DARKECLIP]
// [TOKEN-ATOM, ATOM-ECLIP, ECLIP-DARKECLIP]
// [TOKEN-NTRN, NTRN-ATOM, ATOM-ECLIP, ECLIP-DARKECLIP]
// [TOKEN-USDC, USDC-NTRN, NTRN-ATOM, ATOM-ECLIP, ECLIP-DARKECLIP]
// [TOKEN-axlUSDC, axlUSDC-NTRN, NTRN-ATOM, ATOM-ECLIP, ECLIP-DARKECLIP]
// [TOKEN-ASTRO, ASTRO-USDC, USDC-NTRN, NTRN-ATOM, ATOM-ECLIP, ECLIP-DARKECLIP]
// [TOKEN-AXL, AXL-NTRN, NTRN-ATOM, ATOM-ECLIP, ECLIP-DARKECLIP]
// [TOKEN-wstETH, wstETH-NTRN, NTRN-ATOM, ATOM-ECLIP, ECLIP-DARKECLIP]
// [TOKEN-TIA, TIA-NTRN, NTRN-ATOM, ATOM-ECLIP, ECLIP-DARKECLIP]
pub const ROUTE_CONFIG: Map<&str, Vec<RouteItem<Token>>> = Map::new("route_config");

/// temporary storage for sender address and darkeclip amount added in lp
/// or sender address and lp amount withdrawn from lp
pub const SENDER_AND_AMOUNT: Item<(Addr, Uint128)> = Item::new("sender_and_darkeclip");
