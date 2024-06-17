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

    #[error("Empty voting list!")]
    EmptyVotingList,

    #[error("Voting list has pool addresses duplicaion!")]
    VotingListDuplication,

    #[error("Sum of weights is not equal one!")]
    WeightsAreUnbalanced,

    #[error("Weight is out of range!")]
    WeightIsOutOfRange,
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
