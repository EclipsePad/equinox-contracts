use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_json_binary, Addr, CosmosMsg, Decimal, Decimal256, Env, StdResult, Uint128, Uint256, WasmMsg,
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
    pub eclipastro_token: String,
    /// Eclip address
    pub eclip: String,
    /// astro staking pool
    pub astro_staking: String,
    /// Equinox ASTRO/eclipASTRO converter contract
    pub converter: String,
    /// eclipASTRO/xASTRO pool
    pub liquidity_pool: String,
    pub dao_treasury_address: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    // Receive hook used to accept ASTRO/xASTRO Token deposits
    Receive(Cw20ReceiveMsg),
    // ADMIN Function ::: To update configuration
    UpdateConfig {
        new_config: UpdateConfigMsg,
    },
    UpdateRewardDistributionConfig {
        new_config: RewardDistributionConfig,
    },
    // ADMIN Function ::: To deposit ASTRO/xASTRO to Eclipse Equinox vxASTRO holder contract
    StakeToVaults {},
    // Function to increase lockup duration while deposit window
    ExtendLock {
        stake_type: StakeType,
        from: u64,
        to: u64,
    },
    // Relock after lockdrop ends
    // Function to facilitate ASTRO/xASTRO Token withdrawals from lockups
    SingleLockupWithdraw {
        amount: Option<Uint128>,
        duration: u64,
    },
    // Function to facilitate ASTRO/xASTRO Token withdrawals from lockups
    LpLockupWithdraw {
        amount: Option<Uint128>,
        duration: u64,
    },
    IncreaseEclipIncentives {},
    RelockSingleStaking {
        from: u64,
        to: u64,
    },
    // Facilitates ECLIP reward withdrawal along with optional Unlock
    ClaimRewardsAndOptionallyUnlock {
        stake_type: StakeType,
        duration: u64,
        amount: Option<Uint128>,
    },
    /// Callbacks; only callable by the contract itself.
    Callback(CallbackMsg),
    /// ProposeNewOwner creates a proposal to change contract ownership.
    /// The validity period for the proposal is set in the `expires_in` variable.
    ProposeNewOwner {
        /// Newly proposed contract owner
        owner: String,
        /// The date after which this proposal expires
        expires_in: u64,
    },
    /// DropOwnershipProposal removes the existing offer to change contract ownership.
    DropOwnershipProposal {},
    /// Used to claim contract ownership.
    ClaimOwnership {},
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
    /// query owner
    #[returns(Addr)]
    Owner {},
    /// query lockup info
    #[returns(Vec<LockupInfoResponse>)]
    SingleLockupInfo {},
    #[returns(Vec<LockupInfoResponse>)]
    LpLockupInfo {},
    /// query lockup state
    #[returns(SingleLockupStateResponse)]
    SingleLockupState {},
    #[returns(LpLockupStateResponse)]
    LpLockupState {},
    /// query user lockup info
    #[returns(Vec<UserSingleLockupInfoResponse>)]
    UserSingleLockupInfo { user: Addr },
    #[returns(Vec<UserLpLockupInfoResponse>)]
    UserLpLockupInfo { user: Addr },
    #[returns(BalanceResponse)]
    TotalEclipIncentives {},
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Open a new user position or add to an existing position (Cw20ReceiveMsg)
    IncreaseLockup {
        stake_type: StakeType,
        duration: u64,
    },
    ExtendDuration {
        stake_type: StakeType,
        from: u64,
        to: u64,
    },
    Relock {
        from: u64,
        to: u64,
    },
}

