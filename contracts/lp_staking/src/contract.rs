use cosmwasm_std::{
    ensure_eq, entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult,
};
use cw2::{get_contract_version, set_contract_version};
use equinox_msg::lp_staking::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use semver::Version;

use crate::{
    entry::{
        execute::{
            _handle_callback, add_rewards, allow_users, block_users, claim,
            claim_blacklist_rewards, claim_ownership, drop_ownership_proposal,
            handle_withdraw_liquidity_reply, propose_new_owner, stake, unbond, unstake,
            update_config, update_reward_distribution, withdraw,
        },
        instantiate::try_instantiate,
        query::{
            query_blacklist, query_blacklist_rewards, query_config, query_owner, query_reward,
            query_reward_distribution, query_reward_schedule, query_reward_weights, query_staking,
            query_total_staking, query_unbonded, query_user_reward_weights,
        },
    },
    error::ContractError,
    state::{ALLOWED_USERS, CONTRACT_NAME, CONTRACT_VERSION, REWARD, WITHDRAW_LIQUIDITY_REPLY_ID},
};

// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    try_instantiate(deps, env, info, msg)
}

/// Exposes execute functions available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { config } => update_config(deps, env, info, config),
        ExecuteMsg::UpdateRewardDistribution { distribution } => {
            update_reward_distribution(deps, env, info, distribution)
        }
        ExecuteMsg::ProposeNewOwner { owner, expires_in } => {
            propose_new_owner(deps, env, info, owner, expires_in)
        }
        ExecuteMsg::DropOwnershipProposal {} => drop_ownership_proposal(deps, info),
        ExecuteMsg::ClaimOwnership {} => claim_ownership(deps, env, info),
        ExecuteMsg::Stake { recipient } => stake(deps, env, info, recipient),
        ExecuteMsg::Claim { assets } => {
            claim(deps, env, info.clone(), info.sender.to_string(), assets)
        }
        ExecuteMsg::Callback(msg) => _handle_callback(deps, env, info, msg),
        ExecuteMsg::Unstake { amount, recipient } => unstake(deps, env, info, amount, recipient),

        ExecuteMsg::Unbond { amount, period } => unbond(deps, env, info, amount, period),
        ExecuteMsg::Withdraw { recipient } => withdraw(deps, env, info, recipient),

        ExecuteMsg::AddRewards {
            from,
            duration,
            eclip,
            beclip,
        } => add_rewards(deps, env, info, from, duration, eclip, beclip),
        ExecuteMsg::ClaimBlacklistRewards {} => claim_blacklist_rewards(deps, env),
        ExecuteMsg::AllowUsers { users } => allow_users(deps, info, users),
        ExecuteMsg::BlockUsers { users } => block_users(deps, info, users),
    }
}

/// Exposes queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps, env)?)?),
        QueryMsg::RewardDistribution {} => {
            Ok(to_json_binary(&query_reward_distribution(deps, env)?)?)
        }
        QueryMsg::Owner {} => Ok(to_json_binary(&query_owner(deps, env)?)?),
        QueryMsg::Staking { user } => Ok(to_json_binary(&query_staking(deps, env, user)?)?),

        QueryMsg::Unbonded { user } => to_json_binary(&query_unbonded(deps, env, user)?),

        QueryMsg::TotalStaking {} => Ok(to_json_binary(&query_total_staking(deps, env)?)?),
        QueryMsg::Reward { user } => Ok(to_json_binary(&query_reward(deps, env, user)?)?),
        QueryMsg::RewardWeights {} => Ok(to_json_binary(&query_reward_weights(deps, env)?)?),
        QueryMsg::UserRewardWeights { user } => Ok(to_json_binary(&query_user_reward_weights(
            deps, env, user,
        )?)?),
        QueryMsg::Blacklist {} => Ok(to_json_binary(&query_blacklist(deps)?)?),
        QueryMsg::BlacklistRewards => Ok(to_json_binary(&query_blacklist_rewards(deps, env)?)?),
        QueryMsg::IsAllowed { user } => {
            let is_allowed = ALLOWED_USERS.load(deps.storage, &user).unwrap_or_default();
            Ok(to_json_binary(&is_allowed)?)
        }
        QueryMsg::RewardSchedule { from } => {
            Ok(to_json_binary(&query_reward_schedule(deps, env, from)?)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    let Reply { id, result } = reply;

    match id {
        WITHDRAW_LIQUIDITY_REPLY_ID => handle_withdraw_liquidity_reply(deps, env, &result),
        _ => Err(ContractError::UnknownReplyId(id)),
    }
}

/// Manages contract migration.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;
    let contract_name = get_contract_version(deps.storage)?.contract;

    match msg.update_contract_name {
        Some(true) => {}
        _ => {
            ensure_eq!(
                contract_name,
                CONTRACT_NAME,
                ContractError::ContractNameErr(contract_name)
            );
        }
    }

    ensure_eq!(
        (version > storage_version),
        true,
        ContractError::VersionErr(storage_version.to_string())
    );

    if version > storage_version {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    }

    if let Some(update_rewards) = msg.update_rewards {
        let (time_config, new_reward) = update_rewards;
        REWARD.update(deps.storage, time_config, |reward| -> StdResult<_> {
            if let Some(old_reward) = reward {
                if old_reward.eclip + old_reward.beclip == new_reward.eclip + new_reward.beclip {
                    return Ok(new_reward);
                }
            }
            Err(StdError::generic_err("Update Rewards error"))
        })?;
    }

    Ok(Response::new().add_attribute("new_contract_version", CONTRACT_VERSION))
}
