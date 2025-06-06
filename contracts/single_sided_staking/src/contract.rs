use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult,
};

use crate::{
    entry::{
        execute::{
            _handle_callback, add_rewards, allow_users, block_users, claim, claim_all,
            claim_blacklist_rewards, claim_ownership, drop_ownership_proposal,
            handle_swap_to_astro_reply, propose_new_owner, restake, stake, unbond, unstake,
            update_config, withdraw,
        },
        instantiate::try_instantiate,
        migrate::migrate_contract,
        query::{
            calculate_penalty, query_blacklist, query_blacklist_rewards,
            query_calculate_penalty_amount, query_calculate_reward, query_config,
            query_eclipastro_rewards, query_owner, query_reward, query_reward_list,
            query_reward_schedule, query_staking, query_total_staking,
            query_total_staking_by_duration, query_unbonded,
        },
    },
    error::ContractError,
    state::{ALLOWED_USERS, SWAP_TO_ASTRO_REPLY_ID},
};
use equinox_msg::single_sided_staking::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, RestakeData,
};

// Note, you can use StdResult in some functions where you do not
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
        ExecuteMsg::ProposeNewOwner { owner, expires_in } => {
            propose_new_owner(deps, env, info, owner, expires_in)
        }
        ExecuteMsg::DropOwnershipProposal {} => drop_ownership_proposal(deps, info),
        ExecuteMsg::ClaimOwnership {} => claim_ownership(deps, env, info),
        ExecuteMsg::Claim {
            duration,
            locked_at,
            assets,
        } => claim(deps, env, info, duration, locked_at, assets),
        ExecuteMsg::ClaimAll {
            with_flexible,
            assets,
        } => claim_all(deps, env, info, with_flexible, assets),
        ExecuteMsg::Stake {
            duration,
            recipient,
        } => stake(deps, env, info, duration, recipient),
        ExecuteMsg::Unstake {
            duration,
            locked_at,
            amount,
            recipient,
        } => unstake(deps, env, info, duration, locked_at, amount, recipient),

        ExecuteMsg::Unbond {
            duration,
            locked_at,
            period,
        } => unbond(deps, env, info, duration, locked_at, period),

        ExecuteMsg::Withdraw { recipient } => withdraw(deps, env, info, recipient),

        ExecuteMsg::Restake {
            from_duration,
            locked_at,
            amount,
            to_duration,
            recipient,
        } => {
            let recipient = recipient.unwrap_or(info.sender.to_string());
            let locked_at = locked_at.unwrap_or_default();
            let funds = info.funds;
            restake(
                deps,
                env,
                funds,
                RestakeData {
                    from_duration,
                    locked_at,
                    amount,
                    to_duration,
                    sender: info.sender.to_string(),
                    recipient,
                },
            )
        }
        ExecuteMsg::AllowUsers { users } => allow_users(deps, info, users),
        ExecuteMsg::BlockUsers { users } => block_users(deps, info, users),
        ExecuteMsg::Callback(msg) => _handle_callback(deps, env, info, msg),
        ExecuteMsg::AddRewards {
            from,
            duration,
            eclip,
            beclip,
        } => add_rewards(deps, env, info, from, duration, eclip, beclip),
        ExecuteMsg::ClaimBlacklistRewards {} => claim_blacklist_rewards(deps, env),
    }
}

/// Exposes queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps, env)?)?),
        QueryMsg::Owner {} => Ok(to_json_binary(&query_owner(deps, env)?)?),
        QueryMsg::Staking { user } => Ok(to_json_binary(&query_staking(deps, env, user)?)?),

        QueryMsg::Unbonded { user } => to_json_binary(&query_unbonded(deps, env, user)?),

        QueryMsg::TotalStaking {} => Ok(to_json_binary(&query_total_staking(deps, env)?)?),
        QueryMsg::TotalStakingByDuration { timestamp } => Ok(to_json_binary(
            &query_total_staking_by_duration(deps, env, timestamp)?,
        )?),
        QueryMsg::Reward {
            user,
            duration,
            locked_at,
        } => Ok(to_json_binary(&query_reward(
            deps, env, user, duration, locked_at,
        )?)?),
        QueryMsg::CalculateReward {
            amount,
            duration,
            locked_at,
            from,
            to,
        } => Ok(to_json_binary(&query_calculate_reward(
            deps, env, amount, duration, locked_at, from, to,
        )?)?),
        QueryMsg::CalculatePenalty {
            amount,
            duration,
            locked_at,
        } => Ok(to_json_binary(&calculate_penalty(
            deps, env, amount, duration, locked_at,
        )?)?),
        QueryMsg::IsAllowed { user } => {
            let is_allowed = ALLOWED_USERS.load(deps.storage, &user).unwrap_or_default();
            Ok(to_json_binary(&is_allowed)?)
        }
        QueryMsg::EclipastroRewards {} => {
            Ok(to_json_binary(&query_eclipastro_rewards(deps, env)?)?)
        }
        QueryMsg::Blacklist {} => Ok(to_json_binary(&query_blacklist(deps)?)?),
        QueryMsg::BlacklistRewards => Ok(to_json_binary(&query_blacklist_rewards(deps, env)?)?),
        QueryMsg::RewardSchedule { from } => {
            Ok(to_json_binary(&query_reward_schedule(deps, env, from)?)?)
        }
        QueryMsg::RewardList { user } => Ok(to_json_binary(&query_reward_list(deps, env, user)?)?),
        QueryMsg::CalculatePenaltyAmount {
            amount,
            duration,
            locked_at,
        } => Ok(to_json_binary(&query_calculate_penalty_amount(
            deps, env, amount, duration, locked_at,
        )?)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    let Reply { id, result } = reply;

    match id {
        SWAP_TO_ASTRO_REPLY_ID => handle_swap_to_astro_reply(deps, env, &result),
        _ => Err(ContractError::UnknownReplyId(id)),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    migrate_contract(deps, env, msg)
}