#[cw_serde]
pub enum CallbackMsg {
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
    DistributeLpStakingAssetRewards {
        prev_eclip_balance: Uint128,
        prev_astro_balance: Uint128,
        user_address: Addr,
        recipient: Addr,
        duration: u64,
    },
    DistributeSingleStakingAssetRewards {
        prev_eclip_balance: Uint128,
        prev_eclipastro_balance: Uint128,
        user_address: Addr,
        recipient: Addr,
        duration: u64,
    },
    StakeLpToken {
        prev_lp_token_balance: Uint128,
    },
    ClaimSingleStakingAssetRewards {
        user_address: Addr,
        recipient: Addr,
        duration: u64,
    },
    ClaimLpStakingAssetRewards {
        user_address: Addr,
        recipient: Addr,
        duration: u64,
    },
    ClaimSingleStakingRewards {
        user_address: Addr,
        duration: u64,
    },
    UnlockSingleLockup {
        user_address: Addr,
        duration: u64,
        amount: Uint128,
    },
    UnlockLpLockup {
        user_address: Addr,
        duration: u64,
        amount: Uint128,
    },
    StakeSingleVault {},
    StakeLpVault {},
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
    pub astro_token: Addr,
    /// xASTRO Token address
    pub xastro_token: Addr,
    /// ECLIP address
    pub eclip: String,
    /// eclipASTRO Token address
    pub eclipastro_token: Addr,
    /// ASTRO/eclipASTRO converter contract address
    pub converter: Addr,
    /// eclipASTRO flexible staking pool address
    pub flexible_staking: Option<Addr>,
    /// eclipASTRO timelock staking pool address
    pub timelock_staking: Option<Addr>,
    /// eclipASTRO/xASTRO lp staking pool address
    pub lp_staking: Option<Addr>,
    /// single staking vault reward distributor address
    pub reward_distributor: Option<Addr>,
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
}

#[cw_serde]
pub struct UpdateConfigMsg {
    pub flexible_staking: Option<String>,
    pub timelock_staking: Option<String>,
    pub lp_staking: Option<String>,
    pub reward_distributor: Option<String>,
    pub dao_treasury_address: Option<String>,
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
    pub total_eclip_incentives: Uint128,
    /// ECLIP incentives for participation in the lockdrop, zero before lockdrop ends
    pub claimed_eclip_incentives: Uint128,
    /// Asset rewards weights
    pub reward_weights: Vec<AssetRewardWeight>,
    pub total_eclipastro_staked: Uint128,
    pub total_eclipastro_withdrawed: Uint128,
}

