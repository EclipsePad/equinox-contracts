use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;

use crate::reward_distributor::UserRewardResponse;

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract owner for updating
    pub owner: String,
    /// eclipASTRO token
    pub token: String,
    /// timelock config
    pub timelock_config: Vec<TimeLockConfig>,
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
    Claim {},
    Unstake {
        duration: u64,
        locked_at: u64,
    },
    /// update locking period from short one to long one
    Restake {
        from_duration: u64,
        locked_at: u64,
        to_duration: u64,
    }
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
    /// query user_staking
    #[returns(Vec<UserStaking>)]
    Staking { user: String },
    /// query pending_rewards
    #[returns(UserRewardResponse)]
    Reward { user: String },
    /// query calculating penalty
    #[returns(Uint128)]
    CalculatePenalty { amount: Uint128, duration: u64, locked_at: u64 },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum Cw20HookMsg {
    /// timelock eclipASTRO token
    Lock {
        duration: u64,
    },
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
