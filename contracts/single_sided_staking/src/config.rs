use equinox_msg::single_sided_staking::TimeLockConfig;

pub const ONE_DAY: u64 = 86400u64;
pub const REWARD_DISTRIBUTION_PERIOD: u64 = 8 * ONE_DAY; // 8 days
pub const REWARD_DISTRIBUTION_TIME_DIFF: u64 = 6 * 3_600; // 6 hours
pub const BPS_DENOMINATOR: u64 = 10000;

pub const DEFAULT_TIMELOCK_CONFIG: &[TimeLockConfig] = &[
    TimeLockConfig {
        duration: 0,
        reward_multiplier: 5000,
        early_unlock_penalty_bps: 5000,
    },
    TimeLockConfig {
        duration: ONE_DAY * 30,
        reward_multiplier: 10000,
        early_unlock_penalty_bps: 5000,
    },
    TimeLockConfig {
        duration: ONE_DAY * 30 * 3,
        reward_multiplier: 15000,
        early_unlock_penalty_bps: 5000,
    },
    TimeLockConfig {
        duration: ONE_DAY * 30 * 6,
        reward_multiplier: 20000,
        early_unlock_penalty_bps: 5000,
    },
    TimeLockConfig {
        duration: ONE_DAY * 30 * 9,
        reward_multiplier: 22500,
        early_unlock_penalty_bps: 5000,
    },
    TimeLockConfig {
        duration: 86400 * 365,
        reward_multiplier: 25000,
        early_unlock_penalty_bps: 5000,
    },
];
