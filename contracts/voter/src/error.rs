use cosmwasm_std::StdError;
use cw_controllers::AdminError;
use thiserror::Error;

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}

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
    UnknownMessage,

    #[error("Error staking astro")]
    StakeError,

    #[error("Unknown reply id: {0}")]
    UnknownReplyId(u64),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("The contract is under maintenance")]
    ContractIsPaused,

    #[error("It's too early to claim rewards")]
    ClaimRewardsEarly,

    #[error("Wrong rewards claim stage")]
    WrongRewardsClaimStage,

    #[error("Await completing rewards claim stage")]
    AwaitSwappedStage,

    #[error("Unequal pools")]
    UnequalPools,

    #[error("Last vote results aren't found")]
    LastVoteResultsAreNotFound,

    #[error("Rewards aren't found")]
    RewardsAreNotFound,

    #[error("Event isn't found")]
    EventIsNotFound,

    #[error("Attribute isn't found")]
    AttributeIsNotFound,

    #[error("Reply ID counter overflow")]
    ReplyIdCounterOverflow,

    #[error("Delegator is not found")]
    DelegatorIsNotFound,

    #[error("User is not found")]
    UserIsNotFound,

    #[error("Pool isn't whitelisted")]
    PoolIsNotWhitelisted,

    #[error("Voting period isn't started")]
    VotingDelay,

    #[error("Epoch is completed")]
    EpochEnd,

    #[error("New epoch isn't started yet")]
    EpochIsNotStarted,

    #[error("It's impossible to delegate twice")]
    DelegateTwice,

    #[error("Delegator can't place vote")]
    DelegatorCanNotVote,

    #[error("Zero amount")]
    ZeroAmount,

    #[error("Empty voting list!")]
    EmptyVotingList,

    #[error("Voting list has pool addresses duplicaion!")]
    VotingListDuplication,

    #[error("Sum of weights is not equal one!")]
    WeightsAreUnbalanced,

    #[error("Weight is out of range!")]
    WeightIsOutOfRange,

    #[error("It's too late to accept admin role!")]
    TransferAdminDeadline,

    #[error("Parsing previous version error!")]
    ParsingPrevVersion,

    #[error("Parsing new version error!")]
    ParsingNewVersion,

    #[error("Msg version is not equal contract new version!")]
    ImproperMsgVersion,

    #[error("No astro staking rewards claimable")]
    NoAstroStakingRewards,

    #[error("Invalid reward config")]
    InvalidRewardConfig,
}
