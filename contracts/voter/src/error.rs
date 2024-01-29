use cosmwasm_std::StdError;
use cw_controllers::AdminError;
use thiserror::Error;

/// ## Description
/// This enum describes registry contract errors!
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Admin(#[from] AdminError),
    
    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("Can't stake this token: {0}")]
    UnknownToken(String),

    #[error("Can't handle this message")]
    UnknownMessage(),
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}