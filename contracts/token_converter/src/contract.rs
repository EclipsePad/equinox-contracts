use cosmwasm_std::{
    ensure_eq, entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdResult,
};
use cw2::{get_contract_version, set_contract_version};
use equinox_msg::token_converter::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use semver::Version;

use crate::{
    entry::{
        execute::{
            claim, claim_treasury_reward, handle_stake_reply, receive_cw20, update_config,
            update_owner, update_reward_config, withdraw_xtoken,
        },
        instantiate::try_instantiate,
        query::{
            query_config, query_owner, query_rewards, query_reward_config,
            query_withdrawable_balance,
        },
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
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::UpdateConfig { config } => update_config(deps, env, info, config),
        ExecuteMsg::UpdateRewardConfig { config } => update_reward_config(deps, env, info, config),
        ExecuteMsg::UpdateOwner { owner } => update_owner(deps, env, info, owner),
        ExecuteMsg::Claim {} => claim(deps, env, info),
        ExecuteMsg::ClaimTreasuryReward { amount } => {
            claim_treasury_reward(deps, env, info, amount)
        }
        ExecuteMsg::WithdrawAvailableBalance { amount, recipient } => {
            withdraw_xtoken(deps, env, info, amount, recipient)
        }
    }
}

/// Exposes queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Owner {} => Ok(to_json_binary(&query_owner(deps, env)?)?),
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps, env)?)?),
        QueryMsg::RewardConfig {} => Ok(to_json_binary(&query_reward_config(deps, env)?)?),
        QueryMsg::Rewards {} => Ok(to_json_binary(&query_rewards(deps, env)?)?),
        QueryMsg::WithdrawableBalance {} => {
            Ok(to_json_binary(&query_withdrawable_balance(deps, env)?)?)
        }
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        STAKE_TOKEN_REPLY_ID => handle_stake_reply(deps, env, msg),
        id => Err(ContractError::UnknownReplyId(id)),
    }
}
