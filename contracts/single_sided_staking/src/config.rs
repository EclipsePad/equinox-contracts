use equinox_msg::single_sided_staking::TimeLockConfig;

pub const REWARD_DISTRIBUTION_PERIOD: u64 = 8 * 86_400; // 8 days
pub const REWARD_DISTRIBUTION_TIME_DIFF: u64 = 6 * 3_600; // 6 hours
pub const DEFAULT_BECLIP_DAILY_REWARD: u128 = 1_000_000_000u128;

pub const DEFAULT_TIMELOCK_CONFIG: &[TimeLockConfig] = &[
    TimeLockConfig {
        duration: 0,
        early_unlock_penalty_bps: 0,
        reward_multiplier: 1,
    },
    TimeLockConfig {
        duration: 86400 * 30,
        early_unlock_penalty_bps: 5000,
        reward_multiplier: 2,
    },
    TimeLockConfig {
        duration: 86400 * 30 * 3,
        early_unlock_penalty_bps: 5000,
        reward_multiplier: 6,
    },
    TimeLockConfig {
        duration: 86400 * 30 * 6,
        early_unlock_penalty_bps: 5000,
        reward_multiplier: 12,
    },
    TimeLockConfig {
        duration: 86400 * 30 * 9,
        early_unlock_penalty_bps: 5000,
        reward_multiplier: 18,
    },
    TimeLockConfig {
        duration: 86400 * 365,
        early_unlock_penalty_bps: 5000,
        reward_multiplier: 24,
    },
];