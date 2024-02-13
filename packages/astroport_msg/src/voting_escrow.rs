use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;

/// This structure describes the execute functions in the contract.
#[cw_serde]
pub enum ExecuteMsg {
    /// Receives a message of type [`Cw20ReceiveMsg`] and processes it depending on the received
    /// template.
    Receive(Cw20ReceiveMsg),
    /// Withdraw xASTRO from the vxASTRO contract
    Withdraw {
        amount: Uint128,
    },
}

/// This structure describes a CW20 hook message.
#[cw_serde]
pub enum Cw20HookMsg {
    /// Create a vxASTRO position and lock xASTRO 
    CreateLock {},
    /// Deposit xASTRO in another user's vxASTRO position
    DepositFor { user: String },
}
