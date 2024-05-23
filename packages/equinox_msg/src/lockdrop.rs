use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_json_binary, Addr, CosmosMsg, Decimal256, Env, StdResult, Uint128, Uint256, WasmMsg,
};
use cw20::{BalanceResponse, Cw20ReceiveMsg};

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
    /// ASTRO token address
    pub astro_token: String,
    /// xASTRO token address
    pub xastro_token: String,
    /// eclipASTRO token address
    pub eclipastro_token: Addr,
    /// bECLIP address
    pub beclip: AssetInfo,
    /// astro staking pool
    pub astro_staking: Addr,
    /// Equinox ASTRO/eclipASTRO converter contract
    pub converter: Addr,
    /// eclipASTRO single sided staking pool address
    pub single_sided_staking: Addr,
    /// eclipASTRO/xASTRO lp staking pool address
    pub lp_staking: Addr,
    /// eclipASTRO/xASTRO pool
    pub liquidity_pool: Addr,
    pub dao_treasury_address: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    // ADMIN Function ::: To update configuration
    UpdateConfig {
        new_config: UpdateConfigMsg,
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
    #[returns(BalanceResponse)]
    TotalbEclipIncentives {},
}

#[cw_serde]
pub enum Cw20HookMsg {
    ExtendLockup {
        stake_type: StakeType,
        from: u64,
        to: u64,
    },
    IncreasebEclipIncentives {},
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
        xastro_amount: Uint128,
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
    /// bECLIP address
    pub beclip: AssetInfo,
    /// eclipASTRO Token address
    pub eclipastro_token: Addr,
    /// ASTRO/eclipASTRO converter contract address
    pub converter: Addr,
    /// eclipASTRO single sided staking pool address
    pub single_sided_staking: Addr,
    /// eclipASTRO/xASTRO lp staking pool address
    pub lp_staking: Addr,
    /// eclipASTRO/xASTRO pool
    pub liquidity_pool: Addr,
    /// eclipASTRO/xASTRO LP Token address
    pub lp_token: Addr,
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
    /// Total ECLIP lockdrop incentives to be distributed among the users
    pub lockdrop_incentives: Uint128,
    pub dao_treasury_address: Addr,
    pub claims_allowed: bool,
    pub countdown_start_at: u64,
}

#[cw_serde]
pub struct UpdateConfigMsg {
    pub dao_treasury_address: Option<Addr>,
}

#[cw_serde]
#[derive(Default)]
pub struct LockConfig {
    pub duration: u64,
    pub multiplier: u64, // basis points
    pub early_unlock_penalty_bps: u64,
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
    /// ECLIP incentives for participation in the lockdrop, zero before lockdrop ends
    pub total_beclip_incentives: Uint128,
    /// ECLIP incentives for participation in the lockdrop, zero before lockdrop ends
    pub claimed_beclip_incentives: Uint128,
    /// Asset rewards weights
    pub reward_weights: SingleStakingRewardWeights,
    pub total_eclipastro_staked: Uint128,
    pub total_eclipastro_withdrawed: Uint128,
}

impl Default for SingleUserLockupInfo {
    fn default() -> Self {
        SingleUserLockupInfo {
            xastro_amount_in_lockups: Uint128::zero(),
            withdrawal_flag: false,
            total_beclip_incentives: Uint128::zero(),
            claimed_beclip_incentives: Uint128::zero(),
            reward_weights: SingleStakingRewardWeights::default(),
            total_eclipastro_staked: Uint128::zero(),
            total_eclipastro_withdrawed: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct LpUserLockupInfo {
    /// xASTRO locked by the user
    pub xastro_amount_in_lockups: Uint128,
    /// Boolean value indicating if the user's has withdrawn funds post the only 1 withdrawal limit cutoff
    pub withdrawal_flag: bool,
    /// ECLIP incentives for participation in the lockdrop, zero before lockdrop ends
    pub total_beclip_incentives: Uint128,
    /// ECLIP incentives for participation in the lockdrop, zero before lockdrop ends
    pub claimed_beclip_incentives: Uint128,
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
            total_beclip_incentives: Uint128::zero(),
            claimed_beclip_incentives: Uint128::zero(),
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
    pub beclip: Decimal256,
}

impl Default for SingleStakingRewardWeights {
    fn default() -> Self {
        SingleStakingRewardWeights {
            eclipastro: Decimal256::zero(),
            beclip: Decimal256::zero(),
        }
    }
}

#[cw_serde]
pub struct SingleStakingRewards {
    pub eclipastro: Uint128,
    pub beclip: Uint128,
}

impl Default for SingleStakingRewards {
    fn default() -> Self {
        SingleStakingRewards {
            eclipastro: Uint128::zero(),
            beclip: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct SingleStakingRewardsByDuration {
    pub duration: u64,
    pub rewards: SingleStakingRewards,
}

#[cw_serde]
pub struct LpStakingRewardWeights {
    pub astro: Decimal256,
    pub beclip: Decimal256,
}

impl Default for LpStakingRewardWeights {
    fn default() -> Self {
        LpStakingRewardWeights {
            astro: Decimal256::zero(),
            beclip: Decimal256::zero(),
        }
    }
}

#[cw_serde]
pub struct LpStakingRewards {
    pub astro: Uint128,
    pub beclip: Uint128,
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
    pub reward_multiplier: u64,
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
    pub reward_multiplier: u64,
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
    pub total_beclip_incentives: Uint128,
    pub claimed_beclip_incentives: Uint128,
    pub pending_beclip_incentives: Uint128,
    pub staking_rewards: Vec<Asset>,
    pub countdown_start_at: u64,
    pub reward_weights: SingleStakingRewardWeights,
}

#[cw_serde]
pub struct UserLpLockupInfoResponse {
    pub duration: u64,
    pub xastro_amount_in_lockups: Uint128,
    pub lp_token_staked: Uint128,
    pub lp_token_withdrawed: Uint128,
    pub withdrawal_flag: bool,
    pub total_beclip_incentives: Uint128,
    pub claimed_beclip_incentives: Uint128,
    pub pending_beclip_incentives: Uint128,
    pub staking_rewards: Vec<Asset>,
    pub countdown_start_at: u64,
    pub reward_weights: LpStakingRewardWeights,
}

#[cw_serde]
pub struct RewardDistributionConfig {
    pub instant: u64,        // bps
    pub vesting_period: u64, // seconds
}