impl Default for SingleUserLockupInfo {
    fn default() -> Self {
        SingleUserLockupInfo {
            xastro_amount_in_lockups: Uint128::zero(),
            withdrawal_flag: false,
            total_eclip_incentives: Uint128::zero(),
            claimed_eclip_incentives: Uint128::zero(),
            reward_weights: vec![],
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
    pub total_eclip_incentives: Uint128,
    /// ECLIP incentives for participation in the lockdrop, zero before lockdrop ends
    pub claimed_eclip_incentives: Uint128,
    /// Asset rewards weights
    pub reward_weights: Vec<AssetRewardWeight>,
    pub total_lp_staked: Uint128,
    pub total_lp_withdrawed: Uint128,
}

impl Default for LpUserLockupInfo {
    fn default() -> Self {
        LpUserLockupInfo {
            xastro_amount_in_lockups: Uint128::zero(),
            withdrawal_flag: false,
            total_eclip_incentives: Uint128::zero(),
            claimed_eclip_incentives: Uint128::zero(),
            reward_weights: vec![],
            total_lp_staked: Uint128::zero(),
            total_lp_withdrawed: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct SingleLockupState {
    /// Boolean value indicating if the user can withdraw their ECLIP rewards or not
    pub are_claims_allowed: bool,
    /// start time to countdown lock
    pub countdown_start_at: u64,
    /// Boolean value indicating if the asset is already staked
    pub is_staked: bool,
    /// total locked eclipASTRO amount
    pub total_eclipastro_lockup: Uint128,
    /// total locked eclipASTRO amount * lockdrop reward multiplier for ECLIP incentives
    pub weighted_total_eclipastro_lockup: Uint256,
    /// total xASTRO at the end of the lockdrop
    pub total_xastro: Uint128,
    /// total xASTRO at the end of the lockdrop
    pub weighted_total_xastro: Uint128,
    /// Asset rewards weights
    pub reward_weights: Vec<AssetRewardWeight>,
}

impl Default for SingleLockupState {
    fn default() -> Self {
        SingleLockupState {
            are_claims_allowed: false,
            countdown_start_at: 0u64,
            is_staked: false,
            total_eclipastro_lockup: Uint128::zero(),
            weighted_total_eclipastro_lockup: Uint256::zero(),
            total_xastro: Uint128::zero(),
            weighted_total_xastro: Uint128::zero(),
            reward_weights: vec![],
        }
    }
}

#[cw_serde]
pub struct AssetRewardWeight {
    pub asset: AssetInfo,
    pub weight: Decimal,
}

#[cw_serde]
pub struct LpLockupState {
    /// Boolean value indicating if the user can withdraw their ECLIP rewards or not
    pub are_claims_allowed: bool,
    /// start time to countdown lock
    pub countdown_start_at: u64,
    /// Boolean value indicating if the asset is already staked
    pub is_staked: bool,
    /// total locked lp token amount
    pub total_lp_lockdrop: Uint128,
    /// total locked lp amount * lockdrop reward multiplier for ECLIP incentives
    pub weighted_total_lp_lockdrop: Uint256,
    /// total xASTRO at the end of the lockdrop
    pub total_xastro: Uint128,
    /// total xASTRO at the end of the lockdrop
    pub weighted_total_xastro: Uint128,
    /// Asset rewards weights
    pub reward_weights: Vec<AssetRewardWeight>,
}

impl Default for LpLockupState {
    fn default() -> Self {
        LpLockupState {
            are_claims_allowed: false,
            countdown_start_at: 0u64,
            is_staked: false,
            total_lp_lockdrop: Uint128::zero(),
            weighted_total_lp_lockdrop: Uint256::zero(),
            total_xastro: Uint128::zero(),
            weighted_total_xastro: Uint128::zero(),
            reward_weights: vec![],
        }
    }
}

#[cw_serde]
pub enum StakeType {
    SingleStaking,
    LpStaking,
}

#[cw_serde]
pub struct SingleStakingAssetRewardWeights {
    eclipastro: Decimal256,
    eclip: Decimal256,
}

impl Default for SingleStakingAssetRewardWeights {
    fn default() -> Self {
        SingleStakingAssetRewardWeights {
            eclipastro: Decimal256::zero(),
            eclip: Decimal256::zero(),
        }
    }
}

#[cw_serde]
pub struct LpStakingAssetRewardWeights {
    astro: Decimal256,
    eclip: Decimal256,
}

impl Default for LpStakingAssetRewardWeights {
    fn default() -> Self {
        LpStakingAssetRewardWeights {
            astro: Decimal256::zero(),
            eclip: Decimal256::zero(),
        }
    }
}

#[cw_serde]
pub struct LockupInfoResponse {
    pub duration: u64,
    /// total xastro amount received
    pub xastro_amount_in_lockups: Uint128,
    /// total staked balance
    pub total_staked: Uint128,
    /// withdrawed balance
    pub total_withdrawed: Uint128,
}

#[cw_serde]
pub struct SingleLockupStateResponse {
    pub are_claims_allowed: bool,
    pub countdown_start_at: u64,
    pub is_staked: bool,
    pub total_eclipastro_lockup: Uint128,
}

#[cw_serde]
pub struct LpLockupStateResponse {
    pub are_claims_allowed: bool,
    pub countdown_start_at: u64,
    pub is_staked: bool,
    pub total_lp_lockdrop: Uint128,
}

#[cw_serde]
pub struct UserSingleLockupInfoResponse {
    pub duration: u64,
    pub xastro_amount_in_lockups: Uint128,
    pub eclipastro_staked: Uint128,
    pub eclipastro_withdrawed: Uint128,
    pub withdrawal_flag: bool,
    pub total_eclip_incentives: Uint128,
    pub claimed_eclip_incentives: Uint128,
    pub pending_eclip_incentives: Uint128,
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
    pub total_eclip_incentives: Uint128,
    pub claimed_eclip_incentives: Uint128,
    pub pending_eclip_incentives: Uint128,
    pub staking_rewards: Vec<Asset>,
    pub countdown_start_at: u64,
}

#[cw_serde]
pub struct RewardDistributionConfig {
    pub instant: u64,        // bps
    pub vesting_period: u64, // seconds
}
