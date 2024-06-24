use cosmwasm_std::{
    ensure_eq, entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdResult,
};
use cw2::{get_contract_version, set_contract_version};
use equinox_msg::voter::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use semver::Version;

use crate::{
    entry::{
        execute as e,
        instantiate::try_instantiate,
        query::{
            query_config, query_convert_ratio, query_owner, query_voter_info, query_voting_power,
        },
    },
    error::ContractError,
    state::CONTRACT_NAME,
};

/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const STAKE_ASTRO_REPLY_ID: u64 = 1;

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
        ExecuteMsg::UpdateConfig { config } => e::update_config(deps, env, info, config),
        ExecuteMsg::UpdateOwner { owner } => e::update_owner(deps, env, info, owner),
        ExecuteMsg::WithdrawBribeRewards {} => e::withdraw_bribe_rewards(deps, env, info),

        ExecuteMsg::SwapToEclipAstro {} => e::try_swap_to_eclip_astro(deps, env, info),
        ExecuteMsg::Vote { voting_list } => e::try_vote(deps, env, info, voting_list),
        ExecuteMsg::CaptureEssence {
            user_and_essence_list,
            total_essence,
        } => e::try_capture_essence(deps, env, info, user_and_essence_list, total_essence),
    }
}

/// Exposes queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps, env)?)?),
        QueryMsg::Owner {} => Ok(to_json_binary(&query_owner(deps, env)?)?),
        QueryMsg::VotingPower { address } => {
            Ok(to_json_binary(&query_voting_power(deps, env, address)?)?)
        }
        QueryMsg::ConvertRatio {} => Ok(to_json_binary(&query_convert_ratio(deps, env)?)?),

        QueryMsg::VoterInfo { address } => to_json_binary(&query_voter_info(deps, env, address)?),
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
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    let Reply { id, result } = reply;

    match id {
        STAKE_ASTRO_REPLY_ID => e::handle_stake_astro_reply(deps, env, &result),
        _ => Err(ContractError::UnknownReplyId(id)),
    }
}
