use astroport::asset::Asset;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Env, StdResult, Uint128, WasmMsg};

#[cw_serde]
pub struct Config {
    /// ASTRO token
    pub astro: String,
    /// xASTRO token
    pub xastro: String,
    /// eclipASTRO token
    pub eclipastro: String,
    /// ASTRO staking contract
    pub staking_contract: Addr,
    /// eclipASTRO converter contract
    pub voter: Addr,
    /// liquidity_pool
    pub lp_contract: Addr,
    /// lp_token
    pub lp_token: String,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// ASTRO token
    pub astro: String,
    /// xASTRO token
    pub xastro: String,
    /// eclipASTRO token
    pub eclipastro: String,
    /// ASTRO staking contract
    pub staking_contract: String,
    /// eclipASTRO converter contract
    pub voter: String,
    /// liquidity_pool
    pub lp_contract: String,
    /// lp_token
    pub lp_token: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// convert and deposit astro, xastro to lp by user
    Convert {
        recipient: Option<String>,
    },
    Callback(CallbackMsg),
}

#[cw_serde]
pub enum MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// query config
    #[returns(Config)]
    Config {},

    #[returns(Uint128)]
    Simulate { asset: Asset },
}
#[cw_serde]
pub enum Cw20HookMsg {
    /// convert and deposit eclipastro to lp by user
    Convert { recipient: Option<String> },
}

#[cw_serde]
pub enum CallbackMsg {
    DepositIntoPool { recipient: String },
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
