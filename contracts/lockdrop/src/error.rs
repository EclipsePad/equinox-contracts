use cosmwasm_std::StdError;
use cw_utils::{ParseReplyError, PaymentError};
use thiserror::Error;

use cw_controllers::AdminError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("Lockup assets already staked")]
    AlreadyStaked {},

    #[error("Withdraw allowes only one time during withdraw window")]
    AlreadyWithdrawed {},

    #[error("User is in blacklist")]
    Blacklisted {},

    #[error("Claim reward not allowed")]
    ClaimRewardNotAllowed {},

    #[error("Contract name must be same: {0}")]
    ContractNameErr(String),

    #[error("Deposit is only allowed in Deposit window")]
    NotDepositWindow {},

    #[error("Can't update deposit period after the deposit window ended")]
    DepositWindowUpdateDisabled {},

    #[error("Can't update deposit period to a past point")]
    DepositWindowUpdateErr {},

    #[error("Can't update withdraw period after the first half of withdrawal window ended")]
    WithdrawalWindowUpdateDisabled {},

    #[error("Can't update withdraw period to a past point")]
    WithdrawalWindowUpdateErr {},

    #[error("Ensure list contains unique assets")]
    DuplicatedAssets {},

    #[error("Must be done after Equinox is live and update config")]
    EquinoxNotLive {},

    #[error("Can't extend from duration {0} to duration {1}")]
    ExtendDurationErr(u64, u64),

    #[error("Extend Lockup only allowed on deposit phase or when Equinox is live")]
    ExtendLockupError {},

    #[error("amounts doesn't matched")]
    InvalidAmountCheck {},

    #[error("Asset is not allowed")]
    InvalidAsset {},

    #[error("Callbacks cannot be invoked externally")]
    InvalidCallbackInvoke {},

    #[error("Invalid denom {0}")]
    InvalidDenom(String),

    #[error("Invalid token balances for lp deposit")]
    InvalidDepositAmounts {},

    #[error("Duration {0} is not allowed to lockup")]
    InvalidDuration(u64),

    #[error("Invalid init_timestamp. Current timestamp : {0}")]
    InvalidInitTimestamp(u64),

    #[error("Duplicated duration or invalid reward multiplier")]
    InvalidLockConfig {},

    #[error("Invalid lp token balances")]
    InvalidLpTokenBalance {},

    #[error("Invalid multiplier bps {0}")]
    InvalidMultiplier(u64),

    #[error("Invalid Penalty bps {0}")]
    InvalidPenalty(u64),

    #[error("Invalid token balance")]
    InvalidTokenBalance {},

    #[error("Time window must be greater than 86400, but got {0}")]
    InvalidTimeWindow(u64),

    #[error("Lockdrop has ended")]
    LockdropEnded {},

    #[error("Lockdrop is ongoing")]
    LockdropNotEnded {},

    #[error("Tokens are not staked")]
    NotStaked {},

    #[error("Early Unlock is not allowed")]
    EarlyUnlockDisabled {},

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("New version must be greater than previous one: {0}")]
    VersionErr(String),

    #[error("Amount exceeds maximum allowed withdrawal limit of {0}")]
    WithdrawLimitExceed(String),

    #[error("Token amount must not be zero")]
    ZeroAmount {},
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
