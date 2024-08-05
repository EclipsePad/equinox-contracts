use equinox_msg::lockdrop::{LockConfig, RewardDistributionConfig};

pub const DEFAULT_DEPOSIT_WINDOW: u64 = 86400 * 5;
pub const DEFAULT_WITHDRAW_WINDOW: u64 = 86400 * 2;
pub const MINIMUM_WINDOW: u64 = 86400;
pub const BPS_DENOMINATOR: u64 = 10000;

pub const DEFAULT_LOCK_CONFIGS: &[LockConfig] = &[
    LockConfig {
        duration: 0,
        multiplier: 5000,
        early_unlock_penalty_bps: 5000,
    },
    LockConfig {
        duration: 86400 * 30,
        multiplier: 10000,
        early_unlock_penalty_bps: 5000,
    },
    LockConfig {
        duration: 86400 * 30 * 3,
        multiplier: 15000,
        early_unlock_penalty_bps: 5000,
    },
    LockConfig {
        duration: 86400 * 30 * 6,
        multiplier: 20000,
        early_unlock_penalty_bps: 5000,
    },
    LockConfig {
        duration: 86400 * 30 * 9,
        multiplier: 22500,
        early_unlock_penalty_bps: 5000,
    },
    LockConfig {
        duration: 86400 * 365,
        multiplier: 25000,
        early_unlock_penalty_bps: 5000,
    },
];

pub const DEFAULT_REWARD_DISTRIBUTION_CONFIG: RewardDistributionConfig = RewardDistributionConfig {
    instant: BPS_DENOMINATOR, // bps
    vesting_period: 0,        // no vesting
};
