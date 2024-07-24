use cosmwasm_std::StdError;
use cw_utils::PaymentError;
use thiserror::Error;

use cw_controllers::AdminError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("Contract name must be same: {0}")]
    ContractNameErr(String),

    #[error(
        "Sender's CW20 token contract address {got} does not match one from config {expected}"
    )]
    Cw20AddressesNotMatch { got: String, expected: String },

    #[error("Duplicated address: {0}")]
    DuplicatedAddress(String),

    #[error("Ensure list contains unique assets")]
    DuplicatedAssets {},

    #[error("Amount {got} exceeds your staking {expected}")]
    ExeedingUnstakeAmount { got: u128, expected: u128 },

    #[error("Can't extend from duration {0} to duration {1}")]
    ExtendDurationErr(u64, u64),

    #[error("Exceed locked amount")]
    ExceedAmount {},

    #[error("Invalid asset")]
    InvalidAsset {},

    #[error("Callbacks cannot be invoked externally")]
    InvalidCallbackInvoke {},

    #[error("No locking period found: {0}")]
    NoLockingPeriodFound(u64),

    #[error("No locked amount found")]
    NoLockedAmount {},

    #[error("Not allowed address: {0}")]
    NotAllowed(String),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("New version must be greater than previous one: {0}")]
    VersionErr(String),

    #[error("Token amount must not be zero")]
    ZeroAmount {},
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
