use cosmwasm_std::{
    ensure_eq, entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult,
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use crate::{
    entry::{
        execute::{
            allow_users, block_users, claim, claim_all, receive_cw20, relock, unlock,
            update_config, update_owner,
        },
        instantiate::try_instantiate,
        query::{
            calculate_penalty, query_config, query_owner, query_reward, query_staking,
            query_total_staking, query_total_staking_by_duration,
        },
    },
    error::ContractError,
    state::{ALLOWED_USERS, CONTRACT_NAME, CONTRACT_VERSION},
};
use equinox_msg::timelock_staking::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, RelockingDetail,
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
        ExecuteMsg::UpdateOwner { owner } => update_owner(deps, env, info, owner),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Claim {
            duration,
            locked_at,
        } => claim(deps, env, info, duration, locked_at),
        ExecuteMsg::ClaimAll {} => claim_all(deps, env, info),
        ExecuteMsg::Unlock {
            duration,
            locked_at,
            amount,
            recipient,
        } => unlock(deps, env, info, duration, locked_at, amount, recipient),
        ExecuteMsg::Relock {
            from_duration,
            to_duration,
            relocks,
            recipient,
        } => {
            let recipient = recipient.unwrap_or(info.sender.to_string());
            relock(
                deps,
                env,
                RelockingDetail {
                    sender: info.sender,
                    recipient,
                    relocks,
                    from_duration,
                    to_duration,
                },
            )
        }
        ExecuteMsg::AllowUsers { users } => allow_users(deps, info, users),
        ExecuteMsg::BlockUsers { users } => block_users(deps, info, users),
    }
}

/// Exposes queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps, env)?)?),
        QueryMsg::Owner {} => Ok(to_json_binary(&query_owner(deps, env)?)?),
        QueryMsg::Staking { user } => Ok(to_json_binary(&query_staking(deps, env, user)?)?),
        QueryMsg::TotalStaking {} => Ok(to_json_binary(&query_total_staking(deps, env)?)?),
        QueryMsg::TotalStakingByDuration {} => Ok(to_json_binary(
            &query_total_staking_by_duration(deps, env)?,
        )?),
        QueryMsg::Reward { user } => Ok(to_json_binary(&query_reward(deps, env, user)?)?),
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
        (version >= storage_version),
        true,
        ContractError::VersionErr(storage_version.to_string())
    );

    if version > storage_version {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    }

    Ok(Response::new()
        .add_attribute("new_contract_name", CONTRACT_NAME)
        .add_attribute("new_contract_version", CONTRACT_VERSION))
}