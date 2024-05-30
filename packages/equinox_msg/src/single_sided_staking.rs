use astroport::asset::AssetInfo;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Decimal256, Env, StdResult, Uint128, WasmMsg};
use cw20::Cw20ReceiveMsg;
#[cw_serde]
pub struct InstantiateMsg {
    /// Contract owner for updating
    pub owner: Addr,
    /// eclipASTRO token
    pub token: Addr,
    pub rewards: RewardConfig,
    /// timelock config
    pub timelock_config: Option<Vec<TimeLockConfig>>,
    /// ASTRO/eclipASTRO converter contract
    pub token_converter: Addr,
    pub treasury: Addr,
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
        locked_at: Option<u64>,
    },
    ClaimAll {
        with_flexible: bool,
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
    #[returns(Vec<UserRewardByDuration>)]
    Reward { user: String },
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
pub enum Cw20HookMsg {
    /// timelock eclipASTRO token
    Stake {
        lock_duration: u64,
        recipient: Option<String>,
    },
    Restake {
        from_duration: u64,
        locked_at: Option<u64>,
        amount: Option<Uint128>,
        to_duration: u64,
        recipient: Option<String>,
    },
}

#[cw_serde]
pub struct UpdateConfigMsg {
    pub token: Option<String>,
    pub timelock_config: Option<Vec<TimeLockConfig>>,
    pub token_converter: Option<Addr>,
    pub rewards: Option<RewardConfig>,
    pub treasury: Option<Addr>,
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
    pub token: Addr,
    pub rewards: RewardConfig,
    /// lock config
    pub timelock_config: Vec<TimeLockConfig>,
    /// ASTRO/eclipASTRO converter contract
    pub token_converter: Addr,
    pub treasury: Addr,
}

#[cw_serde]
pub struct RewardConfig {
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
pub struct RewardWeights {
    pub eclipastro: Decimal256,
    pub beclip: Decimal256,
    pub eclip: Decimal256,
}

impl Default for RewardWeights {
    fn default() -> Self {
        RewardWeights {
            eclip: Decimal256::zero(),
            eclipastro: Decimal256::zero(),
            beclip: Decimal256::zero(),
        }
    }
}
#[cw_serde]
pub struct UserStaked {
    pub staked: Uint128,
    pub reward_weights: RewardWeights,
}

#[cw_serde]
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
    pub locked_at: Option<u64>,
}

#[cw_serde]
pub struct RestakeData {
    pub from_duration: u64,
    pub locked_at: u64,
    pub amount: Option<Uint128>,
    pub to_duration: u64,
    pub add_amount: Option<Uint128>,
    pub sender: String,
    pub recipient: String,
}

#[cw_serde]
pub struct StakingWithDuration {
    pub amount: Uint128,
    pub duration: u64,
}

pub struct AstroStaking {
    pub total_shares: Uint128,
    pub total_deposit: Uint128,
}
