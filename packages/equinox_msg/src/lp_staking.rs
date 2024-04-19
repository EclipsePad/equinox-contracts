use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Decimal256, Env, StdResult, Uint128, WasmMsg};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract owner for updating
    pub owner: String,
    /// lp token
    pub lp_token: String,
    /// lp contract
    pub lp_contract: String,
    /// ECLIP token
    pub eclip: String,
    /// ASTRO token
    pub astro: String,
    /// xASTRO token
    pub xastro: String,
    /// astro staking contract
    pub astro_staking: String,
    /// eclipASTRO converter
    pub converter: String,
    /// ECLIP daily reward
    pub eclip_daily_reward: Option<Uint128>,
    /// Astroport generator
    pub astroport_generator: String,
    /// Eclipse treasury. send 67.5% of 20% of generator rewards
    pub treasury: String,
    /// eclipASTRO / xASTRO stability pool. send xastro converted from 12.5% of 20% of generator rewards
    pub stability_pool: String,
    /// cosmic essence reward distributor. send 20% of 20% of generator rewards
    pub ce_reward_distributor: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Change the owner
    UpdateOwner { owner: String },
    /// Change config
    UpdateConfig { config: UpdateConfigMsg },
    /// Change reward config
    UpdateRewardConfig { config: RewardConfig },
    /// This accepts a properly-encoded ReceiveMsg from a cw20 contract
    Receive(Cw20ReceiveMsg),
    /// Claim rewards of user.
    Claim {},
    /// Callbacks; only callable by the contract itself.
    Callback(CallbackMsg),
    Unstake {
        amount: Uint128,
        recipient: Option<String>,
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
    #[returns(TotalStaking)]
    TotalStaking {},
    /// query user_staking
    #[returns(UserStaking)]
    Staking { user: String },
    /// query pending_rewards
    #[returns(Vec<UserRewardResponse>)]
    Reward { user: String },
}

#[cw_serde]
pub struct MigrateMsg {
    pub update_contract_name: Option<bool>,
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Stake eclipASTRO token
    Stake {},
}

#[cw_serde]
pub enum CallbackMsg {
    Claim { user: String },
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
    pub lp_token: Option<String>,
    pub lp_contract: Option<String>,
    pub eclip: Option<String>,
    pub eclip_daily_reward: Option<Uint128>,
    pub converter: Option<String>,
    pub astroport_generator: Option<String>,
    pub treasury: Option<String>,
    pub stability_pool: Option<String>,
    pub ce_reward_distributor: Option<String>,
}

#[cw_serde]
pub struct UpdateRewardConfigMsg {
    pub users: Option<u32>,
    pub treasury: Option<u32>,
    pub ce_holders: Option<u32>,
    pub stability_pool: Option<u32>,
}

#[cw_serde]
pub struct Config {
    /// lp token
    pub lp_token: Addr,
    /// lp contract
    pub lp_contract: Addr,
    /// ECLIP token
    pub eclip: String,
    /// ASTRO token
    pub astro: Addr,
    /// xASTRO token
    pub xastro: Addr,
    /// ASTRO staking contract
    pub astro_staking: Addr,
    /// eclipASTRO converter
    pub converter: Addr,
    /// ECLIP daily reward
    pub eclip_daily_reward: Uint128,
    /// Astroport generator
    pub astroport_generator: Addr,
    pub treasury: Addr,
    pub stability_pool: Addr,
    pub ce_reward_distributor: Option<Addr>,
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
pub struct LpRewards {
    pub eclip: Uint128,
    pub astro: Uint128,
}

#[cw_serde]
pub struct TotalStaking {
    pub total_staked: Uint128,
    pub astroport_reward_weights: Vec<AstroportRewardWeight>,
    pub eclip_reward_weight: Decimal256,
}

impl Default for TotalStaking {
    fn default() -> Self {
        TotalStaking {
            total_staked: Uint128::zero(),
            astroport_reward_weights: vec![],
            eclip_reward_weight: Decimal256::zero(),
        }
    }
}

#[cw_serde]
pub struct UserStaking {
    pub staked: Uint128,
    pub astroport_rewards: Vec<UserAstroportReward>,
    pub eclip_reward_weight: Decimal256,
    pub pending_eclip_rewards: Uint128,
}

impl Default for UserStaking {
    fn default() -> Self {
        UserStaking {
            staked: Uint128::zero(),
            astroport_rewards: vec![],
            eclip_reward_weight: Decimal256::zero(),
            pending_eclip_rewards: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct AstroportRewardWeight {
    pub asset: AssetInfo,
    pub reward_weight: Decimal256,
}

#[cw_serde]
pub struct UserAstroportReward {
    pub asset: AssetInfo,
    pub amount: Uint128,
    pub reward_weight: Decimal256,
}

#[cw_serde]
pub struct UserRewardResponse {
    pub asset: AssetInfo,
    pub amount: Uint128,
}
