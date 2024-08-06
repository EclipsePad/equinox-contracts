#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, from_json, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult,
};

use cw20::Cw20ReceiveMsg;

use eclipse_base::{
    error::ContractError,
    staking::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
};

use crate::actions::{
    execute as e, instantiate::try_instantiate, migrate::migrate_contract, query as q,
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
        ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender,
            amount,
            msg,
        }) => match from_json(msg)? {
            ExecuteMsg::Unbond {} => e::try_unbond(deps, env, info, Some(sender), Some(amount)),

            _ => Err(ContractError::WrongMessageType)?,
        },

        ExecuteMsg::Stake {} => e::try_stake(deps, env, info),

        ExecuteMsg::Unstake {} => e::try_unstake(deps, env, info),

        ExecuteMsg::Lock { amount, lock_tier } => e::try_lock(deps, env, info, amount, lock_tier),

        ExecuteMsg::Unlock {} => e::try_unlock(deps, env, info),

        ExecuteMsg::Relock {
            vault_creation_date,
            from_tier,
            to_tier,
        } => e::try_relock(deps, env, info, vault_creation_date, from_tier, to_tier),

        ExecuteMsg::Withdraw {
            vault_creation_date,
        } => e::try_withdraw(deps, env, info, vault_creation_date),

        ExecuteMsg::Bond {
            vault_creation_date_list,
        } => e::try_bond(deps, env, info, vault_creation_date_list),

        ExecuteMsg::BondFor {
            address_and_amount_list,
        } => e::try_bond_for(deps, env, info, address_and_amount_list),

        ExecuteMsg::Claim {} => e::try_claim(deps, env, info),

        ExecuteMsg::AggregateVaults {
            tier,
            vault_creation_date_list,
        } => e::try_aggregate_vaults(deps, env, info, tier, vault_creation_date_list),

        ExecuteMsg::AcceptAdminRole {} => e::try_accept_admin_role(deps, env, info),

        ExecuteMsg::UpdateConfig {
            admin,
            equinox_voter,
            beclip_minter,
            beclip_address,
            beclip_whitelist,
            lock_schedule,
            dao_treasury_address,
            penalty_multiplier,
            eclip_per_second_multiplier,
        } => e::try_update_config(
            deps,
            env,
            info,
            admin,
            equinox_voter,
            beclip_minter,
            beclip_address,
            beclip_whitelist,
            lock_schedule,
            dao_treasury_address,
            penalty_multiplier,
            eclip_per_second_multiplier,
        ),

        ExecuteMsg::UpdatePaginationConfig { pagination_amount } => {
            e::try_update_pagination_config(deps, env, info, pagination_amount)
        }

        ExecuteMsg::DecreaseBalance { amount } => e::try_decrease_balance(deps, env, info, amount),

        ExecuteMsg::UpdateStakingEssenceStorages { amount, start_from } => {
            e::try_update_staking_essence_storages(deps, env, info, amount, start_from)
        }

        ExecuteMsg::UpdateLockingEssenceStorages { amount, start_from } => {
            e::try_update_locking_essence_storages(deps, env, info, amount, start_from)
        }

        ExecuteMsg::FilterStakers { amount, start_from } => {
            e::try_filter_stakers(deps, env, info, amount, start_from)
        }

        ExecuteMsg::FilterLockers { amount, start_from } => {
            e::try_filter_lockers(deps, env, info, amount, start_from)
        }

        ExecuteMsg::Pause {} => e::try_pause(deps, env, info),

        ExecuteMsg::Unpause {} => e::try_unpause(deps, env, info),

        _ => Err(ContractError::WrongMessageType)?,
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryConfig {} => to_json_binary(&q::query_config(deps, env)?),

        QueryMsg::QueryPaginationConfig {} => {
            to_json_binary(&q::query_pagination_config(deps, env)?)
        }

        QueryMsg::QueryState {} => to_json_binary(&q::query_state(deps, env)?),

        QueryMsg::QueryStakerInfo { staker } => {
            to_json_binary(&q::query_staker_info(deps, env, staker)?)
        }

        QueryMsg::QueryTimeUntilDecreasingRewards {} => {
            to_json_binary(&q::query_time_until_decreasing_rewards(deps, env)?)
        }

        QueryMsg::QueryUsersAmount {} => to_json_binary(&q::query_users_amount(deps, env)?),

        QueryMsg::QueryAggregatedVault {
            user,
            tier,
            vault_creation_date_list,
        } => to_json_binary(&q::query_aggregated_vault(
            deps,
            env,
            user,
            tier,
            vault_creation_date_list,
        )?),

        QueryMsg::QueryBalances {} => to_json_binary(&q::query_balances(deps, env)?),

        QueryMsg::QueryEssence { user } => to_json_binary(&q::query_essence(deps, env, user)?),

        QueryMsg::QueryTotalEssence {} => to_json_binary(&q::query_total_essence(deps, env)?),

        QueryMsg::QueryWalletsPerTier {} => to_json_binary(&q::query_wallets_per_tier(deps, env)?),

        QueryMsg::QueryStakingEssenceList {
            amount,
            start_from,
            block_time,
        } => to_json_binary(&q::query_staking_essence_list(
            deps, env, amount, start_from, block_time,
        )?),

        QueryMsg::QueryLockingEssenceList { amount, start_from } => to_json_binary(
            &q::query_locking_essence_list(deps, env, amount, start_from)?,
        ),

        QueryMsg::QueryStorageVolumes {} => to_json_binary(&q::query_storage_volumes(deps, env)?),

        QueryMsg::QueryAprInfo {
            amount_to_add,
            staker_address,
        } => to_json_binary(&q::query_apr_info(
            deps,
            env,
            amount_to_add,
            staker_address,
        )?),

        QueryMsg::QueryStakerInfoList { amount, start_from } => {
            to_json_binary(&q::query_staker_info_list(deps, env, amount, start_from)?)
        }

        QueryMsg::QueryLockerInfoList { amount, start_from } => {
            to_json_binary(&q::query_locker_info_list(deps, env, amount, start_from)?)
        }

        QueryMsg::QueryRewardsReductionInfo {} => {
            to_json_binary(&q::query_rewards_reduction_info(deps, env)?)
        }

        QueryMsg::QueryPauseState {} => to_json_binary(&q::query_pause_state(deps, env)?),

        QueryMsg::QueryBondedVaultCreationDate { user } => {
            to_json_binary(&q::query_bonded_vault_creation_date(deps, env, user)?)
        }

        QueryMsg::QueryBeclipSupply {} => to_json_binary(&q::query_beclip_supply(deps, env)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    migrate_contract(deps, env, msg)
}
