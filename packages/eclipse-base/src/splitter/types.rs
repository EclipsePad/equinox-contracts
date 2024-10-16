use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};

use crate::{
    assets::{Funds, Token},
    staking::types::{Vault, WithNewEmpty},
};

#[cw_serde]
pub struct AddressConfig {
    pub admin: Addr,
    pub minter: Addr,
    pub eclipse_staking: Addr,
    pub equinox_voter: Addr,
    pub astroport_router: Addr,
    pub astroport_incentives: Addr,
    pub eclip_darkeclip_pair: Addr,
}

#[cw_serde]
pub struct TokenConfig {
    pub eclip_denom: String,
    /// beclip - same as tier 4 vault
    pub beclip_address: Addr,
    /// darkess - IDO participation
    pub darkess_address: Addr,
    /// darkeclip - staking/locking rewards, voting
    pub darkeclip_address: Addr,
    /// eclip-darkeclip lp token
    pub eclip_darkeclip_lp_denom: String,
}

#[cw_serde]
pub struct RouteItem<T: From<Token>> {
    pub token_in: T,
    pub token_out: T,
}

impl<T: From<Token> + Clone> RouteItem<T> {
    pub fn new(token_in: &T, token_out: &T) -> Self {
        Self {
            token_in: token_in.to_owned(),
            token_out: token_out.to_owned(),
        }
    }
}

#[cw_serde]
pub struct RouteListItem<T: From<Token>> {
    pub token_in: T,
    pub route: Vec<RouteItem<T>>,
}

impl<T: From<Token> + Clone> RouteListItem<T> {
    pub fn new(token_in: &T, route: &[RouteItem<T>]) -> Self {
        Self {
            token_in: token_in.to_owned(),
            route: route.to_owned(),
        }
    }
}

#[cw_serde]
pub struct VaultAndRewards {
    pub vault: RoleData<TotalVault, UserVault>,
    pub rewards: RoleData<TotalRewards, UserRewards>,
}

#[cw_serde]
pub struct RoleData<T, U> {
    pub total: T,
    pub user: U,
}

#[cw_serde]
pub struct TotalVault {
    pub darkeclip: Vault,
    pub lp_with_darkeclip: Vault,
    pub lp: Vault,
}

impl WithNewEmpty for TotalVault {
    fn new_empty(block_time: u64) -> Self {
        Self {
            darkeclip: Vault::new_empty(block_time),
            lp_with_darkeclip: Vault::new_empty(block_time),
            lp: Vault::new_empty(block_time),
        }
    }
}

#[cw_serde]
pub struct UserVault {
    pub darkeclip: Vault,
    pub lp: Vault,
}

impl WithNewEmpty for UserVault {
    fn new_empty(block_time: u64) -> Self {
        Self {
            darkeclip: Vault::new_empty(block_time),
            lp: Vault::new_empty(block_time),
        }
    }
}

#[derive(Default)]
#[cw_serde]
pub struct TotalRewards {
    /// incentives for providing eclip-darkeclip liquidity
    pub incentives: ExternalRewards<Vec<Funds<Token>>>,
    /// eclip rewards from splitter bonded vault
    pub eclip: ExternalRewards<Uint128>,
    /// darkeclip rewards from TOTAL_DARKECLIP_VAULT
    pub darkeclip_full: Uint128,
    /// darkeclip rewards from TOTAL_LP_VAULT_WITH_DARKECLIP
    pub darkeclip_from_lp: Uint128,
    /// lp rewards from TOTAL_LP_VAULT
    pub lp: Uint128,
}

#[derive(Default)]
#[cw_serde]
pub struct UserRewards {
    /// incentives for providing eclip-darkeclip liquidity
    pub incentives: Vec<Funds<Token>>,
    /// eclip shares calculated from user's all darkeclip rewards
    pub eclip: MultipleRewards,
    /// darkeclip rewards from USER_DARKECLIP_VAULT and
    /// darkeclip shares calculated from lp rewards from USER_LP_VAULT
    pub darkeclip: MultipleRewards,
    /// lp rewards from USER_LP_VAULT
    pub lp: Uint128,
}

pub trait WithFull<T> {
    fn full(&self) -> T;
}

#[derive(Default)]
#[cw_serde]
pub struct ExternalRewards<T> {
    pub claimed: T,
    pub unclaimed: T,
}

impl WithFull<Uint128> for ExternalRewards<Uint128> {
    fn full(&self) -> Uint128 {
        self.claimed + self.unclaimed
    }
}

impl WithFull<Vec<Funds<Token>>> for ExternalRewards<Vec<Funds<Token>>> {
    fn full(&self) -> Vec<Funds<Token>> {
        add_asset_lists(&self.unclaimed, &self.claimed, false)
    }
}

/// if is_negative == false return list_a + list_b                              \
/// if is_negative == true return list_a - list_b
pub fn add_asset_lists(
    list_a: &[Funds<Token>],
    list_b: &[Funds<Token>],
    is_negative: bool,
) -> Vec<Funds<Token>> {
    let mut denom_or_address_list: Vec<String> = vec![list_a, list_b]
        .concat()
        .iter()
        .map(|x| x.currency.token.get_denom_or_address())
        .collect();
    denom_or_address_list.sort_unstable();
    denom_or_address_list.dedup();

    denom_or_address_list
        .iter()
        .map(|denom_or_address| {
            let asset_a = list_a
                .iter()
                .find(|x| &x.currency.token.get_denom_or_address() == denom_or_address);

            let asset_b = list_b
                .iter()
                .find(|x| &x.currency.token.get_denom_or_address() == denom_or_address);

            let asset = asset_a.map_or(asset_b, Some).unwrap();
            let amount_a = asset_a.map(|x| x.amount).unwrap_or_default();
            let amount_b = asset_b.map(|x| x.amount).unwrap_or_default();
            let amount = if is_negative {
                amount_a - amount_b
            } else {
                amount_a + amount_b
            };

            Funds::new(amount, &asset.currency)
        })
        .filter(|x| !x.amount.is_zero())
        .collect()
}

#[derive(Default)]
#[cw_serde]
pub struct MultipleRewards {
    pub from_darkeclip: Uint128,
    pub from_lp: Uint128,
}

impl WithFull<Uint128> for MultipleRewards {
    fn full(&self) -> Uint128 {
        self.from_darkeclip + self.from_lp
    }
}

#[cw_serde]
pub struct RewardsCalculationResult {
    pub user_rewards: Uint128,
    pub user_internal_rewards: Uint128,
    pub total_internal_rewards: Uint128,
}

#[cw_serde]
pub struct BalancesResponse {
    pub eclip: Uint128,
    pub beclip: Uint128,
    pub darkeclip: Uint128,
    pub darkess: Uint128,
}

#[cw_serde]
pub struct UserEssenceResponse {
    pub staking_contract_essence: Uint128,
    pub darkeclip_essence: Uint128,
    pub darkess_essence: Uint128,
    pub lp_essence: Uint128,
    pub gov_essence: Uint128,
    pub ido_essence: Uint128,
}

#[cw_serde]
pub struct PoolInfoResponse {
    pub darkeclip: Uint128,
    pub eclip: Uint128,
    pub lp: Uint128,
}
