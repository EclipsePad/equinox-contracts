use cosmwasm_std::StdError;
use cw_controllers::AdminError;
use cw_utils::{ParseReplyError, PaymentError};
use thiserror::Error;

/// ## Description
/// This enum describes registry contract errors!
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("Callbacks cannot be invoked externally")]
    InvalidCallbackInvoke {},

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("New version must be greater than previous one: {0}")]
    VersionErr(String),

    #[error("Can't stake this token: {0}")]
    UnknownToken(String),

    #[error("Can't handle this message")]
    UnknownMessage(),

    #[error("Error staking astro")]
    StakeError {},

    #[error("Unknown reply id: {0}")]
    UnknownReplyId(u64),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Zero amount")]
    ZeroAmount {},
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
