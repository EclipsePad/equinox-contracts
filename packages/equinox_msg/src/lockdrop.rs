use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_json_binary, Addr, CosmosMsg, Decimal256, Env, StdResult, Uint128, Uint256, WasmMsg,
};
use cw20::Cw20ReceiveMsg;

use crate::single_sided_staking::UserReward;

#[cw_serde]
pub struct InstantiateMsg {
    /// Account which can update config
    pub owner: Option<String>,
    /// Timestamp when Contract will start accepting ASTRO/xASTRO tokens
    pub init_timestamp: u64,
    /// Number of seconds during which lockup deposits will be accepted
    pub deposit_window: Option<u64>,
    /// Withdrawal Window Length :: Post the deposit window
    pub withdrawal_window: Option<u64>,
    /// lockup config(duration, multiplier)
    pub lock_configs: Option<Vec<LockConfig>>,
    /// ASTRO token denom
    pub astro_token: String,
    /// xASTRO token denom
    pub xastro_token: String,
    /// bECLIP token address
    pub beclip: String,
    /// ECLIP denom
    pub eclip: String,
    pub eclip_staking: String,
    /// astro staking pool
    pub astro_staking: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    // ADMIN Function ::: To update configuration
    UpdateConfig {
        new_config: UpdateConfigMsg,
    },
    // ADMIN Function ::: To update owner
    UpdateOwner {
        new_owner: Addr,
    },
    UpdateRewardDistributionConfig {
        new_config: RewardDistributionConfig,
    },
    // Stake ASTRO/xASTRO tokens during deposit phase
    IncreaseLockup {
        stake_type: StakeType,
        duration: u64,
    },
    // Function to increase lockup duration while deposit window
    ExtendLock {
        stake_type: StakeType,
        from: u64,
        to: u64,
    },
    Unlock {
        stake_type: StakeType,
        duration: u64,
        amount: Option<Uint128>,
    },
    // Receive hook used to accept ASTRO/xASTRO Token deposits
    Receive(Cw20ReceiveMsg),
    // ADMIN Function ::: To deposit ASTRO/xASTRO to Eclipse Equinox vxASTRO holder contract
    StakeToVaults {},
    /// Callbacks; only callable by the contract itself.
    // Facilitates ECLIP reward withdrawal along with optional Unlock
    ClaimRewards {
        stake_type: StakeType,
        duration: u64,
        assets: Option<Vec<AssetInfo>>,
    },
    Callback(CallbackMsg),
    ClaimAllRewards {
        stake_type: StakeType,
        with_flexible: bool,
        assets: Option<Vec<AssetInfo>>,
    },
    IncreaseIncentives {
        rewards: Vec<IncentiveRewards>,
    },
}

#[cw_serde]
pub struct MigrateMsg {
    pub update_contract_name: Option<bool>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// query config
    #[returns(Config)]
    Config {},
    /// query reward config
    #[returns(RewardDistributionConfig)]
    RewardConfig {},
    /// query owner
    #[returns(Addr)]
    Owner {},
    /// query lockup info
    #[returns(SingleLockupInfoResponse)]
    SingleLockupInfo {},
    #[returns(LpLockupInfoResponse)]
    LpLockupInfo {},
    /// query lockup state
    #[returns(SingleLockupStateResponse)]
    SingleLockupState {},
    #[returns(LpLockupStateResponse)]
    LpLockupState {},
    /// query user lockup info
    #[returns(Vec<UserSingleLockupInfoResponse>)]
    UserSingleLockupInfo { user: String },
    #[returns(Vec<UserLpLockupInfoResponse>)]
    UserLpLockupInfo { user: String },
    #[returns(IncentiveAmounts)]
    Incentives { stake_type: StakeType },
}

#[cw_serde]
pub enum Cw20HookMsg {
    IncreaseIncentives { rewards: Vec<IncentiveRewards> },
}

