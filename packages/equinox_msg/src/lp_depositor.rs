use astroport::asset::Asset;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Env, StdResult, WasmMsg};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct Config {
    /// ASTRO token
    pub astro: String,
    /// xASTRO token
    pub xastro: String,
    /// eclipASTRO token
    pub eclipastro: Addr,
    /// ASTRO staking contract
    pub staking_contract: Addr,
    /// eclipASTRO converter contract
    pub converter_contract: Addr,
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
    pub eclipastro: Addr,
    /// ASTRO staking contract
    pub staking_contract: Addr,
    /// eclipASTRO converter contract
    pub converter_contract: Addr,
    /// liquidity_pool
    pub lp_contract: Addr,
    /// lp_token
    pub lp_token: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// convert and deposit astro, xastro to lp by user
    Convert {
        recipient: Option<String>,
    },
    /// convert and deposit eclipASTRO to lp by users
    Receive(Cw20ReceiveMsg),
    Callback(CallbackMsg),
}

#[cw_serde]
pub enum QueryMsg {
    Config {},
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
