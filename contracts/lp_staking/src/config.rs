use equinox_msg::lp_staking::RewardDistribution;

pub const DEFAULT_REWARD_DISTRIBUTION: RewardDistribution = RewardDistribution {
    users: 8000,
    treasury: 1350,
    ce_holders: 400,
    stability_pool: 250,
};

pub const ONE_DAY: u64 = 86400u64;
pub const BPS_DENOMINATOR: u32 = 10_000u32;
pub const DEFAULT_REWARD_DISTRIBUTION_PERIOD: u64 = 30 * ONE_DAY;

pub const DEFAULT_REWARD_PERIOD: u64 = 31_536_000; // 1 year
pub const MAX_PROPOSAL_TTL: u64 = 1209600;
