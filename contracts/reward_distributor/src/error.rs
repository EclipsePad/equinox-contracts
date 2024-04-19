use cosmwasm_std::StdError;
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

    #[error("Amount {got} exceeds your staking {expected}")]
    ExeedingUnstakeAmount { got: u128, expected: u128 },

    #[error("Duration is not allowed: {0}")]
    InvalidDuration(u64),

    #[error("No reward to claim")]
    NoReward {},

    #[error("Balance is not enough")]
    NotEnoughBalance {},

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

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
