use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Env, StdResult, Uint128, WasmMsg};

use crate::token::InstantiateMarketingInfo;

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract owner for updating
    pub owner: String,
    /// ASTRO token
    pub astro: String,
    /// xASTRO token
    pub xastro: String,
    /// ASTRO staking contract
    pub staking_contract: Addr,
    /// Eclipse treasury
    pub treasury: String,
    /// eclipASTRO token code id
    pub token_code_id: u64,
    /// eclipASTRO marketing info
    pub marketing: Option<InstantiateMarketingInfo>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// convert astro, xastro to eclipastro by user
    Convert {
        recipient: Option<String>,
    },
    /// update config
    UpdateConfig { config: UpdateConfig },
    /// update reward config
    UpdateRewardConfig { config: RewardConfig },
    /// update owner
    UpdateOwner { owner: String },
    /// claim reward
    Claim {},
    /// claim treasury reward
    ClaimTreasuryReward { amount: Uint128 },
    /// withdraw xASTRO
    WithdrawAvailableBalance { amount: Uint128, recipient: String },
    /// Callbacks; only callable by the contract itself.
    Callback(CallbackMsg),
}

#[cw_serde]
pub struct MigrateMsg {
    pub update_contract_name: Option<bool>,
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
    /// query rewards
    #[returns(RewardResponse)]
    Rewards {},
    /// query owner
    #[returns(Addr)]
    Owner {},
    /// query withdrawable xASTRO balance
    #[returns(Uint128)]
    WithdrawableBalance {},
}

#[cw_serde]
pub enum Cw20HookMsg {
    Convert {},
}

#[cw_serde]
pub struct UpdateConfig {
    /// Eclipse vxASTRO holder contract
    pub vxastro_holder: Option<Addr>,
    /// Eclipse treasury
    pub treasury: Option<Addr>,
    /// eclipASTRO / ASTRO stability pool
    pub stability_pool: Option<Addr>,
    /// eclipASTRO staking reward distributor
    pub staking_reward_distributor: Option<Addr>,
    /// cosmic essence reward distributor
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
pub struct RewardResponse {
    pub users_reward: Reward,
    pub ce_holders_reward: Reward,
    pub stability_pool_reward: Reward,
    pub treasury_reward: Reward,
}

#[cw_serde]
pub struct Reward {
    pub token: String,
    pub amount: Uint128,
}

#[cw_serde]
pub struct Config {
    /// ASTRO token
    pub astro: String,
    /// xASTRO token
    pub xastro: String,
    /// ASTRO staking contract
    pub staking_contract: Addr,
    /// eclipASTRO token
    pub eclipastro: Addr,
    /// Eclipse vxASTRO holder contract
    pub vxastro_holder: Option<Addr>,
    /// Eclipse treasury
    pub treasury: Addr,
    /// eclipASTRO / ASTRO stability pool
    pub stability_pool: Option<Addr>,
    /// eclipASTRO staking reward distributor
    pub staking_reward_distributor: Option<Addr>,
    /// cosmic essence reward distributor
    pub ce_reward_distributor: Option<Addr>,
}

#[cw_serde]
pub enum CallbackMsg {
    ConvertAstro {
        prev_xastro_balance: Uint128,
        astro_amount_to_convert: Uint128,
        receiver: String,
    }
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
