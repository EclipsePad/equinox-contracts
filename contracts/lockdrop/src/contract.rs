use cosmwasm_std::{
    ensure_eq, entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult,
};
use cw2::{get_contract_version, set_contract_version};
use equinox_msg::lockdrop::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use semver::Version;

use crate::{
    entry::{
        execute::{
            _handle_callback, receive_cw20, try_claim_all_rewards, try_claim_rewards,
            try_extend_lockup, try_increase_lockup, try_stake_to_vaults, try_unlock,
            try_update_config, try_update_reward_distribution_config,
        },
        instantiate::try_instantiate,
        query::{
            query_config, query_lp_lockup_info, query_lp_lockup_state, query_owner,
            query_reward_config, query_single_lockup_info, query_single_lockup_state,
            query_total_beclip_incentives, query_user_lp_lockup_info,
            query_user_single_lockup_info,
        },
    },
    error::ContractError,
    state::{CONTRACT_NAME, CONTRACT_VERSION},
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    try_instantiate(deps, env, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { new_config } => try_update_config(deps, info, new_config),
        ExecuteMsg::UpdateRewardDistributionConfig { new_config } => {
            try_update_reward_distribution_config(deps, env, info, new_config)
        }
        ExecuteMsg::IncreaseLockup {
            stake_type,
            duration,
        } => try_increase_lockup(deps, env, info, stake_type, duration),
        ExecuteMsg::ExtendLock {
            stake_type,
            from,
            to,
        } => try_extend_lockup(deps, env, info, stake_type, from, to),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Unlock {
            stake_type,
            duration,
            amount,
        } => try_unlock(deps, env, info, stake_type, duration, amount),
        ExecuteMsg::StakeToVaults {} => try_stake_to_vaults(deps, env, info),
        ExecuteMsg::ClaimRewards {
            stake_type,
            duration,
            assets,
        } => try_claim_rewards(deps, env, info, stake_type, duration, assets),
        ExecuteMsg::ClaimAllRewards {
            stake_type,
            with_flexible,
            assets,
        } => try_claim_all_rewards(deps, env, info, stake_type, with_flexible, assets),
        ExecuteMsg::Callback(msg) => _handle_callback(deps, env, info, msg),
    }
}

/// Exposes queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps, env)?)?),
        QueryMsg::RewardConfig {} => Ok(to_json_binary(&query_reward_config(deps, env)?)?),
        QueryMsg::Owner {} => Ok(to_json_binary(&query_owner(deps, env)?)?),
        QueryMsg::SingleLockupInfo {} => Ok(to_json_binary(&query_single_lockup_info(deps, env)?)?),
        QueryMsg::LpLockupInfo {} => Ok(to_json_binary(&query_lp_lockup_info(deps, env)?)?),
        QueryMsg::SingleLockupState {} => {
            Ok(to_json_binary(&query_single_lockup_state(deps, env)?)?)
        }
        QueryMsg::LpLockupState {} => Ok(to_json_binary(&query_lp_lockup_state(deps, env)?)?),
        QueryMsg::UserSingleLockupInfo { user } => Ok(to_json_binary(
            &query_user_single_lockup_info(deps, env, user)?,
        )?),
        QueryMsg::UserLpLockupInfo { user } => Ok(to_json_binary(&query_user_lp_lockup_info(
            deps, env, user,
        )?)?),
        QueryMsg::TotalbEclipIncentives {} => {
            Ok(to_json_binary(&query_total_beclip_incentives(deps)?)?)
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
