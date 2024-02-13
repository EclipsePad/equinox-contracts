use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract owner for updating
    pub owner: String,
    /// eclipASTRO token
    pub eclipastro: String,
    /// ECLIP token
    pub eclip: String,
    /// flexible staking pool
    pub flexible_staking: String,
    /// timelock staking pool
    pub timelock_staking: String,
    /// eclipASTRO reward contract
    pub token_converter: String,
    /// ECLIP daily reward
    pub eclip_daily_reward: Uint128,
    /// locking_reward_config
    pub locking_reward_config: Vec<LockingRewardConfig>,
}

/// This structure describes the execute messages available in the contract.
#[cw_serde]
pub enum ExecuteMsg {
    /// Change the owner
    UpdateOwner {
        owner: String,
    },
    /// Change config
    UpdateConfig {
        config: UpdateConfigMsg,
    },
    FlexibleStake {
        user: String,
        amount: Uint128,
    },
    TimelockStake {
        user: String,
        amount: Uint128,
        duration: u64,
    },
    Claim {
        user: String,
    },
    FlexibleUnstake {
        user: String,
        amount: Uint128,
    },
    TimelockUnstake {
        user: String,
        amount: Uint128,
        duration: u64,
    },
    Restake {
        user: String,
        amount: Uint128,
        from: u64,
        to: u64,
    },
}

/// This structure describes the query messages available in the contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},

    #[returns(Addr)]
    Owner {},

    #[returns(UserRewardResponse)]
    Reward { user: String },
}

#[cw_serde]
pub struct LockingRewardConfig {
    pub duration: u64,
    pub multiplier: u64,
}

impl Default for LockingRewardConfig {
    fn default() -> Self {
        LockingRewardConfig {
            duration: 0u64,
            multiplier: 0u64,
        }
    }
}

impl Default for &LockingRewardConfig {
    fn default() -> Self {
        &LockingRewardConfig {
            duration: 0u64,
            multiplier: 0u64,
        }
    }
}

#[cw_serde]
pub struct Config {
    /// eclipASTRO token
    pub eclipastro: Addr,
    /// ECLIP token
    pub eclip: String,
    /// flexible staking pool
    pub flexible_staking: Addr,
    /// timelock staking pool
    pub timelock_staking: Addr,
    /// eclipASTRO reward contract
    pub token_converter: Addr,
    /// ECLIP daily reward
    pub eclip_daily_reward: Uint128,
    /// locking_reward_config
    pub locking_reward_config: Vec<LockingRewardConfig>,
}

#[cw_serde]
pub struct UpdateConfigMsg {
    /// eclipASTRO token
    pub eclipastro: Option<String>,
    /// ECLIP token
    pub eclip: Option<String>,
    /// flexible staking pool
    pub flexible_staking: Option<String>,
    /// timelock staking pool
    pub timelock_staking: Option<String>,
    /// eclipASTRO reward contract
    pub token_converter: Option<String>,
    /// ECLIP daily reward
    pub eclip_daily_reward: Option<Uint128>,
    /// locking_reward_config
    pub locking_reward_config: Option<Vec<LockingRewardConfig>>,
}

#[cw_serde]
pub struct UserRewardResponse {
    pub eclip: Uint128,
    pub eclipastro: Uint128,
}
