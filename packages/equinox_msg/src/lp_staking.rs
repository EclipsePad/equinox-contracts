use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Decimal256, Env, StdResult, Uint128, WasmMsg};

#[cw_serde]
pub struct InstantiateMsg {
    /// contract owner
    pub owner: Option<String>,
    /// lp token
    pub lp_token: AssetInfo,
    /// lp contract
    pub lp_contract: String,
    /// ASTRO token
    pub astro: String,
    /// xASTRO token
    pub xastro: String,
    /// ECLIP token
    pub eclip: String,
    /// bECLIP token
    pub beclip: String,
    /// astro staking contract
    pub astro_staking: String,
    /// ECLIP staking
    pub eclip_staking: String,
    /// Astroport incentives
    pub astroport_incentives: String,
    /// Eclipse treasury. send 67.5% of 20% of incentives rewards
    pub treasury: String,
    /// eclipASTRO / xASTRO stability pool. send xastro converted from 12.5% of 20% of incentives rewards
    pub stability_pool: String,
    /// cosmic essence reward distributor. send 20% of 20% of incentives rewards
    pub ce_reward_distributor: String,
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
    /// Change reward config
    UpdateRewardConfig {
        distribution: Option<RewardDistribution>,
        reward_end_time: Option<u64>,
        details: Option<RewardDetails>,
    },
    /// Claim rewards of user.
    Claim {
        assets: Option<Vec<AssetInfo>>,
    },
    /// Callbacks; only callable by the contract itself.
    Callback(CallbackMsg),
    Unstake {
        amount: Uint128,
        recipient: Option<String>,
    },
    Stake {
        recipient: Option<String>,
    },
    // UpdateUserRewardWeight {},
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
    /// query user_staking
    #[returns(UserStaking)]
    Staking { user: String },
    /// query pending_rewards
    #[returns(Vec<RewardAmount>)]
    Reward { user: String },

    #[returns(Vec<RewardWeight>)]
    RewardWeights {},
    #[returns(Vec<RewardWeight>)]
    UserRewardWeights { user: String },
}

#[cw_serde]
pub struct MigrateMsg {
    pub update_contract_name: Option<bool>,
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Stake eclipASTRO token
    Stake { recipient: Option<String> },
}

#[cw_serde]
pub enum CallbackMsg {
    DistributeEclipseRewards { assets: Vec<Asset> },
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
pub struct UpdateConfigMsg {
    pub lp_token: Option<AssetInfo>,
    pub lp_contract: Option<Addr>,
    pub astroport_incentives: Option<Addr>,
    pub treasury: Option<Addr>,
    pub stability_pool: Option<Addr>,
    pub ce_reward_distributor: Option<Addr>,
}

#[cw_serde]
pub struct Config {
    /// lp token
    pub lp_token: AssetInfo,
    /// lp contract
    pub lp_contract: Addr,
    /// ASTRO token
    pub astro: String,
    /// xASTRO token
    pub xastro: String,
    /// ASTRO staking contract
    pub astro_staking: Addr,
    /// ECLIP staking
    pub eclip_staking: Addr,
    /// Astroport incentives
    pub astroport_incentives: Addr,
    pub treasury: Addr,
    pub stability_pool: Addr,
    pub ce_reward_distributor: Addr,
}

#[cw_serde]
pub struct RewardDistribution {
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
pub struct RewardConfig {
    pub distribution: RewardDistribution,
    pub reward_end_time: u64,
    pub details: RewardDetails,
}

#[cw_serde]
pub struct LpRewards {
    pub eclip: Uint128,
    pub astro: Uint128,
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
pub struct RewardWeight {
    pub info: AssetInfo,
    pub reward_weight: Decimal256,
}

#[cw_serde]
pub struct RewardAmount {
    pub info: AssetInfo,
    pub amount: Uint128,
}

#[cw_serde]
pub struct UserStaking {
    pub staked: Uint128,
    pub reward_weights: Vec<RewardWeight>,
}

impl Default for UserStaking {
    fn default() -> Self {
        UserStaking {
            staked: Uint128::zero(),
            reward_weights: vec![],
        }
    }
}

#[cw_serde]
pub struct UserAstroportReward {
    pub asset: AssetInfo,
    pub amount: Uint128,
    pub reward_weight: Decimal256,
}

/// This structure describes the parameters used for creating a request for a change of contract ownership.
#[cw_serde]
pub struct OwnershipProposal {
    /// The newly proposed contract owner
    pub owner: Addr,
    /// Time until the proposal to change ownership expires
    pub ttl: u64,
}
