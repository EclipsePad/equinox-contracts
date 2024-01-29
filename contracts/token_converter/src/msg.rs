use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;

use crate::state::{Config, RewardConfig};

#[cw_serde]
pub struct InstantiateMsg {
    /// ASTRO token address
    pub base_token: String,
    /// contract owner for update
    pub owner: String,
    /// eclipASTRO token
    pub token: String,
    /// Eclipse vxASTRO holder contract
    pub voter: String,
    /// Eclipse treasury
    pub treasury: String,
    /// eclipASTRO / xASTRO lp staking vault
    pub lp_staking_vault: String,
    /// eclipASTRO staking reward distributor
    pub staking_reward_distributor: String,
    /// cosmic essence reward distributor
    pub pos_reward_distributor: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// stake ASTRO from user
    Receive(Cw20ReceiveMsg),
    /// update config
    UpdateConfig {
        config: Config
    },
    /// update reward config
    UpdateRewardConfig {
        config: RewardConfig
    },
    /// update owner
    UpdateOwner {
        owner: String
    },
    /// claim reward
    Claim {},
    /// claim treasury reward
    ClaimTreasuryReward {
        amount: Uint128
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum QueryMsg {
    /// query config
    Config {},
    /// query reward config
    RewardConfig {},
    /// query user reward
    Reward { user: String },
    /// query owner
    Owner {},
    /// query rewards
    Rewards {}
}

#[cw_serde]
pub enum Cw20HookMsg {
    Stake {},
}
