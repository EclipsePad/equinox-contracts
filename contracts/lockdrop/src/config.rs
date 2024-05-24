use equinox_msg::lockdrop::{LockConfig, RewardDistributionConfig};

pub const MAX_WITHDRAW_BPS: u64 = 5000u64;
pub const DEFAULT_DEPOSIT_WINDOW: u64 = 86400 * 5;
pub const DEFAULT_WITHDRAW_WINDOW: u64 = 86400 * 2;

pub const DEFAULT_LOCK_CONFIGS: &[LockConfig] = &[
    LockConfig {
        duration: 0,
        multiplier: 1,
        early_unlock_penalty_bps: 5000,
    },
    LockConfig {
        duration: 86400 * 30,
        multiplier: 2,
        early_unlock_penalty_bps: 5000,
    },
    LockConfig {
        duration: 86400 * 30 * 3,
        multiplier: 6,
        early_unlock_penalty_bps: 5000,
    },
    LockConfig {
        duration: 86400 * 30 * 6,
        multiplier: 12,
        early_unlock_penalty_bps: 5000,
    },
    LockConfig {
        duration: 86400 * 30 * 9,
        multiplier: 18,
        early_unlock_penalty_bps: 5000,
    },
    LockConfig {
        duration: 86400 * 365,
        multiplier: 24,
        early_unlock_penalty_bps: 5000,
    },
];

pub const DEFAULT_REWARD_DISTRIBUTION_CONFIG: RewardDistributionConfig = RewardDistributionConfig {
    instant: 10000,    // bps
    vesting_period: 0, // no vesting
};
