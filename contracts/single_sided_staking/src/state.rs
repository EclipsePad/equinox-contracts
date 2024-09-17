use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal256, StdResult, Storage, Uint128, Uint256};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map, SnapshotItem, SnapshotMap, Strategy};

use equinox_msg::single_sided_staking::{Config, OwnershipProposal, RewardConfig};

use crate::{config::ONE_DAY, error::ContractError};

/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "single sided staking contract";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const OWNER: Admin = Admin::new("owner");
pub const CONFIG: Item<Config> = Item::new("config");
// user staking info (address, duration, start_time)
pub const USER_STAKED: Map<(&String, u64, u64), UserStaked> = Map::new("user_staking");
pub const TOTAL_STAKING: Item<Uint128> = Item::new("total_staking");
pub const TOTAL_STAKING_BY_DURATION: SnapshotMap<u64, TotalStakingByDuration> = SnapshotMap::new(
    "total_staking_by_duration",
    "total_staking_by_duration__checkpoints",
    "total_staking_by_duration__changelog",
    Strategy::EveryBlock,
);
// duration, block_time, amount
pub const STAKING_DURATION_BY_END_TIME: Map<(u64, u64), Uint128> =
    Map::new("staking_duration_by_end_time");
// only allowed users can set amount when withdraw and relock
pub const ALLOWED_USERS: Map<&String, bool> = Map::new("allowed_users");

pub const LAST_CLAIM_TIME: Item<u64> = Item::new("last_claim_time");
pub const REWARD_WEIGHTS: SnapshotItem<RewardWeights> = SnapshotItem::new(
    "reward_weights",
    "reward_weights__checkpoints",
    "reward_weights__changelog",
    Strategy::EveryBlock,
);
pub const PENDING_ECLIPASTRO_REWARDS: Map<u64, Uint128> = Map::new("pending_eclipastro_rewards");
pub const REWARD_CONFIG: Item<RewardConfig> = Item::new("reward_config");
/// Stores the latest contract ownership transfer proposal
pub const OWNERSHIP_PROPOSAL: Item<OwnershipProposal> = Item::new("ownership_proposal");

#[cw_serde]
pub struct TotalStakingByDuration {
    pub staked: Uint128,
    pub valid_staked: Uint128,
}

impl Default for TotalStakingByDuration {
    fn default() -> Self {
        TotalStakingByDuration {
            staked: Uint128::zero(),
            valid_staked: Uint128::zero(),
        }
    }
}

impl TotalStakingByDuration {
    pub fn load_at_ts(
        storage: &dyn Storage,
        duration: u64,
        block_time: u64,
        timestamp: Option<u64>,
    ) -> StdResult<Self> {
        let staking = match timestamp.unwrap_or(block_time) {
            timestamp if timestamp == block_time => {
                TOTAL_STAKING_BY_DURATION.may_load(storage, duration)
            }
            timestamp => TOTAL_STAKING_BY_DURATION.may_load_at_height(storage, duration, timestamp),
        }?
        .unwrap_or_default();

        Ok(staking)
    }

    pub fn load(storage: &dyn Storage, block_time: u64, duration: u64) -> StdResult<Self> {
        Self::load_at_ts(storage, block_time, duration, None)
    }
    /// calculate boosted total staking at certain time
    pub fn load_boost_sum_at_ts(
        storage: &dyn Storage,
        block_time: u64,
        timestamp: Option<u64>,
    ) -> StdResult<Uint256> {
        let cfg = CONFIG.load(storage)?;
        Ok(cfg
            .timelock_config
            .into_iter()
            .fold(Uint256::zero(), |acc, cur| {
                let boosted_stake = Uint256::from_uint128(
                    TotalStakingByDuration::load_at_ts(
                        storage,
                        block_time,
                        cur.duration,
                        timestamp,
                    )
                    .unwrap_or_default()
                    .valid_staked,
                ) * Uint256::from_u128(cur.reward_multiplier.into());
                acc + boosted_stake
            }))
    }

    pub fn add(
        storage: &mut dyn Storage,
        amount: Uint128,
        duration: u64,
        block_time: u64,
    ) -> Result<(), ContractError> {
        let mut staking = TOTAL_STAKING_BY_DURATION
            .load(storage, duration)
            .unwrap_or_default();
        staking.staked += amount;
        staking.valid_staked += amount;
        TOTAL_STAKING_BY_DURATION.save(storage, duration, &staking, block_time)?;
        let end_time = block_time / ONE_DAY * ONE_DAY + ONE_DAY;
        STAKING_DURATION_BY_END_TIME
            .update(storage, (duration, end_time), |s| {
                Ok(s.unwrap_or_default() + amount)
            })
            .map(|_| ())
    }

    pub fn sub(
        storage: &mut dyn Storage,
        amount: Uint128,
        duration: u64,
        locked_at: u64,
        block_time: u64,
    ) -> Result<(), ContractError> {
        let mut staking = TOTAL_STAKING_BY_DURATION
            .load(storage, duration)
            .unwrap_or_default();
        staking.staked -= amount;
        let end_time = (locked_at + duration) / ONE_DAY * ONE_DAY + ONE_DAY;
        if end_time > block_time {
            staking.valid_staked -= amount;
        }
        TOTAL_STAKING_BY_DURATION.save(storage, duration, &staking, block_time)?;
        STAKING_DURATION_BY_END_TIME
            .update(storage, (duration, end_time), |s| {
                Ok(s.unwrap_or_default() - amount)
            })
            .map(|_| ())
    }
}

#[cw_serde]
pub struct RewardWeights {
    pub eclipastro: Decimal256,
    pub beclip: Decimal256,
    pub eclip: Decimal256,
}

impl Default for RewardWeights {
    fn default() -> Self {
        RewardWeights {
            eclip: Decimal256::zero(),
            eclipastro: Decimal256::zero(),
            beclip: Decimal256::zero(),
        }
    }
}

impl RewardWeights {
    pub fn load_at_ts(
        storage: &dyn Storage,
        block_time: u64,
        timestamp: Option<u64>,
    ) -> StdResult<Self> {
        let staking = match timestamp.unwrap_or(block_time) {
            timestamp if timestamp == block_time => REWARD_WEIGHTS.may_load(storage),
            timestamp => REWARD_WEIGHTS.may_load_at_height(storage, timestamp),
        }?
        .unwrap_or_default();

        Ok(staking)
    }

    pub fn load(storage: &dyn Storage, block_time: u64) -> StdResult<Self> {
        Self::load_at_ts(storage, block_time, None)
    }
}

#[cw_serde]
pub struct UserStaked {
    pub staked: Uint128,
    pub reward_weights: RewardWeights,
}

impl Default for UserStaked {
    fn default() -> Self {
        UserStaked {
            staked: Uint128::zero(),
            reward_weights: RewardWeights::default(),
        }
    }
}
