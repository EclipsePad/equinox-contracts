use equinox_msg::single_sided_staking::TimeLockConfig;

pub const ONE_DAY: u64 = 86400u64;
pub const ECLIPASTRO_REWARD_DISTRIBUTION_PERIOD: u64 = 8 * ONE_DAY; // 8 days
pub const DEFAULT_REWARD_DISTRIBUTION_PERIOD: u64 = 30 * ONE_DAY;
pub const REWARD_DISTRIBUTION_TIME_DIFF: u64 = 6 * 3_600; // 6 hours
pub const BPS_DENOMINATOR: u64 = 10000;
pub const MAX_PROPOSAL_TTL: u64 = 1209600;
pub const DEFAULT_INIT_EARLY_UNLOCK_PENALTY: &str = "0.7";

pub const DEFAULT_TIMELOCK_CONFIG: &[TimeLockConfig] = &[
    TimeLockConfig {
        duration: 0,
        reward_multiplier: 10000,
    },
    TimeLockConfig {
        duration: ONE_DAY * 30,
        reward_multiplier: 12500,
    },
    TimeLockConfig {
        duration: ONE_DAY * 30 * 3,
        reward_multiplier: 15000,
    },
    TimeLockConfig {
        duration: ONE_DAY * 30 * 6,
        reward_multiplier: 20000,
    },
];
