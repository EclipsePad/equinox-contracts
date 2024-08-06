use cosmwasm_std::StdError;
use thiserror::Error;

impl From<StdError> for ContractError {
    fn from(std_error: StdError) -> Self {
        Self::CustomError {
            val: std_error.to_string(),
        }
    }
}

impl From<ContractError> for StdError {
    fn from(contract_error: ContractError) -> Self {
        Self::generic_err(contract_error.to_string())
    }
}

pub fn parse_err(err: anyhow::Error) -> StdError {
    let context = format!("{}", err);
    let source = err.source().map(|x| x.to_string()).unwrap_or_default();

    StdError::GenericErr {
        msg: format!("{}\n{}", context, source),
    }
}

/// Never is a placeholder to ensure we don't return any errors
#[derive(Error, Debug)]
pub enum Never {}

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },

    // ------------------------------ common ----------------------------------------
    #[error("Parsing previous version error!")]
    ParsingPrevVersion,

    #[error("Parsing new version error!")]
    ParsingNewVersion,

    #[error("Msg version is not equal contract new version!")]
    ImproperMsgVersion,

    #[error("Msg is disabled!")]
    MessageIsDisabled,

    #[error("Sender does not have access permissions!")]
    Unauthorized,

    #[error("Parameters are not provided!")]
    NoParameters,

    #[error("Wrong message type!")]
    WrongMessageType,

    #[error("Wrong action type!")]
    WrongActionType,

    #[error("Undefined Reply ID!")]
    UndefinedReplyId,

    #[error("It's too late to accept admin role!")]
    TransferAdminDeadline,

    #[error("{value:?} config is not found!")]
    ParameterIsNotFound { value: String },

    #[error("Asset is not found!")]
    AssetIsNotFound,

    #[error("Wrong asset type!")]
    WrongAssetType,

    #[error("Wrong funds combination!")]
    WrongFundsCombination,

    // ------------------------------ faucet ----------------------------------------
    #[error("Come back later!")]
    ClaimCooldown,

    // ------------------------------ lottery ----------------------------------------
    #[error("Job ID is too long!")]
    JobIdTooLong,

    #[error("Job is in progress!")]
    JobIsInProgress,

    #[error("Received invalid randomness!")]
    InvalidRandomness,

    // ------------------------------ presale ----------------------------------------
    #[error("Vesting contract address can't be changed after sales start!")]
    ImmutableVesting,

    #[error("Client fee rate is out of range")]
    ClientFeeRateIsOutOfRange,

    #[error("Invalid date parameters")]
    InvalidDateParameters,

    #[error("Invalid private start time")]
    InvalidPrivateStartTime,

    #[error("Invalid public start time")]
    InvalidPublicStartTime,

    #[error("Invalid allocation parameters")]
    InvalidAllocationParameters,

    #[error("Public Not In Progress")]
    PublicNotInProgress,

    #[error("Private Not In Progress")]
    PrivateNotInProgress,

    #[error("Presale In Progress")]
    PresaleInProgress,

    #[error("Not Whitelisted")]
    NotWhitelisted,

    #[error("Allocation is not specified")]
    AllocationIsNotSpecified,

    #[error("Funds can't be divided by lots!")]
    FundLots,

    #[error("Sold out!")]
    SoldOut,

    #[error("Exceed Allocation")]
    ExceedAllocation,

    #[error("Exceed Total Allocation")]
    ExceedTotalAllocation,

    #[error("CW20 is not supported!")]
    Cw20IsNotSupported,

    // ------------------------------ staking ----------------------------------------
    #[error("Bonded vault can't be withdrawn!")]
    BondedVault,

    #[error("The contract is under maintenance!")]
    ContractIsPaused,

    #[error("Multiple vaults with same creation date are not allowed!")]
    MultipleVaultsWithSameCreationDate,

    #[error("Improper amount sum!")]
    ImproperAmountSum,

    #[error("Amount can not be equal zero!")]
    ZeroAmount,

    #[error("bECLIP amount is greater than total bonded amount!")]
    ExceedingBondedAmount,

    #[error("Locked amount is greater than total staked amount!")]
    ExceedingLockingAmount,

    #[error("Lock tier can not be decreased!")]
    DecreasingLockTier,

    #[error("Lock tier is out of range!")]
    LockTierIsOutOfRange,

    #[error("Tiers amount can't be changed!")]
    ImmutableTiersAmount,

    #[error("It's required to stake before locking or unlocking!")]
    StakerIsNotFound,

    #[error("It's required to lock before unlocking!")]
    LockerIsNotFound,

    #[error("Vault is not found!")]
    VaultIsNotFound,

    #[error("Vaults limit is reached! Use vault aggregation to reduce amount of vaults")]
    TooMuchVaults,

    #[error("Data should be given!")]
    DataIsNotProvided,

    #[error("Wrong token sent!")]
    WrongToken,

    #[error("Cannot unbond more than bond amount!")]
    ExceedingUnbondAmount,

    #[error("Invalid lock index!")]
    InvalidLockIndex,

    #[error("Tier is not found!")]
    TierIsNotFound,

    #[error("Cannot update! The new schedule must support all of the previous schedule!")]
    ScheduleDoesntSupportPrev,

    #[error("New schedule removes already started distribution!")]
    ScheduleRemovesDistribution,

    #[error("New schedule adds an already started distribution!")]
    ScheduleAddsDistribution,

    #[error("Coins amount is zero!")]
    ZeroCoinsAmount,

    #[error("Amount of denoms is not equal 1!")]
    MultipleDenoms,

    // ------------------------------ vesting ----------------------------------------
    #[error("initial unlock must be <= 1")]
    InvaildInitialUnlock,

    #[error("less than already distributed amount")]
    InvalidDistributionAmount,

    #[error("vesting already started")]
    VestingAlreadyStarted,

    #[error("can't set earlier time")]
    InvalidStartTime,

    #[error("exceed total distribution amount")]
    ExceedTotalDistributionAmount,

    #[error("can't withdraw unsold tokens before vesting start")]
    WithdrawUnSoldTokenBeforeVestingStart,

    #[error("invalid release interval")]
    InvalidReleaseInterval,

    #[error("invalid release rate")]
    InvalidReleaseRate,

    #[error("withdrawable amount is zero")]
    ZeroWithdrawableAmount,

    // ------------------------------ whitelist ----------------------------------------
    #[error("Exceeded wallet amount limit!")]
    WalletLimit,

    #[error("Default address can't be changed!")]
    ImmutableDefaultAddress,

    #[error("Default address is not found!")]
    DefaultAddressIsNotFound,

    #[error("User is not found!")]
    UserIsNotFound,

    #[error("User is added!")]
    UserIsAdded,

    #[error("Network duplication!")]
    NetworkDuplication,

    // ------------------------------ minter ----------------------------------------
    #[error("Exceeded tokens per owner limit!")]
    TokenLimit,

    // ------------------------------  ----------------------------------------
    #[error("Denom already exists!")]
    DenomExists,

    #[error("Address already exists!")]
    AddressExists,

    #[error("Multiple positions with same creation date are not allowed!")]
    SameCreationDate,
}
