use cosmwasm_std::{
    attr, ensure, ensure_eq, entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult,
};
use cw2::{get_contract_version, set_contract_version};
use equinox_msg::{
    common::{drop_ownership_proposal, propose_new_owner},
    lockdrop::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, StakeType},
};
use semver::Version;

use crate::{
    entry::{
        execute::{
            _handle_callback, handle_claim_lp_staking_asset_rewards,
            handle_claim_rewards_and_unlock_for_lp_lockup,
            handle_claim_rewards_and_unlock_for_single_lockup,
            handle_claim_single_staking_asset_rewards, handle_enable_claims, handle_extend_lock,
            handle_increase_eclip_incentives, handle_lp_locking_withdraw, handle_restake,
            handle_single_locking_withdraw, handle_stake_lp_vault, handle_stake_single_vault,
            handle_update_config, receive_cw20,
        },
        instantiate::try_instantiate,
        query::{
            query_config, query_lp_lockup_info, query_lp_lockup_state, query_owner,
            query_single_lockup_info, query_single_lockup_state, query_user_lp_lockup_info,
            query_user_single_lockup_info,
        },
    },
    error::ContractError,
    state::{CONTRACT_NAME, CONTRACT_VERSION, OWNER, OWNERSHIP_PROPOSAL},
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
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { new_config } => handle_update_config(deps, info, new_config),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::ExtendLock {
            stake_type,
            amount,
            from,
            to,
        } => handle_extend_lock(deps, env, info, stake_type, amount, from, to),
        ExecuteMsg::SingleLockupWithdraw { amount, duration } => {
            handle_single_locking_withdraw(deps, env, info, amount, duration)
        }
        ExecuteMsg::LpLockupWithdraw { amount, duration } => {
            handle_lp_locking_withdraw(deps, env, info, amount, duration)
        }
        ExecuteMsg::IncreaseEclipIncentives { stake_type } => {
            handle_increase_eclip_incentives(deps, env, info, stake_type)
        }
        ExecuteMsg::StakeToSingleVault {} => handle_stake_single_vault(deps, env, info),
        ExecuteMsg::StakeToLpVault {} => handle_stake_lp_vault(deps, env, info),
        ExecuteMsg::RestakeSingleStaking { amount, from, to } => {
            handle_restake(deps, env, info, amount, from, to)
        }
        ExecuteMsg::EnableClaims {} => handle_enable_claims(deps, env, info),
        ExecuteMsg::ClaimRewardsAndOptionallyUnlock {
            stake_type,
            duration,
            amount,
        } => match stake_type {
            StakeType::SingleStaking => handle_claim_rewards_and_unlock_for_single_lockup(
                deps,
                env,
                duration,
                info.sender,
                amount,
            ),
            StakeType::LpStaking => {
                handle_claim_rewards_and_unlock_for_lp_lockup(deps, env, info, duration, amount)
            }
        },
        ExecuteMsg::Callback(msg) => _handle_callback(deps, env, info, msg),
        ExecuteMsg::ClaimAssetReward {
            recipient,
            stake_type,
            duration,
        } => {
            let recipient = recipient.map_or_else(
                || Ok(info.sender.clone()),
                |recip_addr| deps.api.addr_validate(&recip_addr),
            )?;
            match stake_type {
                StakeType::SingleStaking => handle_claim_single_staking_asset_rewards(
                    deps,
                    env,
                    info.sender,
                    recipient,
                    duration,
                ),
                StakeType::LpStaking => handle_claim_lp_staking_asset_rewards(
                    deps,
                    env,
                    info.sender,
                    recipient,
                    duration,
                ),
            }
        }
        ExecuteMsg::ProposeNewOwner { owner, expires_in } => {
            let old_owner = OWNER.get(deps.as_ref()).unwrap().unwrap();
            Ok(propose_new_owner(
                deps,
                info,
                env,
                owner,
                expires_in,
                old_owner,
                OWNERSHIP_PROPOSAL,
            )?)
        }
        ExecuteMsg::DropOwnershipProposal {} => {
            let old_owner = OWNER.get(deps.as_ref()).unwrap().unwrap();
            Ok(drop_ownership_proposal(
                deps.branch(),
                info,
                old_owner,
                OWNERSHIP_PROPOSAL,
            )?)
        }
        ExecuteMsg::ClaimOwnership {} => {
            let p = OWNERSHIP_PROPOSAL.load(deps.storage)?;
            OWNER.assert_admin(deps.as_ref(), &info.sender)?;

            ensure!(
                env.block.time.seconds() <= p.ttl,
                ContractError::OwnershipProposalExpired {}
            );

            OWNERSHIP_PROPOSAL.remove(deps.storage);

            OWNER.set(deps.branch(), Some(p.owner.clone()))?;

            Ok(Response::new().add_attributes(vec![
                attr("action", "claim_ownership"),
                attr("new_owner", p.owner),
            ]))
        }
    }
}

/// Exposes queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps, env)?)?),
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
