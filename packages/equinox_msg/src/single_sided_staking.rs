use astroport::asset::AssetInfo;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Env, StdResult, Uint128, WasmMsg};
#[cw_serde]
pub struct InstantiateMsg {
    /// Contract owner for updating
    pub owner: String,
    /// eclipASTRO token
    pub token: String,
    /// ECLIP token
    pub eclip: String,
    /// bECLIP token
    pub beclip: String,
    /// timelock config
    pub timelock_config: Option<Vec<TimeLockConfig>>,
    /// ASTRO/eclipASTRO converter contract
    pub voter: String,
    pub treasury: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Change the owner
    ProposeNewOwner {
        owner: String,
        expires_in: u64,
    },
    DropOwnershipProposal {},
    ClaimOwnership {},
    /// Change config
    UpdateConfig {
        config: UpdateConfigMsg,
    },
    /// Update reward config
    UpdateRewardConfig {
        details: Option<RewardDetails>,
        reward_end_time: Option<u64>,
    },
    /// Claim rewards of user.
    Claim {
        duration: u64,
        locked_at: Option<u64>,
        assets: Option<Vec<AssetInfo>>,
    },
    ClaimAll {
        with_flexible: bool,
        assets: Option<Vec<AssetInfo>>,
    },
    Callback(CallbackMsg),
    Stake {
        duration: u64,
        recipient: Option<String>,
    },
    Unstake {
        duration: u64,
        locked_at: Option<u64>,
        amount: Option<Uint128>,
        recipient: Option<String>,
    },
    /// update locking period from short one to long one
    Restake {
        from_duration: u64,
        locked_at: Option<u64>,
        amount: Option<Uint128>,
        to_duration: u64,
        recipient: Option<String>,
    },
    AllowUsers {
        users: Vec<String>,
    },
    BlockUsers {
        users: Vec<String>,
    },
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
    /// query owner
    #[returns(Addr)]
    Owner {},
    /// query total_staking
    #[returns(Uint128)]
    TotalStaking {},
    /// query total_staking_by_duration
    #[returns(Vec<StakingWithDuration>)]
    TotalStakingByDuration { timestamp: Option<u64> },
    /// query user_staking
    #[returns(Vec<UserStaking>)]
    Staking { user: String },
    /// query pending_rewards
    #[returns(UserReward)]
    Reward {
        user: String,
        duration: u64,
        locked_at: u64,
    },
    /// query calculate reward
    #[returns(UserReward)]
    CalculateReward {
        amount: Uint128,
        duration: u64,
        locked_at: Option<u64>,
        from: u64,
        to: Option<u64>,
    },
    /// query calculating penalty
    #[returns(Uint128)]
    CalculatePenalty {
        amount: Uint128,
        duration: u64,
        locked_at: u64,
    },
    #[returns(bool)]
    IsAllowed { user: String },
    #[returns(Vec<(u64, Uint128)>)]
    EclipastroRewards {},
}

#[cw_serde]
pub struct MigrateMsg {
    pub update_contract_name: Option<bool>,
}

#[cw_serde]
pub struct UpdateConfigMsg {
    pub timelock_config: Option<Vec<TimeLockConfig>>,
    pub voter: Option<String>,
    pub treasury: Option<String>,
}

#[cw_serde]
pub enum CallbackMsg {
    Convert {
        prev_eclipastro_balance: Uint128,
        duration: u64,
        sender: String,
        recipient: String,
    },
}

impl CallbackMsg {
    pub fn to_cosmos_msg(self, env: &Env) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::Callback(self))?,
            funds: vec![],
        }))
    }
}

#[cw_serde]
pub struct Config {
    /// eclipASTRO token
    pub token: String,
    /// lock config
    pub timelock_config: Vec<TimeLockConfig>,
    /// ASTRO/eclipASTRO converter contract
    pub voter: Addr,
    pub treasury: Addr,
}

#[cw_serde]
pub struct RewardConfig {
    pub details: RewardDetails,
    pub reward_end_time: Option<u64>,
}

#[cw_serde]
pub struct RewardDetails {
    pub eclip: RewardDetail,
    pub beclip: RewardDetail,
}

#[cw_serde]
pub struct RewardDetail {
    pub info: AssetInfo,
    pub daily_reward: Uint128,
}

#[cw_serde]
pub struct VaultRewards {
    pub eclip: Uint128,
    pub beclip: Uint128,
}

#[cw_serde]
pub struct TimeLockConfig {
    pub duration: u64,
    pub early_unlock_penalty_bps: u64,
    pub reward_multiplier: u64,
}
#[cw_serde]
#[derive(Default)]
pub struct UserReward {
    pub eclipastro: Uint128,
    pub beclip: Uint128,
    pub eclip: Uint128,
}

#[cw_serde]
pub struct UserRewardByLockedAt {
    pub locked_at: u64,
    pub rewards: UserReward,
}

#[cw_serde]
pub struct UserRewardByDuration {
    pub duration: u64,
    pub rewards: Vec<UserRewardByLockedAt>,
}

#[cw_serde]
pub struct UserStaking {
    pub duration: u64,
    pub staking: Vec<UserStakingByDuration>,
}

#[cw_serde]
pub struct UserStakingByDuration {
    pub amount: Uint128,
    pub locked_at: u64,
}

#[cw_serde]
pub struct RestakeData {
    pub from_duration: u64,
    pub locked_at: u64,
    pub amount: Option<Uint128>,
    pub to_duration: u64,
    pub sender: String,
    pub recipient: String,
}

#[cw_serde]
pub struct StakingWithDuration {
    pub staked: Uint128,
    pub valid_staked: Uint128,
    pub duration: u64,
}

pub struct AstroStaking {
    pub total_shares: Uint128,
    pub total_deposit: Uint128,
}

/// This structure describes the parameters used for creating a request for a change of contract ownership.
#[cw_serde]
pub struct OwnershipProposal {
    /// The newly proposed contract owner
    pub owner: Addr,
    /// Time until the proposal to change ownership expires
    pub ttl: u64,
}
