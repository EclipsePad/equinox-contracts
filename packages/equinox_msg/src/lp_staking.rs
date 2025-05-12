use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Decimal256, Env, StdResult, Uint128, WasmMsg};

use crate::single_sided_staking::UnbondedItem;

// #[cw_serde]
// pub struct MigrateMsg {
//     pub update_contract_name: Option<bool>,
//     pub update_rewards: Option<((u64, u64), Reward)>,
// }

#[cw_serde]
pub struct MigrateMsg {
    pub version: String,
}

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
    pub lockdrop: Option<String>,
    /// Eclipse treasury
    pub treasury: String,
    /// funding DAO
    pub funding_dao: String,
    /// blacklisted wallets
    pub blacklist: Option<Vec<String>>,
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
    UpdateRewardDistribution {
        distribution: RewardDistribution,
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

    Unbond {
        amount: Option<Uint128>,
        period: u64,
    },
    Withdraw {
        recipient: Option<String>,
    },

    AddRewards {
        from: Option<u64>,
        duration: Option<u64>,
        eclip: Uint128,
        beclip: Uint128,
    },
    ClaimBlacklistRewards {},
    AllowUsers {
        users: Vec<String>,
    },
    BlockUsers {
        users: Vec<String>,
    },

    RemoveFromBlacklist {
        user: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// query config
    #[returns(Config)]
    Config {},
    /// query reward config
    #[returns(Reward)]
    RewardDistribution {},
    /// query owner
    #[returns(Addr)]
    Owner {},
    /// query total_staking
    #[returns(Uint128)]
    TotalStaking {},
    /// query user_staking
    #[returns(UserStaking)]
    Staking { user: String },
    /// query unbonded user positions
    #[returns(Vec<UnbondedItem>)]
    Unbonded { user: String },
    /// query pending_rewards
    #[returns(Vec<RewardAmount>)]
    Reward { user: String },

    #[returns(Vec<RewardWeight>)]
    RewardWeights {},

    #[returns(Vec<RewardWeight>)]
    UserRewardWeights { user: String },

    #[returns(Vec<String>)]
    Blacklist {},
    // rewards of blacklist users which goes to the Equinox treasury
    #[returns(Vec<RewardAmount>)]
    BlacklistRewards,

    #[returns(bool)]
    IsAllowed { user: String },

    #[returns(Vec<((u64, u64), Reward)>)]
    RewardSchedule { from: Option<u64> },
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
    pub lp_contract: Option<String>,
    pub lockdrop: Option<String>,
    pub astroport_incentives: Option<String>,
    pub treasury: Option<String>,
    pub funding_dao: Option<String>,
    pub eclip: Option<String>,
    pub beclip: Option<String>,
}

#[cw_serde]
pub struct ConfigPre {
    /// lp token
    pub lp_token: AssetInfo,
    /// lp contract
    pub lp_contract: Addr,
    /// ASTRO token
    pub astro: String,
    /// xASTRO token
    pub xastro: String,
    /// ECLIP token
    pub eclip: String,
    /// bECLIP token
    pub beclip: Addr,
    /// ASTRO staking contract
    pub astro_staking: Addr,
    /// ECLIP staking
    pub eclip_staking: Addr,
    /// Astroport incentives
    pub astroport_incentives: Addr,
    pub treasury: Addr,
    pub funding_dao: Addr,
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
    /// ECLIP token
    pub eclip: String,
    /// bECLIP token
    pub beclip: Addr,
    /// ASTRO staking contract
    pub astro_staking: Addr,
    /// ECLIP staking
    pub eclip_staking: Addr,
    pub lockdrop: Addr,
    /// Astroport incentives
    pub astroport_incentives: Addr,
    pub treasury: Addr,
    pub funding_dao: Addr,
}

#[cw_serde]
pub struct RewardDistribution {
    /// users' reward in basis point
    pub users: u32,
    /// treasury reward in basis point
    pub treasury: u32,
    /// funding dao
    pub funding_dao: u32,
}

#[cw_serde]
pub struct Reward {
    pub eclip: Uint128,
    pub beclip: Uint128,
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
