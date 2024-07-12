use cosmwasm_std::StdError;
use cw_utils::PaymentError;
use thiserror::Error;

/// ## Description
/// This enum describes registry contract errors!
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("Callbacks cannot be invoked externally")]
    InvalidCallbackInvoke {},

    #[error("Asset is not allowed")]
    InvalidAsset {},

    #[error("Expected {0} or {1}, got {2}")]
    InvalidCoinAsset(String, String, String),

    #[error("Invalid token balance")]
    InvalidTokenBalance {},

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Token amount must not be zero")]
    ZeroAmount {},
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
