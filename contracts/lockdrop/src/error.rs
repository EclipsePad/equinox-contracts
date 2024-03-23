use cosmwasm_std::StdError;
use thiserror::Error;

use cw_controllers::AdminError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("Already allowed")]
    AlreadyAllowed {},

    #[error("{0} is already set")]
    AlreadySet(String),

    #[error("Lockup assets already staked")]
    AlreadyStaked {},

    #[error("Already unlocked")]
    AlreadyUnlocked {},

    #[error("Withdraw allowes only one time during withdraw window")]
    AlreadyWithdrawed {},

    #[error("Claim reward not allowed")]
    ClaimRewardNotAllowed {},

    #[error("Contract name must be same: {0}")]
    ContractNameErr(String),

    #[error("Deposit window is closed")]
    DepositWindowClosed {},

    #[error("Deposit window is not started")]
    DepositWindowNotStarted {},

    #[error("Failed to parse or process reply message")]
    FailedToParseReply {},

    #[error("Insufficient ASTRO/xASTRO amount in the contract")]
    InsufficientAmountInContract {},

    #[error("Asset {0} is not allowed to lockup")]
    InvalidLockupAsset(String),

    #[error("Callbacks cannot be invoked externally")]
    InvalidCallbackInvoke {},

    #[error("Invalid token balances for lp deposit")]
    InvalidDepositAmounts {},

    #[error("Invalid token balance")]
    InvalidTokenBalance {},

    #[error("Invalid lp token balances")]
    InvalidLpTokenBalance {},

    #[error("Duration {0} is not allowed to lockup")]
    InvalidDuration(u64),

    #[error("Invalid init_timestamp. Current timestamp : {0}")]
    InvalidInitTimestamp(u64),

    #[error("The asset is not staked into lp staking vault")]
    LpStakingNotHappend {},

    #[error("Tokens are not staked")]
    NotStaked {},

    #[error("Only {expected} is allowed: received {got}")]
    OnlyEclipAllowed { expected: String, got: String },

    #[error("Ownership proposal expired")]
    OwnershipProposalExpired {},

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("The asset is not staked into staking vault")]
    StakingNotHappend {},

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unknown reply id: {0}")]
    UnknownReplyId(u64),

    #[error("New version must be greater than previous one: {0}")]
    VersionErr(String),

    #[error("{0} seconds to unlock")]
    WaitToUnlock(u64),

    #[error("Lockdrop is not finished yet")]
    LockdropNotFinished {},

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