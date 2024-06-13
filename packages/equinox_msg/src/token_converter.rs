use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;

use crate::token::InstantiateMarketingInfo;

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract owner for updating
    pub owner: String,
    /// ASTRO token
    pub token_in: String,
    /// xASTRO token
    pub xtoken: String,
    /// Eclipse treasury
    pub treasury: String,
    /// eclipASTRO token code id
    pub token_code_id: u64,
    /// eclipASTRO marketing info
    pub marketing: Option<InstantiateMarketingInfo>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// stake ASTRO from user
    Receive(Cw20ReceiveMsg),
    /// update config
    UpdateConfig { config: UpdateConfig },
    /// update reward config
    UpdateRewardConfig { config: RewardConfig },
    /// update owner
    UpdateOwner { owner: String },
    /// claim reward
    Claim {},
    /// claim treasury reward
    ClaimTreasuryReward { amount: Uint128 },
    /// withdraw xASTRO
    WithdrawAvailableBalance { amount: Uint128, recipient: String },

    /// mint eclipAstro
    MintEclipAstro { amount: Uint128, recipient: String },
}

#[cw_serde]
pub struct MigrateMsg {
    pub update_contract_name: Option<bool>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// query config
    #[returns(Config)]
    Config {},
    /// query reward config
    #[returns(RewardConfig)]
    RewardConfig {},
    /// query rewards
    #[returns(RewardResponse)]
    Rewards {},
    /// query owner
    #[returns(Addr)]
    Owner {},
    /// query withdrawable xASTRO balance
    #[returns(Uint128)]
    WithdrawableBalance {},
}

#[cw_serde]
pub enum Cw20HookMsg {
    Convert {},
}

#[cw_serde]
pub struct UpdateConfig {
    /// ASTRO token
    pub token_in: Option<String>,
    /// eclipASTRO token
    pub token_out: Option<String>,
    /// xASTRO token
    pub xtoken: Option<String>,
    /// Eclipse vxASTRO holder contract
    pub vxtoken_holder: Option<String>,
    /// Eclipse treasury
    pub treasury: Option<String>,
    /// eclipASTRO / ASTRO stability pool
    pub stability_pool: Option<String>,
    /// eclipASTRO staking reward distributor
    pub staking_reward_distributor: Option<String>,
    /// cosmic essence reward distributor
    pub ce_reward_distributor: Option<String>,
}

#[cw_serde]
pub struct RewardConfig {
    /// users' reward in basis point
    pub users: u32,
    /// treasury reward in basis point
    pub treasury: u32,
    /// cosmic essence holders' reward in basis point
    pub ce_holders: u32,
    /// stability pool reward in basis point
    pub stability_pool: u32,
}

#[cw_serde]
pub struct RewardResponse {
    pub users_reward: Reward,
    pub ce_holders_reward: Reward,
    pub stability_pool_reward: Reward,
    pub treasury_reward: Reward,
}

#[cw_serde]
pub struct Reward {
    pub token: String,
    pub amount: Uint128,
}

#[cw_serde]
pub struct Config {
    /// ASTRO token
    pub token_in: Addr,
    /// eclipASTRO token
    pub token_out: Addr,
    /// xASTRO token
    pub xtoken: Addr,
    /// Eclipse vxASTRO holder contract
    pub vxtoken_holder: Addr,
    /// Eclipse treasury
    pub treasury: Addr,
    /// eclipASTRO / ASTRO stability pool
    pub stability_pool: Addr,
    /// eclipASTRO staking reward distributor
    pub staking_reward_distributor: Addr,
    /// cosmic essence reward distributor
    pub ce_reward_distributor: Addr,
}
