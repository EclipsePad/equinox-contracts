use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal256, Uint128};

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
    /// ASTRO/eclipASTRO converter contract
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
    FlexibleStakeClaim {
        user: String,
    },
    TimelockStakeClaim {
        user: String,
        duration: u64,
        locked_at: u64,
    },
    TimelockStakeClaimAll {
        user: String,
    },
    FlexibleUnstake {
        user: String,
        amount: Uint128,
    },
    TimelockUnstake {
        user: String,
        duration: u64,
        locked_at: u64,
    },
    Restake {
        user: String,
        from: u64,
        locked_at: u64,
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

    #[returns(TotalStakingData)]
    TotalStaking {},

    #[returns(Vec<(u64, Uint128)>)]
    PendingRewards {},
}

#[cw_serde]
pub struct MigrateMsg {
    pub update_contract_name: Option<bool>,
}

#[cw_serde]
#[derive(Default)]
pub struct LockingRewardConfig {
    pub duration: u64,
    pub multiplier: u64,
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
pub struct FlexibleReward {
    pub eclip: Uint128,
    pub eclipastro: Uint128,
}

#[cw_serde]
pub struct TimelockReward {
    pub duration: u64,
    pub locked_at: u64,
    pub eclip: Uint128,
    pub eclipastro: Uint128,
}

#[cw_serde]
pub struct UserRewardResponse {
    pub flexible: FlexibleReward,
    pub timelock: Vec<TimelockReward>,
}

#[cw_serde]
pub struct TotalStakingData {
    pub staking_data: Vec<StakingData>,
    pub reward_weight_eclipastro: Decimal256,
    pub reward_weight_eclip: Decimal256,
}

impl Default for TotalStakingData {
    fn default() -> Self {
        TotalStakingData {
            staking_data: vec![],
            reward_weight_eclipastro: Decimal256::zero(),
            reward_weight_eclip: Decimal256::zero(),
        }
    }
}

#[cw_serde]
pub struct StakingData {
    pub duration: u64,
    pub amount: Uint128,
}