#[cw_serde]
pub enum CallbackMsg {
    IncreaseLockup {
        prev_xastro_balance: Uint128,
        stake_type: StakeType,
        duration: u64,
        sender: String,
    },
    ExtendLockup {
        prev_xastro_balance: Uint128,
        stake_type: StakeType,
        from_duration: u64,
        to_duration: u64,
        sender: String,
    },
    ExtendLockupAfterLockdrop {
        prev_eclipastro_balance: Uint128,
        from_duration: u64,
        to_duration: u64,
        sender: String,
    },
    StakeToSingleVault {
        prev_eclipastro_balance: Uint128,
        xastro_amount_to_convert: Uint128,
        weighted_amount: Uint128,
    },
    DepositIntoPool {
        prev_eclipastro_balance: Uint128,
        total_xastro_amount: Uint128,
        xastro_amount_to_deposit: Uint128,
        weighted_amount: Uint128,
    },
    StakeLpToken {
        prev_lp_token_balance: Uint128,
    },
}

impl CallbackMsg {
    pub fn to_cosmos_msg(self, env: &Env) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::Callback(self))?,
            funds: vec![],
        }))
    }
}

#[cw_serde]
pub struct Config {
    /// ASTRO Token address
    pub astro_token: String,
    /// xASTRO Token address
    pub xastro_token: String,
    /// bECLIP
    pub beclip: AssetInfo,
    /// ECLIP
    pub eclip: AssetInfo,
    /// eclipASTRO Token
    pub eclipastro_token: Option<AssetInfo>,
    /// ASTRO/eclipASTRO converter contract address
    pub voter: Option<Addr>,
    /// ECLIP staking
    pub eclip_staking: Option<Addr>,
    /// eclipASTRO single sided staking pool address
    pub single_sided_staking: Option<Addr>,
    /// eclipASTRO/xASTRO lp staking pool address
    pub lp_staking: Option<Addr>,
    /// eclipASTRO/xASTRO pool
    pub liquidity_pool: Option<Addr>,
    /// eclipASTRO/xASTRO LP Token address
    pub lp_token: Option<AssetInfo>,
    /// astro staking pool
    pub astro_staking: Addr,
    /// Timestamp when Contract will start accepting ASTRO/xASTRO Token deposits
    pub init_timestamp: u64,
    /// Number of seconds during which lockup deposits will be accepted
    pub deposit_window: u64,
    /// Withdrawal Window Length :: Post the deposit window
    pub withdrawal_window: u64,
    /// lockup config
    pub lock_configs: Vec<LockConfig>,
    pub dao_treasury_address: Option<Addr>,
    pub claims_allowed: bool,
    pub countdown_start_at: u64,
}

#[cw_serde]
pub struct UpdateConfigMsg {
    pub single_sided_staking: Option<String>,
    pub lp_staking: Option<String>,
    pub liquidity_pool: Option<String>,
    pub eclipastro_token: Option<String>,
    pub voter: Option<String>,
    pub eclip_staking: Option<String>,
    pub dao_treasury_address: Option<String>,
}

#[cw_serde]
#[derive(Default)]
pub struct LockConfig {
    pub duration: u64,
    pub multiplier: u64,               // basis points
    pub early_unlock_penalty_bps: u64, // basis points
}

// change when user deposit/withdraw
#[cw_serde]
pub struct LockupInfo {
    /// total xastro amount received
    pub xastro_amount_in_lockups: Uint128,
    /// total staked balance to staking vault
    pub total_staked: Uint128,
    /// withdrawed balance from staking vault
    pub total_withdrawed: Uint128,
}

impl Default for LockupInfo {
    fn default() -> Self {
        LockupInfo {
            xastro_amount_in_lockups: Uint128::zero(),
            total_staked: Uint128::zero(),
            total_withdrawed: Uint128::zero(),
        }
    }
}

// change when user deposit/withdraw ASTRO/xASTRO
// if user withdraw assets during withdraw window, withdrawal_flag is set true, and can't withdraw more
// when user try to unlock, if eclipastro_locked is zero and is_calculated is false, calculate user's eclipastro_locked
#[cw_serde]
pub struct SingleUserLockupInfo {
    /// xASTRO locked by the user
    pub xastro_amount_in_lockups: Uint128,
    /// Boolean value indicating if the user's has withdrawn funds post the only 1 withdrawal limit cutoff
    pub withdrawal_flag: bool,
    pub lockdrop_incentives: LockdropIncentives,
    pub last_claimed: Option<u64>,
    pub total_eclipastro_staked: Uint128,
    pub total_eclipastro_withdrawed: Uint128,
    pub unclaimed_rewards: UnclaimedRewards,
}

