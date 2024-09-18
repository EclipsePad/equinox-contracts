use cosmwasm_std::StdError;
use cw_utils::{ParseReplyError, PaymentError};
use thiserror::Error;

use cw_controllers::AdminError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("Amount {got} doesn't match with arguments {expected}")]
    AmountNotMatch { got: u128, expected: u128 },

    #[error("Sender's asset denom {got} does not match one from config {expected}")]
    AssetsNotMatch { got: String, expected: String },

    #[error("Contract name must be same: {0}")]
    ContractNameErr(String),

    #[error(
        "Sender's CW20 token contract address {got} does not match one from config {expected}"
    )]
    Cw20AddressesNotMatch { got: String, expected: String },

    #[error("Ensure list contains unique assets")]
    DuplicatedAssets {},

    #[error("Amount {got} exceeds your staking {expected}")]
    ExeedingUnstakeAmount { got: u128, expected: u128 },

    #[error("Parameter expires_in cannot be higher than {0}")]
    ExpiresInErr(u64),

    #[error("Invalid asset")]
    InvalidAsset {},

    #[error("Callbacks cannot be invoked externally")]
    InvalidCallbackInvoke {},

    #[error("Invalid denom {0}")]
    InvalidDenom(String),

    #[error("Invalid reward end time")]
    InvalidEndTime {},

    #[error("Start time must be greater thatn {expect}, but got {got}")]
    InvalidStartTime { got: u64, expect: u64 },

    #[error("Staking amount is zero")]
    InvalidStakingAmount {},

    #[error("Ownership proposal expired")]
    OwnershipProposalExpired {},

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("Total Reward point must be 10000")]
    RewardDistributionErr {},

    #[error("New owner cannot be same")]
    SameOwner {},

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
