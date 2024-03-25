use cosmwasm_std::{
    ensure_eq, entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult,
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use crate::{
    entry::{
        execute::{
            flexible_stake, flexible_stake_claim, flexible_unstake, restake, timelock_stake,
            timelock_stake_claim, timelock_stake_claim_all, timelock_unstake, update_config,
            update_owner,
        },
        instantiate::try_instantiate,
        query::{
            query_config, query_owner, query_pending_rewards, query_reward, query_total_staking,
        },
    },
    error::ContractError,
    state::{CONTRACT_NAME, CONTRACT_VERSION},
};
use equinox_msg::reward_distributor::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

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
        ExecuteMsg::FlexibleStake { user, amount } => flexible_stake(deps, env, info, user, amount),
        ExecuteMsg::TimelockStake {
            user,
            amount,
            duration,
        } => timelock_stake(deps, env, info, user, amount, duration),
        ExecuteMsg::FlexibleStakeClaim { user } => flexible_stake_claim(deps, env, info, user),
        ExecuteMsg::TimelockStakeClaim {
            user,
            duration,
            locked_at,
        } => timelock_stake_claim(deps, env, info, user, duration, locked_at),
        ExecuteMsg::TimelockStakeClaimAll { user } => {
            timelock_stake_claim_all(deps, env, info, user)
        }
        ExecuteMsg::FlexibleUnstake { user, amount } => {
            flexible_unstake(deps, env, info, user, amount)
        }
        ExecuteMsg::TimelockUnstake {
            user,
            duration,
            locked_at,
        } => timelock_unstake(deps, env, info, user, duration, locked_at),
        ExecuteMsg::Restake {
            user,
            from,
            locked_at,
            to,
        } => restake(deps, env, info, user, from, locked_at, to),
    }
}

/// Exposes queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps, env)?)?),
        QueryMsg::Owner {} => Ok(to_json_binary(&query_owner(deps, env)?)?),
        QueryMsg::Reward { user } => Ok(to_json_binary(&query_reward(deps, env, user)?)?),
        QueryMsg::TotalStaking {} => Ok(to_json_binary(&query_total_staking(deps, env)?)?),
        QueryMsg::PendingRewards {} => Ok(to_json_binary(&query_pending_rewards(deps, env)?)?),
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
