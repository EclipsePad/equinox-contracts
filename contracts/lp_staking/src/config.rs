use equinox_msg::lp_staking::RewardDistribution;

pub const DEFAULT_REWARD_DISTRIBUTION: RewardDistribution = RewardDistribution {
    users: 8000,
    treasury: 1350,
    ce_holders: 400,
    stability_pool: 250,
};

pub const DEFAULT_ECLIP_DAILY_REWARD: u128 = 19_200_000_000u128;
pub const DEFAULT_BECLIP_DAILY_REWARD: u128 = 12_800_000_000u128;

pub const BPS_DENOMINATOR: u32 = 10_000u32;

pub const DEFAULT_REWARD_PERIOD: u64 = 31_536_000; // 1 year
