use cosmwasm_std::StdError;
use thiserror::Error;

use cw_controllers::AdminError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error(
        "Sender's CW20 token contract address {got} does not match one from config {expected}"
    )]
    Cw20AddressesNotMatch { got: String, expected: String },

    #[error("Amount {got} exceeds your staking {expected}")]
    ExeedingUnstakeAmount { got: u128, expected: u128 },

    #[error("No locking period found: {0}")]
    NoLockingPeriodFound(u64),

    #[error("No locked amount found")]
    NoLockedAmount {},
}
