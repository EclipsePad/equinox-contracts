use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw_asset::AssetInfo;

#[cw_serde]
pub struct State {
    /************** Vesting Params *************/
    // Start time of vesting
    pub start_time: u64,
    // Percent of tokens initially unlocked
    pub initial_unlock: u64,
    // Period before release vesting starts, also it unlocks initialUnlock reward tokens.
    pub lock_period: u64,
    // Period to release all reward token, after lockPeriod + vestingPeriod it releases 100% of reward tokens.
    pub vesting_period: u64,
    // Reward assetInfo of the project.
    pub reward_token: AssetInfo,
    // Total reward token amount
    pub distribution_amount: Uint128,

    /************** Status Info *************/
    // Sum of all user's vesting amount
    pub total_vesting_amount: Uint128,
    // User count
    pub usercount: u64,

    /************** ignored *************/
    // Intervals that the release happens. Every interval, releaseRate of tokens are released.
    pub release_interval: u64,
    // Release percent in each withdrawing interval
    pub release_rate: u64,
}

#[cw_serde]
#[derive(Default)]
pub struct UserInfo {
    // Total amount of tokens to be vested.
    pub total_amount: Uint128,
    // The amount that has been withdrawn.
    pub withdrawn_amount: Uint128,
    // The amount that has been withdrawn during vesting
    pub vesting_withdrawn_amount: Uint128,
}

#[cw_serde]
pub struct VestingSchedule {
    pub release_interval: u64,
    pub release_rate: u64,
    pub initial_unlock: u64,
    pub lock_period: u64,
    pub vesting_period: u64,
}