#[cw_serde]
pub struct UnclaimedRewards {
    pub eclip: Uint128,
    pub beclip: Uint128,
    pub eclipastro: Uint128,
}

impl Default for UnclaimedRewards {
    fn default() -> Self {
        UnclaimedRewards {
            eclip: Uint128::zero(),
            beclip: Uint128::zero(),
            eclipastro: Uint128::zero(),
        }
    }
}

#[cw_serde]
#[derive(Default)]
pub struct IncentiveAmounts {
    pub beclip: Uint128,
    pub eclip: Uint128,
}

#[cw_serde]
pub struct LockdropIncentives {
    pub beclip: LockdropIncentive,
    pub eclip: LockdropIncentive,
}

#[cw_serde]
#[derive(Default)]
pub struct LockdropIncentive {
    pub allocated: Uint128,
    pub claimed: Uint128,
}

impl Default for SingleUserLockupInfo {
    fn default() -> Self {
        SingleUserLockupInfo {
            xastro_amount_in_lockups: Uint128::zero(),
            withdrawal_flag: false,
            lockdrop_incentives: LockdropIncentives {
                beclip: LockdropIncentive::default(),
                eclip: LockdropIncentive::default(),
            },
            last_claimed: None,
            total_eclipastro_staked: Uint128::zero(),
            total_eclipastro_withdrawed: Uint128::zero(),
            unclaimed_rewards: UnclaimedRewards::default(),
        }
    }
}

#[cw_serde]
pub struct LpUserLockupInfo {
    /// xASTRO locked by the user
    pub xastro_amount_in_lockups: Uint128,
    /// Boolean value indicating if the user's has withdrawn funds post the only 1 withdrawal limit cutoff
    pub withdrawal_flag: bool,
    pub lockdrop_incentives: LockdropIncentives,
    /// Asset rewards weights
    pub reward_weights: LpStakingRewardWeights,
    pub total_lp_staked: Uint128,
    pub total_lp_withdrawed: Uint128,
}

