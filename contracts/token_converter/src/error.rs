use cosmwasm_std::StdError;
use cw_controllers::AdminError;
use cw_utils::{ParseReplyError, PaymentError};
use thiserror::Error;

/// ## Description
/// This enum describes registry contract errors!
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("Contract name must be same: {0}")]
    ContractNameErr(String),

    #[error("Failed to parse or process reply message")]
    FailedToParseReply {},

    #[error("Callbacks cannot be invoked externally")]
    InvalidCallbackInvoke {},

    #[error("Balance is not enough")]
    NotEnoughBalance {},

    #[error("No reward claimable for user")]
    NoRewardClaimable {},

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("Error staking astro")]
    StakeError {},

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unknown reply id: {0}")]
    UnknownReplyId(u64),

    #[error("Total Reward point must be 10000")]
    RewardDistributionErr {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Can't handle this message")]
    UnknownMessage {},

    #[error("Can't stake this token: {0}")]
    UnknownToken(String),

    #[error("New version must be greater than previous one: {0}")]
    VersionErr(String),
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
