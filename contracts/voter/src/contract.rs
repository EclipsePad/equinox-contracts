use cosmwasm_std::{
    ensure_eq, entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult,
};
use cw2::{get_contract_version, set_contract_version};
use equinox_msg::voter::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use semver::Version;

use crate::{
    entry::{
        execute::{
            place_vote, try_stake, update_config, update_owner, withdraw, withdraw_bribe_rewards
        },
        instantiate::try_instantiate,
        query::{query_config, query_owner, query_voting_power},
    },
    error::ContractError,
    state::CONTRACT_NAME,
};

/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const STAKE_TOKEN_REPLY_ID: u64 = 1;

/// Creates a new contract with the specified parameters in the [`InstantiateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    try_instantiate(deps, _env, _info, msg)
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
        ExecuteMsg::Stake{} => try_stake(deps, env, info),
        ExecuteMsg::UpdateConfig { config } => update_config(deps, env, info, config),
        ExecuteMsg::UpdateOwner { owner } => update_owner(deps, env, info, owner),
        ExecuteMsg::Withdraw { amount, recipient } => withdraw(deps, env, info, amount, recipient),
        ExecuteMsg::WithdrawBribeRewards {} => withdraw_bribe_rewards(deps, env, info),
        ExecuteMsg::PlaceVote { gauge, votes } => place_vote(deps, env, info, gauge, votes),
    }
}

/// Exposes queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps, env)?)?),
        QueryMsg::Owner {} => Ok(to_json_binary(&query_owner(deps, env)?)?),
        QueryMsg::VotingPower {} => Ok(to_json_binary(&query_voting_power(deps, env)?)?),
    }
}

/// Manages contract migration.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    ensure_eq!(
        (storage_version < version),
        true,
        ContractError::VersionErr(storage_version.to_string())
    );

    if storage_version < version {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    }

    Ok(Response::new()
        .add_attribute("new_contract_name", CONTRACT_NAME)
        .add_attribute("new_contract_version", CONTRACT_VERSION))
}
