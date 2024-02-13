use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;

/// This structure describes the execute messages available in the contract.
#[cw_serde]
pub enum ExecuteMsg {
    /// Receive receives a message of type [`Cw20ReceiveMsg`] and processes it depending on the received template.
    Receive(Cw20ReceiveMsg),
}

/// This structure describes the query messages available in the contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Config returns the contract configuration specified in a custom [`ConfigResponse`] structure
    #[returns(ConfigResponse)]
    Config {},
    #[returns(Uint128)]
    TotalShares {},
    #[returns(Uint128)]
    TotalDeposit {},
}

#[cw_serde]
pub struct ConfigResponse {
    /// The ASTRO token address
    pub deposit_token_addr: Addr,
    /// The xASTRO token address
    pub share_token_addr: Addr,
}

/// This structure describes a CW20 hook message.
#[cw_serde]
pub enum Cw20HookMsg {
    /// Deposits ASTRO in exchange for xASTRO
    Enter {},
    /// Burns xASTRO in exchange for ASTRO
    Leave {},
}
