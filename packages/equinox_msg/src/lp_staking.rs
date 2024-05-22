use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Decimal256, Env, StdResult, Uint128, WasmMsg};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {
    /// lp token
    pub lp_token: Addr,
    /// lp contract
    pub lp_contract: Addr,
    /// bECLIP token
    pub beclip: AssetInfo,
    /// ASTRO token
    pub astro: String,
    /// xASTRO token
    pub xastro: String,
    /// astro staking contract
    pub astro_staking: Addr,
    /// eclipASTRO converter
    pub converter: Addr,
    /// bECLIP daily reward
    pub beclip_daily_reward: Option<Uint128>,
    /// Astroport generator
    pub astroport_generator: Addr,
    /// Eclipse treasury. send 67.5% of 20% of generator rewards
    pub treasury: Addr,
    /// eclipASTRO / xASTRO stability pool. send xastro converted from 12.5% of 20% of generator rewards
    pub stability_pool: Option<Addr>,
    /// cosmic essence reward distributor. send 20% of 20% of generator rewards
    pub ce_reward_distributor: Option<Addr>,
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
    pub lp_token: Option<Addr>,
    pub lp_contract: Option<Addr>,
    pub beclip: Option<AssetInfo>,
    pub beclip_daily_reward: Option<Uint128>,
    pub converter: Option<Addr>,
    pub astroport_generator: Option<Addr>,
    pub treasury: Option<Addr>,
    pub stability_pool: Option<Addr>,
    pub ce_reward_distributor: Option<Addr>,
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
    /// bECLIP token
    pub beclip: AssetInfo,
    /// ASTRO token
    pub astro: String,
    /// xASTRO token
    pub xastro: String,
    /// ASTRO staking contract
    pub astro_staking: Addr,
    /// eclipASTRO converter
    pub converter: Addr,
    /// bECLIP daily reward
    pub beclip_daily_reward: Uint128,
    /// Astroport generator
    pub astroport_generator: Addr,
    pub treasury: Addr,
    pub stability_pool: Option<Addr>,
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
