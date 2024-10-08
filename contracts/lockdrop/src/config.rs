use std::str;
use equinox_msg::lockdrop::{LockConfig, RewardDistributionConfig};

pub const DEFAULT_DEPOSIT_WINDOW: u64 = 86400 * 5;
pub const DEFAULT_WITHDRAW_WINDOW: u64 = 86400 * 2;
pub const MINIMUM_WINDOW: u64 = 86400;
pub const BPS_DENOMINATOR: u64 = 10000;
pub const DEFAULT_INIT_EARLY_UNLOCK_PENALTY: &str = "0.7";

pub const DEFAULT_LOCK_CONFIGS: &[LockConfig] = &[
    LockConfig {
        duration: 0,
        multiplier: 10000,
    },
    LockConfig {
        duration: 86400 * 30,
        multiplier: 12500,
    },
    LockConfig {
        duration: 86400 * 30 * 3,
        multiplier: 15000,
    },
    LockConfig {
        duration: 86400 * 30 * 6,
        multiplier: 20000,
    },
];

pub const DEFAULT_REWARD_DISTRIBUTION_CONFIG: RewardDistributionConfig = RewardDistributionConfig {
    instant: BPS_DENOMINATOR, // bps
    vesting_period: 0,        // no vesting
};