impl Default for LpUserLockupInfo {
    fn default() -> Self {
        LpUserLockupInfo {
            xastro_amount_in_lockups: Uint128::zero(),
            withdrawal_flag: false,
            lockdrop_incentives: LockdropIncentives {
                beclip: LockdropIncentive::default(),
                eclip: LockdropIncentive::default(),
            },
            reward_weights: LpStakingRewardWeights::default(),
            total_lp_staked: Uint128::zero(),
            total_lp_withdrawed: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct SingleLockupState {
    /// total locked eclipASTRO amount
    pub total_eclipastro_lockup: Uint128,
    /// total locked eclipASTRO amount * lockdrop reward multiplier for ECLIP incentives
    pub weighted_total_eclipastro_lockup: Uint256,
    /// total xASTRO at the end of the lockdrop
    pub total_xastro: Uint128,
    /// total xASTRO at the end of the lockdrop
    pub weighted_total_xastro: Uint128,
}

impl Default for SingleLockupState {
    fn default() -> Self {
        SingleLockupState {
            total_eclipastro_lockup: Uint128::zero(),
            weighted_total_eclipastro_lockup: Uint256::zero(),
            total_xastro: Uint128::zero(),
            weighted_total_xastro: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct LpLockupState {
    /// total locked lp token amount
    pub total_lp_lockdrop: Uint128,
    /// total locked lp amount * lockdrop reward multiplier for ECLIP incentives
    pub weighted_total_lp_lockdrop: Uint256,
    /// total xASTRO at the end of the lockdrop
    pub total_xastro: Uint128,
    /// total xASTRO at the end of the lockdrop
    pub weighted_total_xastro: Uint128,
}

impl Default for LpLockupState {
    fn default() -> Self {
        LpLockupState {
            total_lp_lockdrop: Uint128::zero(),
            weighted_total_lp_lockdrop: Uint256::zero(),
            total_xastro: Uint128::zero(),
            weighted_total_xastro: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub enum StakeType {
    SingleStaking,
    LpStaking,
}

#[cw_serde]
pub struct SingleStakingRewardWeights {
    pub eclipastro: Decimal256,
    pub eclip: Decimal256,
    pub beclip: Decimal256,
}

impl Default for SingleStakingRewardWeights {
    fn default() -> Self {
        SingleStakingRewardWeights {
            eclipastro: Decimal256::zero(),
            eclip: Decimal256::zero(),
            beclip: Decimal256::zero(),
        }
    }
}

#[cw_serde]
pub struct SingleStakingRewardsByDuration {
    pub duration: u64,
    pub rewards: UserReward,
}

#[cw_serde]
pub struct LpStakingRewardWeights {
    pub astro: Decimal256,
    pub eclip: Decimal256,
    pub beclip: Decimal256,
}

impl Default for LpStakingRewardWeights {
    fn default() -> Self {
        LpStakingRewardWeights {
            astro: Decimal256::zero(),
            eclip: Decimal256::zero(),
            beclip: Decimal256::zero(),
        }
    }
}

#[cw_serde]
pub struct LpStakingRewards {
    pub astro: Uint128,
    pub eclip: Uint128,
    pub beclip: Uint128,
}

impl Default for LpStakingRewards {
    fn default() -> Self {
        LpStakingRewards {
            astro: Uint128::zero(),
            eclip: Uint128::zero(),
            beclip: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct SingleLockupInfoResponse {
    pub single_lockups: Vec<DetailedSingleLockupInfo>,
    pub pending_rewards: Vec<SingleStakingRewardsByDuration>,
}

#[cw_serde]
pub struct LpLockupInfoResponse {
    pub lp_lockups: Vec<DetailedLpLockupInfo>,
    pub pending_rewards: LpStakingRewards,
    pub reward_weights: LpStakingRewardWeights,
}

#[cw_serde]
pub struct DetailedSingleLockupInfo {
    pub duration: u64,
    /// total xastro amount received
    pub xastro_amount_in_lockups: Uint128,
    /// total staked balance
    pub total_eclipastro_staked: Uint128,
    /// withdrawed balance
    pub total_eclipastro_withdrawed: Uint128,
    pub reward_multiplier: u64, //basis point
    pub reward_weights: SingleStakingRewardWeights,
}

#[cw_serde]
pub struct DetailedLpLockupInfo {
    pub duration: u64,
    /// total xastro amount received
    pub xastro_amount_in_lockups: Uint128,
    /// total staked balance
    pub total_lp_staked: Uint128,
    /// withdrawed balance
    pub total_lp_withdrawed: Uint128,
    pub reward_multiplier: u64, //basis point
}

#[cw_serde]
pub struct SingleLockupStateResponse {
    pub are_claims_allowed: bool,
    pub countdown_start_at: u64,
    pub total_eclipastro_lockup: Uint128,
}

#[cw_serde]
pub struct LpLockupStateResponse {
    pub are_claims_allowed: bool,
    pub countdown_start_at: u64,
    pub total_lp_lockdrop: Uint128,
}

#[cw_serde]
pub struct UserSingleLockupInfoResponse {
    pub duration: u64,
    pub xastro_amount_in_lockups: Uint128,
    pub eclipastro_staked: Uint128,
    pub eclipastro_withdrawed: Uint128,
    pub withdrawal_flag: bool,
    pub lockdrop_incentives: LockdropIncentives,
    pub staking_rewards: Vec<Asset>,
    pub countdown_start_at: u64,
}

#[cw_serde]
pub struct UserLpLockupInfoResponse {
    pub duration: u64,
    pub xastro_amount_in_lockups: Uint128,
    pub lp_token_staked: Uint128,
    pub lp_token_withdrawed: Uint128,
    pub withdrawal_flag: bool,
    pub lockdrop_incentives: LockdropIncentives,
    pub staking_rewards: Vec<Asset>,
    pub countdown_start_at: u64,
    pub reward_weights: LpStakingRewardWeights,
}

#[cw_serde]
pub struct RewardDistributionConfig {
    pub instant: u64,        // bps
    pub vesting_period: u64, // seconds
}

#[cw_serde]
pub struct IncentiveRewards {
    pub stake_type: StakeType,
    pub eclip: Uint128,
    pub beclip: Uint128,
}
