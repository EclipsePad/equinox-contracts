use equinox_msg::lp_staking::RewardConfig;

pub const DEFAULT_REWARD_CONFIG: RewardConfig = RewardConfig {
    users: 8000,
    treasury: 1350,
    ce_holders: 400,
    stability_pool: 250,
};

pub const BPS_DENOMINATOR: u32 = 10000u32;
