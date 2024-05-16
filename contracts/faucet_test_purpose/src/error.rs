use cosmwasm_std::StdError;
use cw_controllers::AdminError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("{0}")]
    Std(#[from] StdError),
}