use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;

use crate::reward_distributor::TimelockReward;

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract owner for updating
    pub owner: String,
    /// eclipASTRO token
    pub token: String,
    /// timelock config
    pub timelock_config: Option<Vec<TimeLockConfig>>,
}

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
    /// This accepts a properly-encoded ReceiveMsg from a cw20 contract
    Receive(Cw20ReceiveMsg),
    /// Claim rewards of user.
    Claim {
        duration: u64,
        locked_at: u64,
    },
    ClaimAll {},
    Unstake {
        duration: u64,
        locked_at: u64,
        amount: Option<Uint128>,
    },
    /// update locking period from short one to long one
    Restake {
        from_duration: u64,
        locked_at: u64,
        to_duration: u64,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// query config
    #[returns(Config)]
    Config {},
    /// query owner
    #[returns(Addr)]
    Owner {},
    /// query total_staking
    #[returns(Uint128)]
    TotalStaking {},
    /// query total_staking_by_duration
    #[returns(Vec<StakingWithDuration>)]
    TotalStakingByDuration {},
    /// query user_staking
    #[returns(Vec<UserStaking>)]
    Staking { user: String },
    /// query pending_rewards
    #[returns(Vec<TimelockReward>)]
    Reward { user: String },
    /// query calculating penalty
    #[returns(Uint128)]
    CalculatePenalty {
        amount: Uint128,
        duration: u64,
        locked_at: u64,
    },
}

#[cw_serde]
pub struct MigrateMsg {
    pub update_contract_name: Option<bool>,
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// timelock eclipASTRO token
    Lock { duration: u64 },
}

#[cw_serde]
pub struct UpdateConfigMsg {
    pub token: Option<String>,
    pub reward_contract: Option<String>,
    pub timelock_config: Option<Vec<TimeLockConfig>>,
}

#[cw_serde]
pub struct Config {
    /// eclipASTRO token
    pub token: Addr,
    /// reward_contract address
    pub reward_contract: Addr,
    /// lock config
    pub timelock_config: Vec<TimeLockConfig>,
}

#[cw_serde]
pub struct TimeLockConfig {
    pub duration: u64,
    pub early_unlock_penalty_bps: u16,
}

#[cw_serde]
pub struct UserStakingByDuration {
    pub amount: Uint128,
    pub locked_at: u64,
}

#[cw_serde]
pub struct UserStaking {
    pub duration: u64,
    pub staking: Vec<UserStakingByDuration>,
}

#[cw_serde]
pub struct StakingWithDuration {
    pub amount: Uint128,
    pub duration: u64,
}
