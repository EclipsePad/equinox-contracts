use crate::{
    math::{calculate_claimable, convert_token},
    state::{
        CONFIG, OWNER, REWARD_CONFIG, TOTAL_STAKE_INFO, TREASURY_REWARD, WITHDRAWABLE_BALANCE,
    },
};
use cosmwasm_std::{Addr, Deps, Env, StdResult, Uint128};
use equinox_msg::{
    token_converter::{Config, Reward, RewardConfig, RewardResponse},
    voter::QueryMsg as VoterQueryMsg,
};

/// query owner
pub fn query_owner(deps: Deps, _env: Env) -> StdResult<Addr> {
    let owner = OWNER.get(deps)?;
    Ok(owner.unwrap())
}

/// query config
pub fn query_config(deps: Deps, _env: Env) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

/// query reward config
pub fn query_reward_config(deps: Deps, _env: Env) -> StdResult<RewardConfig> {
    let config = REWARD_CONFIG.load(deps.storage)?;
    Ok(config)
}

// /// query reward
// pub fn query_rewards(deps: Deps, _env: Env) -> StdResult<RewardResponse> {
//     let config = CONFIG.load(deps.storage)?;
//     let reward_config = REWARD_CONFIG.load(deps.storage)?;
//     let mut treasury_reward = TREASURY_REWARD.load(deps.storage).unwrap_or_default();
//     let (total_deposit, total_shares): (Uint128, Uint128) = deps.querier.query_wasm_smart(
//         config.vxtoken_holder.to_string(),
//         &VoterQueryMsg::ConvertRatio {},
//     )?;
//     let total_staking = TOTAL_STAKE_INFO.load(deps.storage).unwrap_or_default();
//     let total_reward = calculate_claimable(
//         total_staking.xtoken,
//         total_staking.stake,
//         total_shares,
//         total_deposit,
//         total_staking.claimed,
//     );
//     let users_reward = total_reward.multiply_ratio(reward_config.users, 10000u32);
//     let dao_reward_point =
//         reward_config.treasury + reward_config.ce_holders + reward_config.stability_pool;
//     let dao_claimable = total_reward.checked_sub(users_reward).unwrap();
//     // amount to withdraw for staking pools
//     let ce_holders_reward =
//         dao_claimable.multiply_ratio(reward_config.ce_holders, dao_reward_point);
//     let stability_pool_reward =
//         dao_claimable.multiply_ratio(reward_config.stability_pool, dao_reward_point);
//     // amount to withdraw for staking pools
//     treasury_reward = treasury_reward
//         .checked_add(
//             dao_claimable
//                 .checked_sub(ce_holders_reward)
//                 .unwrap()
//                 .checked_sub(stability_pool_reward)
//                 .unwrap(),
//         )
//         .unwrap();
//     Ok(RewardResponse {
//         users_reward: Reward {
//             token: config.token_out.to_string(),
//             amount: convert_token(users_reward, total_shares, total_deposit),
//         },
//         ce_holders_reward: Reward {
//             token: config.xtoken.to_string(),
//             amount: ce_holders_reward,
//         },
//         stability_pool_reward: Reward {
//             token: config.xtoken.to_string(),
//             amount: stability_pool_reward,
//         },
//         treasury_reward: Reward {
//             token: config.xtoken.to_string(),
//             amount: treasury_reward,
//         },
//     })
// }

// /// query treasury reward
// pub fn query_treasury_reward(deps: Deps, _env: Env) -> StdResult<Uint128> {
//     let config = CONFIG.load(deps.storage)?;
//     let reward_config = REWARD_CONFIG.load(deps.storage)?;
//     let total_stake_info = TOTAL_STAKE_INFO.load(deps.storage)?;
//     let treasury_reward = TREASURY_REWARD.load(deps.storage)?;
//     let (total_deposit, total_shares): (Uint128, Uint128) = deps.querier.query_wasm_smart(
//         config.vxtoken_holder.to_string(),
//         &VoterQueryMsg::ConvertRatio {},
//     )?;
//     let claimable = calculate_claimable(
//         total_stake_info.xtoken,
//         total_stake_info.stake,
//         total_shares,
//         total_deposit,
//         total_stake_info.claimed,
//     );
//     let available_treasury_reward = treasury_reward
//         .checked_add(claimable.multiply_ratio(reward_config.treasury, 10000u32))
//         .unwrap();
//     Ok(available_treasury_reward)
// }

/// query treasury reward
pub fn query_withdrawable_balance(deps: Deps, _env: Env) -> StdResult<Uint128> {
    let withdrawable_balance = WITHDRAWABLE_BALANCE.load(deps.storage).unwrap_or_default();
    Ok(withdrawable_balance)
}
