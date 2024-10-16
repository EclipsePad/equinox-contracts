use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};

use cw20::Cw20ReceiveMsg;

use crate::assets::TokenUnverified;

use super::types::{RouteItem, RouteListItem};

#[cw_serde]
pub struct MigrateMsg {
    pub version: String,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub minter: Option<String>,
    pub eclipse_staking: Option<String>,
    pub equinox_voter: Option<String>,
    pub astroport_router: Option<String>,
    pub astroport_incentives: Option<String>,
    pub eclip_darkeclip_pair: Option<String>,

    pub eclip_denom: Option<String>,
    pub beclip_address: Option<String>,
    pub darkess_address: Option<String>,
    pub darkeclip_address: Option<String>,
    pub eclip_darkeclip_lp_denom: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),

    /// disable user actions
    Pause {},

    /// enable user actions
    Unpause {},

    /// accept admin role
    AcceptAdminRole {},

    UpdateAddressConfig {
        admin: Option<String>,
        minter: Option<String>,
        eclipse_staking: Option<String>,
        equinox_voter: Option<String>,
        astroport_router: Option<String>,
        astroport_incentives: Option<String>,
        eclip_darkeclip_pair: Option<String>,
    },

    UpdateTokenConfig {
        eclip_denom: Option<String>,
        beclip_address: Option<String>,
        darkess_address: Option<String>,
        darkeclip_address: Option<String>,
        eclip_darkeclip_lp_denom: Option<String>,
    },

    /// bECLIP -> darkESS + darkECLIP
    Split {},

    /// darkESS + darkECLIP -> bECLIP
    Join {
        amount: Uint128,
    },

    /// claim ECLIP/bECLIP rewards as darkECLIP holder
    /// and rewards for providing liquidity in ECLIP-darkECLIP
    ClaimRewards {},

    /// claim ECLIP/bECLIP rewards as darkECLIP holder and transfer darkECLIP to new owner
    TransferDarkEclip {
        recipient: String,
    },

    /// exchange darkECLIP on Astroport
    SwapDarkeclipIn {
        token_out: TokenUnverified,
        minimum_receive: Option<Uint128>,
        max_spread: Option<Decimal>,
    },

    SwapDarkeclipOut {
        minimum_receive: Option<Uint128>,
        max_spread: Option<Decimal>,
    },

    /// add liquidity in ECLIP-darkECLIP pair
    ProvideLiquidity {
        darkeclip_amount: Uint128,
        slippage_tolerance: Option<Decimal>,
        min_lp_to_receive: Option<Uint128>,
    },

    /// withdraw liquidity from ECLIP-darkECLIP pair
    WithdrawLiquidity {
        lp_amount: Uint128,
        min_assets_to_receive: Option<Vec<(Uint128, TokenUnverified)>>,
    },

    UpdateRouteList {
        route_list: Vec<RouteListItem<TokenUnverified>>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// query account/contract addresses related config
    #[returns(super::types::AddressConfig)]
    AddressConfig {},

    /// query token related config
    #[returns(super::types::TokenConfig)]
    TokenConfig {},

    #[returns(bool)]
    PauseState {},

    #[returns(super::types::BalancesResponse)]
    Balances { address: String },

    #[returns(Vec<(Addr, super::types::BalancesResponse)>)]
    BalancesList {
        amount: u32,
        start_from: Option<String>,
    },

    /// gov essence list for voter
    #[returns(Vec<(Addr, crate::voter::types::EssenceInfo)>)]
    GovEssence { address_list: Vec<String> },

    #[returns(super::types::UserEssenceResponse)]
    UserEssence {
        address: String,
        block_time: Option<u64>,
    },

    #[returns(Vec<(Addr, super::types::UserEssenceResponse)>)]
    UserEssenceList {
        block_time: Option<u64>,
        amount: u32,
        start_from: Option<String>,
    },

    #[returns(super::types::UserRewards)]
    UserRewards { address: String },

    #[returns(Vec<(Addr, super::types::UserRewards)>)]
    UserRewardsList {
        block_time: Option<u64>,
        amount: u32,
        start_from: Option<String>,
    },

    #[returns(super::types::TotalRewards)]
    TotalRewards {},

    #[returns(super::types::UserVault)]
    UserVault { address: String },

    #[returns(Vec<(Addr, super::types::UserVault)>)]
    UserVaultList {
        block_time: Option<u64>,
        amount: u32,
        start_from: Option<String>,
    },

    #[returns(super::types::TotalVault)]
    TotalVault {},

    #[returns(Uint128)]
    SimulatedSwapOutputAmount {
        amount_in: Uint128,
        route: Vec<RouteItem<TokenUnverified>>,
    },

    #[returns(Vec<super::types::RouteItem<crate::assets::Token>>)]
    Route {
        token_in: TokenUnverified,
        is_reversed: bool,
    },

    #[returns(Vec<super::types::RouteListItem<crate::assets::Token>>)]
    RouteList {
        amount: u32,
        start_from: Option<String>,
    },

    #[returns(super::types::PoolInfoResponse)]
    PoolInfo {},
}
