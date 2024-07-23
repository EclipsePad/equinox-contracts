use cosmwasm_std::{
    coins, to_json_binary, Addr, BankMsg, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Order,
    Response, StdResult, Uint128, WasmMsg,
};
use cw_storage_plus::Bound;

use eclipse_base::{
    assets::TokenUnverified,
    error::ContractError,
    staking::{
        state::{
            BECLIP_SUPPLY, BONDED_VAULT_CREATION_DATE, CONFIG, IS_PAUSED, LOCKER_INFO,
            LOCKING_ESSENCE, LOCK_STATES, PAGINATION_CONFIG, STAKER_INFO, STAKE_STATE,
            STAKING_ESSENCE_COMPONENTS, TIER_4, TOTAL_LOCKING_ESSENCE,
            TOTAL_STAKING_ESSENCE_COMPONENTS, TRANSFER_ADMIN_STATE, TRANSFER_ADMIN_TIMEOUT,
        },
        types::{
            Config, LockerInfo, PaginationConfig, StakerInfo, State, TransferAdminState, Vault,
        },
    },
    utils::{add_funds_to_exec_msg, check_funds, unwrap_field, FundsType},
};
use equinox_msg::voter::types::EssenceInfo;

use crate::{helpers, math};

pub fn try_stake(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    helpers::check_pause_state(deps.as_ref())?;
    let (sender_address, asset_amount, asset_info) = check_funds(
        deps.as_ref(),
        &info,
        FundsType::Single {
            sender: None,
            amount: None,
        },
    )?;

    let block_time = env.block.time.seconds();
    let Config {
        staking_token,
        lock_schedule,
        seconds_per_essence,
        eclip_per_second,
        eclip_per_second_multiplier,
        equinox_voter,
        ..
    } = CONFIG.load(deps.storage)?;

    if asset_info.try_get_native()? != staking_token {
        Err(ContractError::WrongToken)?;
    }

    // don't allow too much vaults
    helpers::v3::check_vaults_amount(deps.storage, &sender_address)?;

    // get eclip_per_second values
    let (decreasing_rewards_date, eclip_per_second_before, eclip_per_second_after) =
        helpers::v3::split_eclip_per_second(
            deps.storage,
            eclip_per_second_multiplier,
            eclip_per_second,
            block_time,
        )?;

    // accumulate rewards
    helpers::v3::accumulate_rewards(
        deps.storage,
        &lock_schedule,
        block_time,
        decreasing_rewards_date,
        eclip_per_second_before,
        eclip_per_second_after,
    )?;

    let total_essence =
        helpers::v3::get_total_essence(deps.storage, block_time, seconds_per_essence)?;

    // core logic
    STAKER_INFO.update(
        deps.storage,
        &sender_address,
        |staker_info| -> StdResult<StakerInfo> {
            let mut staker = staker_info.unwrap_or_default();

            // don't allow multiple vaults with same creation date
            if !staker.vaults.is_empty()
                && staker.vaults.last().unwrap().creation_date == block_time
            {
                Err(ContractError::MultipleVaultsWithSameCreationDate)?;
            }

            staker.vaults.push(Vault {
                amount: asset_amount,
                creation_date: block_time,
                claim_date: block_time,
                accumulated_rewards: Uint128::zero(),
            });

            staker.vaults = vec![math::v3::calc_aggregated_vault(
                &staker.vaults,
                0,
                decreasing_rewards_date,
                block_time,
                seconds_per_essence,
                eclip_per_second_before,
                eclip_per_second_after,
                total_essence,
            )];

            Ok(staker)
        },
    )?;

    STAKE_STATE.update(deps.storage, |mut stake_state| -> StdResult<State> {
        stake_state.total_bond_amount += asset_amount;

        Ok(stake_state)
    })?;

    // update essence storages
    let staking_vaults = &STAKER_INFO
        .load(deps.storage, &sender_address)
        .unwrap_or_default()
        .vaults;
    let (a1, b1) = STAKING_ESSENCE_COMPONENTS
        .load(deps.storage, &sender_address)
        .unwrap_or_default();
    let (a2, b2) = math::v3::calc_components_from_staking_vaults(staking_vaults);

    STAKING_ESSENCE_COMPONENTS.save(deps.storage, &sender_address, &(a2, b2))?;
    TOTAL_STAKING_ESSENCE_COMPONENTS
        .update(deps.storage, |(a, b)| -> StdResult<(Uint128, Uint128)> {
            Ok((a + a2 - a1, b + b2 - b1))
        })?;

    helpers::v3::update_eclip_per_second(deps.storage, block_time, decreasing_rewards_date)?;

    // send essence snapshot to equinox voter
    let mut response = Response::new().add_attributes(vec![
        ("action", "try_stake"),
        ("amount", &asset_amount.to_string()),
    ]);

    if let Some(x) = equinox_voter {
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: x.to_string(),
            msg: to_json_binary(
                &equinox_msg::voter::msg::ExecuteMsg::UpdateEssenceAllocation {
                    user_and_essence_list: helpers::get_essence_snapshot(
                        deps.storage,
                        &sender_address,
                    ),
                },
            )?,
            funds: vec![],
        });

        response = response.add_message(msg);
    }

    Ok(response)
}

pub fn try_lock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    lock_tier: u64,
) -> Result<Response, ContractError> {
    helpers::check_pause_state(deps.as_ref())?;

    let sender = &info.sender;
    let block_time = env.block.time.seconds();
    let lock_tier = lock_tier as usize;
    let Config {
        lock_schedule,
        seconds_per_essence,
        eclip_per_second,
        eclip_per_second_multiplier,
        equinox_voter,
        ..
    } = CONFIG.load(deps.storage)?;

    if amount.is_zero() {
        Err(ContractError::ZeroAmount)?;
    }

    let mut staker = STAKER_INFO
        .load(deps.storage, sender)
        .map_err(|_| ContractError::StakerIsNotFound)?;

    let staked_total = staker
        .vaults
        .iter()
        .fold(Uint128::zero(), |acc, cur| acc + cur.amount);

    if amount > staked_total {
        Err(ContractError::ExceedingLockingAmount)?;
    }

    if lock_tier >= CONFIG.load(deps.storage)?.lock_schedule.len() {
        Err(ContractError::LockTierIsOutOfRange)?;
    }

    // don't allow too much vaults
    helpers::v3::check_vaults_amount(deps.storage, sender)?;

    // don't allow multiple vaults with same creation date
    if LOCKER_INFO
        .load(deps.storage, sender)
        .unwrap_or_default()
        .iter()
        .any(|x| !x.vaults.is_empty() && x.vaults.last().unwrap().creation_date == block_time)
    {
        Err(ContractError::MultipleVaultsWithSameCreationDate)?;
    }

    // get eclip_per_second values
    let (decreasing_rewards_date, eclip_per_second_before, eclip_per_second_after) =
        helpers::v3::split_eclip_per_second(
            deps.storage,
            eclip_per_second_multiplier,
            eclip_per_second,
            block_time,
        )?;

    // accumulate rewards
    helpers::v3::accumulate_rewards(
        deps.storage,
        &lock_schedule,
        block_time,
        decreasing_rewards_date,
        eclip_per_second_before,
        eclip_per_second_after,
    )?;

    // core logic
    let total_essence =
        helpers::v3::get_total_essence(deps.storage, block_time, seconds_per_essence)?;

    let mut new_locking_vaults_amount = Uint128::zero();
    let mut new_accumulated_rewards = Uint128::zero();
    let mut unspent_amount = amount;
    let mut new_staking_vaults: Vec<Vault> = vec![];

    for Vault {
        amount,
        creation_date,
        claim_date,
        accumulated_rewards,
    } in staker.vaults
    {
        // claim staking rewards
        let staking_essence_per_vault = math::v3::calc_staking_essence_per_vault(
            amount,
            creation_date,
            block_time,
            seconds_per_essence,
        );

        let staking_rewards = math::v3::calc_staking_rewards_per_vault(
            accumulated_rewards,
            staking_essence_per_vault,
            claim_date,
            decreasing_rewards_date,
            block_time,
            eclip_per_second_before,
            eclip_per_second_after,
            total_essence,
        );

        // consume staking vault completely
        if unspent_amount >= amount {
            new_locking_vaults_amount += amount;
            new_accumulated_rewards += staking_rewards;
            unspent_amount -= amount;
        }
        // consume staking vault partially
        else {
            new_locking_vaults_amount += unspent_amount;

            new_staking_vaults.push(Vault {
                amount: amount - unspent_amount,
                creation_date,
                claim_date: block_time,
                accumulated_rewards: staking_rewards,
            });

            unspent_amount = Uint128::zero();
        }
    }

    // update staker info storage
    if new_staking_vaults.is_empty() {
        STAKER_INFO.remove(deps.storage, sender);
    } else {
        staker.vaults = new_staking_vaults.clone();
        STAKER_INFO.save(deps.storage, sender, &staker)?;
    }

    STAKE_STATE.update(deps.storage, |mut stake_state| -> StdResult<State> {
        stake_state.total_bond_amount -= amount;

        Ok(stake_state)
    })?;

    LOCKER_INFO.update(
        deps.storage,
        sender,
        |locker_info| -> StdResult<Vec<LockerInfo>> {
            let mut locker_infos = locker_info.unwrap_or_default();

            while lock_tier >= locker_infos.len() {
                locker_infos.push(LockerInfo {
                    lock_tier: locker_infos.len() as u64,
                    vaults: vec![],
                })
            }

            // we can aggregate all new locking vaults in single vault as they were created
            // at the same time
            locker_infos[lock_tier].vaults.push(Vault {
                amount: new_locking_vaults_amount,
                creation_date: block_time,
                claim_date: block_time,
                accumulated_rewards: new_accumulated_rewards,
            });

            Ok(locker_infos)
        },
    )?;

    LOCK_STATES.update(deps.storage, |mut lock_states| -> StdResult<Vec<State>> {
        lock_states[lock_tier].total_bond_amount += amount;

        Ok(lock_states)
    })?;

    // update essence storages
    let (a1, b1) = STAKING_ESSENCE_COMPONENTS
        .load(deps.storage, sender)
        .unwrap_or_default();
    let (a2, b2) = math::v3::calc_components_from_staking_vaults(&new_staking_vaults);

    TOTAL_STAKING_ESSENCE_COMPONENTS
        .update(deps.storage, |(a, b)| -> StdResult<(Uint128, Uint128)> {
            Ok((a + a2 - a1, b + b2 - b1))
        })?;

    if new_staking_vaults.is_empty() {
        STAKING_ESSENCE_COMPONENTS.remove(deps.storage, sender);
    } else {
        STAKING_ESSENCE_COMPONENTS.save(deps.storage, sender, &(a2, b2))?;
    }

    let locking_essence_before = LOCKING_ESSENCE
        .load(deps.storage, sender)
        .unwrap_or_default();
    let locking_essence_after = math::v3::calc_locking_essence(
        &LOCKER_INFO.load(deps.storage, sender)?,
        &lock_schedule,
        seconds_per_essence,
    );

    LOCKING_ESSENCE.save(deps.storage, sender, &locking_essence_after)?;
    TOTAL_LOCKING_ESSENCE.update(deps.storage, |x| -> StdResult<Uint128> {
        Ok(x + locking_essence_after - locking_essence_before)
    })?;

    helpers::v3::update_eclip_per_second(deps.storage, block_time, decreasing_rewards_date)?;

    // send essence snapshot to equinox voter
    let mut response = Response::new().add_attributes(vec![
        ("action", "try_lock"),
        ("amount", &amount.to_string()),
    ]);

    if let Some(x) = equinox_voter {
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: x.to_string(),
            msg: to_json_binary(
                &equinox_msg::voter::msg::ExecuteMsg::UpdateEssenceAllocation {
                    user_and_essence_list: helpers::get_essence_snapshot(deps.storage, sender),
                },
            )?,
            funds: vec![],
        });

        response = response.add_message(msg);
    }

    Ok(response)
}

// withdraw staked funds
pub fn try_unstake(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    helpers::check_pause_state(deps.as_ref())?;

    let sender = &info.sender;
    let block_time = env.block.time.seconds();
    let Config {
        staking_token,
        lock_schedule,
        seconds_per_essence,
        eclip_per_second,
        eclip_per_second_multiplier,
        equinox_voter,
        ..
    } = CONFIG.load(deps.storage)?;

    // get eclip_per_second values
    let (decreasing_rewards_date, eclip_per_second_before, eclip_per_second_after) =
        helpers::v3::split_eclip_per_second(
            deps.storage,
            eclip_per_second_multiplier,
            eclip_per_second,
            block_time,
        )?;

    // accumulate rewards
    helpers::v3::accumulate_rewards(
        deps.storage,
        &lock_schedule,
        block_time,
        decreasing_rewards_date,
        eclip_per_second_before,
        eclip_per_second_after,
    )?;

    // core logic
    let staker = STAKER_INFO
        .load(deps.storage, sender)
        .map_err(|_| ContractError::StakerIsNotFound)?;

    let total_essence =
        helpers::v3::get_total_essence(deps.storage, block_time, seconds_per_essence)?;

    let staking_rewards = math::v3::calc_staking_rewards(
        &staker.vaults,
        decreasing_rewards_date,
        block_time,
        seconds_per_essence,
        eclip_per_second_before,
        eclip_per_second_after,
        total_essence,
    );

    let amount_to_withdraw = staker
        .vaults
        .iter()
        .fold(Uint128::zero(), |acc, vault| acc + vault.amount);

    // clear staker info
    STAKER_INFO.remove(deps.storage, sender);

    STAKE_STATE.update(deps.storage, |mut stake_state| -> StdResult<State> {
        stake_state.total_bond_amount -= amount_to_withdraw;
        stake_state.distributed_rewards_per_tier += staking_rewards.u128() as u64;

        Ok(stake_state)
    })?;

    // update essence storages
    let (a1, b1) = STAKING_ESSENCE_COMPONENTS
        .load(deps.storage, sender)
        .unwrap_or_default();

    STAKING_ESSENCE_COMPONENTS.remove(deps.storage, sender);
    TOTAL_STAKING_ESSENCE_COMPONENTS
        .update(deps.storage, |(a, b)| -> StdResult<(Uint128, Uint128)> {
            Ok((a - a1, b - b1))
        })?;

    helpers::v3::update_eclip_per_second(deps.storage, block_time, decreasing_rewards_date)?;

    let msg = BankMsg::Send {
        to_address: sender.to_string(),
        amount: coins((amount_to_withdraw + staking_rewards).u128(), staking_token),
    };

    // send essence snapshot to equinox voter
    let mut response = Response::new().add_message(msg).add_attributes(vec![
        ("action", "try_unstake"),
        ("amount", &amount_to_withdraw.to_string()),
        ("rewards", &staking_rewards.to_string()),
    ]);

    if let Some(x) = equinox_voter {
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: x.to_string(),
            msg: to_json_binary(
                &equinox_msg::voter::msg::ExecuteMsg::UpdateEssenceAllocation {
                    user_and_essence_list: helpers::get_essence_snapshot(deps.storage, sender),
                },
            )?,
            funds: vec![],
        });

        response = response.add_message(msg);
    }

    Ok(response)
}

// withdraw locked funds and claim locking rewards (over all vaults excluding bonded vault)
pub fn try_unlock(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    helpers::check_pause_state(deps.as_ref())?;

    let sender = &info.sender;
    let block_time = env.block.time.seconds();
    let config = CONFIG.load(deps.storage)?;
    let bonded_vault_creation_date = BONDED_VAULT_CREATION_DATE
        .load(deps.storage, sender)
        .unwrap_or_default();
    let Config {
        lock_schedule,
        penalty_multiplier,
        seconds_per_essence,
        eclip_per_second,
        eclip_per_second_multiplier,
        equinox_voter,
        ..
    } = config.clone();

    // get eclip_per_second values
    let (decreasing_rewards_date, eclip_per_second_before, eclip_per_second_after) =
        helpers::v3::split_eclip_per_second(
            deps.storage,
            eclip_per_second_multiplier,
            eclip_per_second,
            block_time,
        )?;

    // accumulate rewards
    helpers::v3::accumulate_rewards(
        deps.storage,
        &lock_schedule,
        block_time,
        decreasing_rewards_date,
        eclip_per_second_before,
        eclip_per_second_after,
    )?;

    // core logic
    let locker_infos = LOCKER_INFO
        .load(deps.storage, sender)
        .map_err(|_| ContractError::LockerIsNotFound)?;

    let total_essence =
        helpers::v3::get_total_essence(deps.storage, block_time, seconds_per_essence)?;

    let mut lock_tier_and_funds_and_rewards_list: Vec<(usize, Uint128, Uint128)> = vec![];
    let mut rewards_to_send = Uint128::zero();
    let mut amount_to_withdraw = Uint128::zero();
    let mut penalty_to_send = Uint128::zero();

    // update claim dates
    for locker_info in locker_infos {
        let LockerInfo {
            lock_tier,
            mut vaults,
        } = locker_info.to_owned();
        let lock_tier = lock_tier as usize;
        let (locking_period, _global_rewards_per_tier) = lock_schedule[lock_tier];

        // ignore bonded vault
        if lock_tier == TIER_4 {
            vaults.retain(|x| x.creation_date != bonded_vault_creation_date);
        }

        let locking_rewards_per_tier = math::v3::calc_locking_rewards_per_tier(
            &locker_info.vaults,
            locking_period,
            decreasing_rewards_date,
            block_time,
            seconds_per_essence,
            eclip_per_second_before,
            eclip_per_second_after,
            total_essence,
        );

        let locked_funds_to_withdraw = vaults
            .iter()
            .fold(Uint128::zero(), |acc, vault| acc + vault.amount);

        let penalty = math::v3::calc_penalty_per_tier(
            &vaults,
            locking_period,
            block_time,
            penalty_multiplier,
        );

        // penalty will be sent to DAO treasury
        penalty_to_send += penalty;
        amount_to_withdraw += locking_rewards_per_tier + locked_funds_to_withdraw - penalty;
        rewards_to_send += locking_rewards_per_tier;
        lock_tier_and_funds_and_rewards_list.push((
            lock_tier,
            locked_funds_to_withdraw,
            locking_rewards_per_tier,
        ));
    }

    // don't remove locker info is bonded vault is found
    if bonded_vault_creation_date == 0 {
        LOCKER_INFO.remove(deps.storage, sender);
    } else {
        let mut locker_infos = LOCKER_INFO
            .load(deps.storage, sender)
            .map_err(|_| ContractError::LockerIsNotFound)?;
        let tier_4_vaults = locker_infos
            .get(TIER_4)
            .ok_or(ContractError::TierIsNotFound)?
            .vaults
            .clone();
        let bonded_vault = tier_4_vaults
            .iter()
            .find(|x| x.creation_date == bonded_vault_creation_date)
            .ok_or(ContractError::VaultIsNotFound)?;

        for locker_info in locker_infos.iter_mut().take(TIER_4) {
            locker_info.vaults = vec![];
        }
        locker_infos[TIER_4].vaults = vec![bonded_vault.to_owned()];
        LOCKER_INFO.save(deps.storage, sender, &locker_infos)?;
    }

    LOCK_STATES.update(deps.storage, |mut lock_states| -> StdResult<Vec<State>> {
        for (lock_tier, funds, rewards) in lock_tier_and_funds_and_rewards_list {
            lock_states[lock_tier].total_bond_amount -= funds;
            lock_states[lock_tier].distributed_rewards_per_tier += rewards.u128() as u64;
        }

        Ok(lock_states)
    })?;

    // update essence storages
    let locking_essence_before = LOCKING_ESSENCE
        .load(deps.storage, sender)
        .unwrap_or_default();

    LOCKING_ESSENCE.remove(deps.storage, sender);
    TOTAL_LOCKING_ESSENCE.update(deps.storage, |x| -> StdResult<Uint128> {
        Ok(x - locking_essence_before)
    })?;

    helpers::v3::update_eclip_per_second(deps.storage, block_time, decreasing_rewards_date)?;

    let mut msgs: Vec<CosmosMsg> = vec![CosmosMsg::Bank(BankMsg::Send {
        to_address: sender.to_string(),
        amount: coins(amount_to_withdraw.u128(), config.staking_token.clone()),
    })];

    if !penalty_to_send.is_zero() {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: config.dao_treasury_address.to_string(),
            amount: coins(penalty_to_send.u128(), config.staking_token),
        }));
    }

    // send essence snapshot to equinox voter
    let mut response = Response::new().add_messages(msgs).add_attributes(vec![
        ("action", "try_unlock"),
        ("amount_to_withdraw", &amount_to_withdraw.to_string()),
        ("penalty", &penalty_to_send.to_string()),
    ]);

    if let Some(x) = equinox_voter {
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: x.to_string(),
            msg: to_json_binary(
                &equinox_msg::voter::msg::ExecuteMsg::UpdateEssenceAllocation {
                    user_and_essence_list: helpers::get_essence_snapshot(deps.storage, sender),
                },
            )?,
            funds: vec![],
        });

        response = response.add_message(msg);
    }

    Ok(response)
}

pub fn try_relock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    vault_creation_date: u64,
    from_tier: u64,
    to_tier: u64,
) -> Result<Response, ContractError> {
    helpers::check_pause_state(deps.as_ref())?;

    let sender = &info.sender;
    let block_time = env.block.time.seconds();
    let Config {
        lock_schedule,
        seconds_per_essence,
        eclip_per_second,
        eclip_per_second_multiplier,
        equinox_voter,
        ..
    } = CONFIG.load(deps.storage)?;

    if to_tier <= from_tier {
        Err(ContractError::DecreasingLockTier)?;
    }

    if to_tier >= lock_schedule.len() as u64 {
        Err(ContractError::LockTierIsOutOfRange)?;
    }

    // get eclip_per_second values
    let (decreasing_rewards_date, eclip_per_second_before, eclip_per_second_after) =
        helpers::v3::split_eclip_per_second(
            deps.storage,
            eclip_per_second_multiplier,
            eclip_per_second,
            block_time,
        )?;

    // accumulate rewards
    helpers::v3::accumulate_rewards(
        deps.storage,
        &lock_schedule,
        block_time,
        decreasing_rewards_date,
        eclip_per_second_before,
        eclip_per_second_after,
    )?;

    // core logic
    let total_essence =
        helpers::v3::get_total_essence(deps.storage, block_time, seconds_per_essence)?;

    let mut vault_amount = Uint128::zero();

    LOCKER_INFO.update(
        deps.storage,
        sender,
        |locker_info| -> StdResult<Vec<LockerInfo>> {
            let mut locker_infos = locker_info.ok_or(ContractError::LockerIsNotFound)?;

            while to_tier >= locker_infos.len() as u64 {
                locker_infos.push(LockerInfo {
                    lock_tier: locker_infos.len() as u64,
                    vaults: vec![],
                })
            }

            let from_tier_vaults = &locker_infos[from_tier as usize].vaults;
            let mut to_tier_vaults = locker_infos[to_tier as usize].vaults.clone();
            let mut from_tier_vaults_new: Vec<Vault> = vec![];

            // don't allow multiple vaults with same creation date
            if from_tier_vaults
                .iter()
                .any(|x| x.creation_date == block_time)
                || to_tier_vaults.iter().any(|x| x.creation_date == block_time)
            {
                Err(ContractError::MultipleVaultsWithSameCreationDate)?;
            }

            for vault in from_tier_vaults {
                if vault.creation_date != vault_creation_date {
                    from_tier_vaults_new.push(vault.to_owned());
                    continue;
                }

                vault_amount = vault.amount;

                // accumulate rewards and update claim date
                let (locking_period, _global_rewards_per_tier) = lock_schedule[from_tier as usize];

                let locking_essence_per_vault = math::v3::calc_locking_essence_per_vault(
                    vault.amount,
                    locking_period,
                    seconds_per_essence,
                );

                let accumulated_rewards = math::v3::calc_locking_rewards_per_vault(
                    vault.accumulated_rewards,
                    locking_essence_per_vault,
                    vault.claim_date,
                    decreasing_rewards_date,
                    block_time,
                    eclip_per_second_before,
                    eclip_per_second_after,
                    total_essence,
                );

                to_tier_vaults.push(Vault {
                    claim_date: block_time,
                    accumulated_rewards,
                    ..vault.to_owned()
                });
            }

            // sort vaults by creation_date
            to_tier_vaults.sort_by_key(|vault| vault.creation_date);

            locker_infos[from_tier as usize].vaults = from_tier_vaults_new;
            locker_infos[to_tier as usize].vaults = to_tier_vaults;

            Ok(locker_infos)
        },
    )?;

    LOCK_STATES.update(deps.storage, |mut lock_states| -> StdResult<Vec<State>> {
        lock_states[from_tier as usize].total_bond_amount -= vault_amount;
        lock_states[to_tier as usize].total_bond_amount += vault_amount;

        Ok(lock_states)
    })?;

    // update essence storages
    let locking_essence_before = LOCKING_ESSENCE
        .load(deps.storage, sender)
        .unwrap_or_default();
    let locking_essence_after = math::v3::calc_locking_essence(
        &LOCKER_INFO.load(deps.storage, sender)?,
        &lock_schedule,
        seconds_per_essence,
    );

    LOCKING_ESSENCE.save(deps.storage, sender, &locking_essence_after)?;
    TOTAL_LOCKING_ESSENCE.update(deps.storage, |x| -> StdResult<Uint128> {
        Ok(x + locking_essence_after - locking_essence_before)
    })?;

    helpers::v3::update_eclip_per_second(deps.storage, block_time, decreasing_rewards_date)?;

    // send essence snapshot to equinox voter
    let mut response = Response::new().add_attributes(vec![("action", "try_relock")]);

    if let Some(x) = equinox_voter {
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: x.to_string(),
            msg: to_json_binary(
                &equinox_msg::voter::msg::ExecuteMsg::UpdateEssenceAllocation {
                    user_and_essence_list: helpers::get_essence_snapshot(deps.storage, sender),
                },
            )?,
            funds: vec![],
        });

        response = response.add_message(msg);
    }

    Ok(response)
}

// withdraw locked funds and claim locking rewards for single vault
pub fn try_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    vault_creation_date: u64,
) -> Result<Response, ContractError> {
    helpers::check_pause_state(deps.as_ref())?;

    let sender = &info.sender;
    let block_time = env.block.time.seconds();
    let config = CONFIG.load(deps.storage)?;
    let bonded_vault_creation_date = BONDED_VAULT_CREATION_DATE
        .load(deps.storage, sender)
        .unwrap_or_default();
    let Config {
        lock_schedule,
        penalty_multiplier,
        seconds_per_essence,
        eclip_per_second,
        eclip_per_second_multiplier,
        equinox_voter,
        ..
    } = config.clone();

    // get eclip_per_second values
    let (decreasing_rewards_date, eclip_per_second_before, eclip_per_second_after) =
        helpers::v3::split_eclip_per_second(
            deps.storage,
            eclip_per_second_multiplier,
            eclip_per_second,
            block_time,
        )?;

    // accumulate rewards
    helpers::v3::accumulate_rewards(
        deps.storage,
        &lock_schedule,
        block_time,
        decreasing_rewards_date,
        eclip_per_second_before,
        eclip_per_second_after,
    )?;

    // core logic
    let total_essence =
        helpers::v3::get_total_essence(deps.storage, block_time, seconds_per_essence)?;

    let mut lock_tier_and_funds_and_rewards_list: Vec<(u64, Uint128, Uint128)> = vec![];
    let mut amount_to_withdraw = Uint128::zero();
    let mut penalty_to_send = Uint128::zero();
    let mut locker_infos_new: Vec<LockerInfo> = vec![];

    let locker_infos = LOCKER_INFO
        .load(deps.storage, sender)
        .map_err(|_| ContractError::LockerIsNotFound)?;

    // update claim dates
    for locker_info in locker_infos {
        let LockerInfo { lock_tier, vaults } = locker_info.to_owned();
        let current_vault = vaults
            .iter()
            .find(|x| x.creation_date == vault_creation_date);
        // skip if target vault isn't found
        if current_vault.is_none() {
            locker_infos_new.push(locker_info);
            continue;
        }

        let current_vault = current_vault.unwrap();
        let (locking_period, _global_rewards_per_tier) = lock_schedule[lock_tier as usize];

        // bonded vault can't be withdrawn
        if (lock_tier as usize) == TIER_4
            && current_vault.creation_date == bonded_vault_creation_date
        {
            Err(ContractError::BondedVault)?;
        }

        let locking_essence_per_vault = math::v3::calc_locking_essence_per_vault(
            current_vault.amount,
            locking_period,
            seconds_per_essence,
        );

        let locking_rewards_per_vault = math::v3::calc_locking_rewards_per_vault(
            current_vault.accumulated_rewards,
            locking_essence_per_vault,
            current_vault.claim_date,
            decreasing_rewards_date,
            block_time,
            eclip_per_second_before,
            eclip_per_second_after,
            total_essence,
        );

        let locked_funds_to_withdraw = current_vault.amount;

        let penalty = math::v3::calc_penalty_per_tier(
            &[current_vault.to_owned()],
            locking_period,
            block_time,
            penalty_multiplier,
        );

        // penalty will be sent to DAO treasury
        penalty_to_send = penalty;
        amount_to_withdraw = locking_rewards_per_vault + locked_funds_to_withdraw - penalty;
        lock_tier_and_funds_and_rewards_list.push((
            lock_tier,
            locked_funds_to_withdraw,
            locking_rewards_per_vault,
        ));

        let vaults_new: Vec<Vault> = vaults
            .iter()
            .cloned()
            .filter(|x| x.creation_date != vault_creation_date)
            .collect();

        locker_infos_new.push(LockerInfo {
            lock_tier,
            vaults: vaults_new,
        });
    }

    // update locker info
    let vaults_amount = locker_infos_new
        .iter()
        .fold(0, |acc, cur| acc + cur.vaults.len());

    if vaults_amount == 0 {
        LOCKER_INFO.remove(deps.storage, sender);
    } else {
        LOCKER_INFO.save(deps.storage, sender, &locker_infos_new)?;
    }

    LOCK_STATES.update(deps.storage, |mut lock_states| -> StdResult<Vec<State>> {
        for (lock_tier, funds, rewards) in lock_tier_and_funds_and_rewards_list {
            lock_states[lock_tier as usize].total_bond_amount -= funds;
            lock_states[lock_tier as usize].distributed_rewards_per_tier += rewards.u128() as u64;
        }

        Ok(lock_states)
    })?;

    // update essence storages
    let locking_essence_before = LOCKING_ESSENCE
        .load(deps.storage, sender)
        .unwrap_or_default();
    let locking_essence_after =
        math::v3::calc_locking_essence(&locker_infos_new, &lock_schedule, seconds_per_essence);

    TOTAL_LOCKING_ESSENCE.update(deps.storage, |x| -> StdResult<Uint128> {
        Ok(x + locking_essence_after - locking_essence_before)
    })?;

    if vaults_amount == 0 {
        LOCKING_ESSENCE.remove(deps.storage, sender);
    } else {
        LOCKING_ESSENCE.save(deps.storage, sender, &locking_essence_after)?;
    }

    helpers::v3::update_eclip_per_second(deps.storage, block_time, decreasing_rewards_date)?;

    let mut msgs: Vec<CosmosMsg> = vec![CosmosMsg::Bank(BankMsg::Send {
        to_address: sender.to_string(),
        amount: coins(amount_to_withdraw.u128(), config.staking_token.clone()),
    })];

    if !penalty_to_send.is_zero() {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: config.dao_treasury_address.to_string(),
            amount: coins(penalty_to_send.u128(), config.staking_token),
        }));
    }

    // send essence snapshot to equinox voter
    let mut response = Response::new().add_messages(msgs).add_attributes(vec![
        ("action", "try_withdraw"),
        ("amount_to_withdraw", &amount_to_withdraw.to_string()),
        ("penalty", &penalty_to_send.to_string()),
    ]);

    if let Some(x) = equinox_voter {
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: x.to_string(),
            msg: to_json_binary(
                &equinox_msg::voter::msg::ExecuteMsg::UpdateEssenceAllocation {
                    user_and_essence_list: helpers::get_essence_snapshot(deps.storage, sender),
                },
            )?,
            funds: vec![],
        });

        response = response.add_message(msg);
    }

    Ok(response)
}

// block actions with bonded vault
pub fn try_bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut vault_creation_date_list: Vec<u64>,
) -> Result<Response, ContractError> {
    helpers::check_pause_state(deps.as_ref())?;

    // TODO: remove this block in v3
    if env.block.chain_id == "neutron-1" {
        Err(ContractError::WrongMessageType)?;
    }

    let sender = &info.sender;
    let block_time = env.block.time.seconds();
    let Config {
        beclip_minter,
        beclip_address,
        lock_schedule,
        seconds_per_essence,
        eclip_per_second,
        eclip_per_second_multiplier,
        equinox_voter,
        ..
    } = CONFIG.load(deps.storage)?;
    let beclip_minter = unwrap_field(beclip_minter, "beclip_minter")?;
    let beclip_address = unwrap_field(beclip_address, "beclip_address")?;

    // get eclip_per_second values
    let (decreasing_rewards_date, eclip_per_second_before, eclip_per_second_after) =
        helpers::v3::split_eclip_per_second(
            deps.storage,
            eclip_per_second_multiplier,
            eclip_per_second,
            block_time,
        )?;

    // accumulate rewards
    helpers::v3::accumulate_rewards(
        deps.storage,
        &lock_schedule,
        block_time,
        decreasing_rewards_date,
        eclip_per_second_before,
        eclip_per_second_after,
    )?;

    // core logic
    let total_essence =
        helpers::v3::get_total_essence(deps.storage, block_time, seconds_per_essence)?;

    let mut locker_infos = LOCKER_INFO
        .load(deps.storage, sender)
        .map_err(|_| ContractError::LockerIsNotFound)?;

    let tier_4_vaults = &locker_infos
        .get(TIER_4)
        .ok_or(ContractError::TierIsNotFound)?
        .vaults
        .clone();

    let amount_to_mint = tier_4_vaults
        .iter()
        .filter(|x| vault_creation_date_list.contains(&x.creation_date))
        .fold(Uint128::zero(), |acc, cur| acc + cur.amount);

    // split vaults
    let mut new_tier_4_vaults = vec![];
    let mut old_tier_4_vaults = vec![];

    if let Ok(x) = BONDED_VAULT_CREATION_DATE.load(deps.storage, sender) {
        vault_creation_date_list.push(x);
    }

    for vault in tier_4_vaults {
        if vault_creation_date_list.contains(&vault.creation_date) {
            new_tier_4_vaults.push(vault.to_owned());
        } else {
            old_tier_4_vaults.push(vault.to_owned());
        }
    }

    let bonded_vault = math::v3::calc_bonded_vault(
        &new_tier_4_vaults,
        &lock_schedule,
        decreasing_rewards_date,
        block_time,
        seconds_per_essence,
        eclip_per_second_before,
        eclip_per_second_after,
        total_essence,
    );

    old_tier_4_vaults.push(bonded_vault);

    BONDED_VAULT_CREATION_DATE.save(deps.storage, sender, &block_time)?;

    BECLIP_SUPPLY.update(deps.storage, |x| -> StdResult<Uint128> {
        Ok(x + amount_to_mint)
    })?;

    // update locker info
    locker_infos[TIER_4].vaults = old_tier_4_vaults;
    LOCKER_INFO.save(deps.storage, sender, &locker_infos)?;

    // update essence storages
    let locking_essence_before = LOCKING_ESSENCE
        .load(deps.storage, sender)
        .unwrap_or_default();
    let locking_essence_after =
        math::v3::calc_locking_essence(&locker_infos, &lock_schedule, seconds_per_essence);

    TOTAL_LOCKING_ESSENCE.update(deps.storage, |x| -> StdResult<Uint128> {
        Ok(x + locking_essence_after - locking_essence_before)
    })?;

    LOCKING_ESSENCE.save(deps.storage, sender, &locking_essence_after)?;

    helpers::v3::update_eclip_per_second(deps.storage, block_time, decreasing_rewards_date)?;

    // mint beclip to sender
    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: beclip_minter.to_string(),
        msg: to_json_binary(&eclipse_base::minter::msg::ExecuteMsg::Mint {
            token: TokenUnverified::new_cw20(beclip_address.as_str()),
            amount: amount_to_mint,
            recipient: sender.to_string(),
        })?,
        funds: vec![],
    });

    // send essence snapshot to equinox voter
    let mut response = Response::new().add_message(msg).add_attributes(vec![
        ("action", "try_bond"),
        ("amount", &amount_to_mint.to_string()),
    ]);

    if let Some(x) = equinox_voter {
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: x.to_string(),
            msg: to_json_binary(
                &equinox_msg::voter::msg::ExecuteMsg::UpdateEssenceAllocation {
                    user_and_essence_list: helpers::get_essence_snapshot(deps.storage, sender),
                },
            )?,
            funds: vec![],
        });

        response = response.add_message(msg);
    }

    Ok(response)
}

/// convert eclip to beclip directly
pub fn try_bond_for(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address_and_amount_list: Vec<(String, Uint128)>,
) -> Result<Response, ContractError> {
    helpers::check_pause_state(deps.as_ref())?;
    let (sender_address, asset_amount, asset_info) = check_funds(
        deps.as_ref(),
        &info,
        FundsType::Single {
            sender: None,
            amount: None,
        },
    )?;

    // TODO: remove this block in v3
    if env.block.chain_id == "neutron-1" {
        Err(ContractError::WrongMessageType)?;
    }

    let block_time = env.block.time.seconds();
    let Config {
        staking_token,
        beclip_minter,
        beclip_address,
        beclip_whitelist,
        lock_schedule,
        seconds_per_essence,
        eclip_per_second,
        eclip_per_second_multiplier,
        equinox_voter,
        ..
    } = CONFIG.load(deps.storage)?;
    let beclip_minter = unwrap_field(beclip_minter, "beclip_minter")?;
    let beclip_address = unwrap_field(beclip_address, "beclip_address")?;

    if !beclip_whitelist.contains(&sender_address) {
        Err(ContractError::Unauthorized)?;
    }

    // check amount sum
    let amount_sum = address_and_amount_list
        .iter()
        .fold(Uint128::zero(), |acc, (_, amount)| acc + amount);
    if amount_sum != asset_amount {
        Err(ContractError::ImproperAmountSum)?;
    }

    // don't allow zero amounts
    if address_and_amount_list
        .iter()
        .any(|(_, amount)| amount.is_zero())
    {
        Err(ContractError::ZeroAmount)?;
    }

    if asset_info.try_get_native()? != staking_token {
        Err(ContractError::WrongToken)?;
    }

    // get eclip_per_second values
    let (decreasing_rewards_date, eclip_per_second_before, eclip_per_second_after) =
        helpers::v3::split_eclip_per_second(
            deps.storage,
            eclip_per_second_multiplier,
            eclip_per_second,
            block_time,
        )?;

    // accumulate rewards
    helpers::v3::accumulate_rewards(
        deps.storage,
        &lock_schedule,
        block_time,
        decreasing_rewards_date,
        eclip_per_second_before,
        eclip_per_second_after,
    )?;

    // core logic
    let total_essence =
        helpers::v3::get_total_essence(deps.storage, block_time, seconds_per_essence)?;

    for (user, beclip_to_mint) in &address_and_amount_list {
        let user = deps.api.addr_validate(user)?;
        let mut locker_infos = LOCKER_INFO.load(deps.storage, &user).unwrap_or_default();

        while TIER_4 >= locker_infos.len() {
            locker_infos.push(LockerInfo {
                lock_tier: locker_infos.len() as u64,
                vaults: vec![],
            })
        }

        let mut tier_4_vaults = locker_infos[TIER_4].vaults.clone();
        let new_tier_4_vault = Vault {
            amount: beclip_to_mint.to_owned(),
            accumulated_rewards: Uint128::zero(),
            creation_date: block_time,
            claim_date: block_time,
        };

        match BONDED_VAULT_CREATION_DATE.load(deps.storage, &user) {
            Ok(x) => {
                // update bonded vault
                tier_4_vaults = tier_4_vaults
                    .into_iter()
                    .map(|y| {
                        if y.creation_date != x {
                            y
                        } else {
                            math::v3::calc_bonded_vault(
                                &[y, new_tier_4_vault.clone()],
                                &lock_schedule,
                                decreasing_rewards_date,
                                block_time,
                                seconds_per_essence,
                                eclip_per_second_before,
                                eclip_per_second_after,
                                total_essence,
                            )
                        }
                    })
                    .collect();
            }
            _ => {
                // add new bonded vault (vault limit can be ignored to avoid failing txs)
                // block time can be incremented to avoid failing txs as well
                if tier_4_vaults.last().is_some()
                    && tier_4_vaults.last().unwrap().creation_date == block_time
                {
                    tier_4_vaults.push(Vault {
                        creation_date: block_time + 1,
                        ..new_tier_4_vault.clone()
                    });
                } else {
                    tier_4_vaults.push(new_tier_4_vault.clone());
                }
            }
        }

        BONDED_VAULT_CREATION_DATE.save(deps.storage, &user, &block_time)?;

        locker_infos[TIER_4].vaults = tier_4_vaults;
        LOCKER_INFO.save(deps.storage, &user, &locker_infos)?;

        // update locking essence
        let locking_essence_before = LOCKING_ESSENCE
            .load(deps.storage, &user)
            .unwrap_or_default();
        let locking_essence_after =
            math::v3::calc_locking_essence(&locker_infos, &lock_schedule, seconds_per_essence);

        LOCKING_ESSENCE.save(deps.storage, &user, &locking_essence_after)?;
        TOTAL_LOCKING_ESSENCE.update(deps.storage, |x| -> StdResult<Uint128> {
            Ok(x + locking_essence_after - locking_essence_before)
        })?;
    }

    BECLIP_SUPPLY.update(deps.storage, |x| -> StdResult<Uint128> {
        Ok(x + asset_amount)
    })?;

    LOCK_STATES.update(deps.storage, |mut lock_states| -> StdResult<Vec<State>> {
        lock_states[TIER_4].total_bond_amount += asset_amount;

        Ok(lock_states)
    })?;

    helpers::v3::update_eclip_per_second(deps.storage, block_time, decreasing_rewards_date)?;

    // mint beclip to specified addresses
    let msg_list = address_and_amount_list
        .iter()
        .cloned()
        .map(|(recipient, amount_to_mint)| {
            Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: beclip_minter.to_string(),
                msg: to_json_binary(&eclipse_base::minter::msg::ExecuteMsg::Mint {
                    token: TokenUnverified::new_cw20(beclip_address.as_str()),
                    amount: amount_to_mint,
                    recipient,
                })?,
                funds: vec![],
            }))
        })
        .collect::<StdResult<Vec<CosmosMsg>>>()?;

    // send essence snapshot to equinox voter
    let mut response = Response::new().add_messages(msg_list).add_attributes(vec![
        ("action", "try_bond_for"),
        ("amount", &asset_amount.to_string()),
    ]);

    if let Some(x) = equinox_voter {
        let mut user_and_essence_list: Vec<(String, EssenceInfo)> = vec![];

        for (sender, _) in address_and_amount_list {
            user_and_essence_list = [
                user_and_essence_list,
                helpers::get_essence_snapshot(deps.storage, &Addr::unchecked(sender)),
            ]
            .concat();
        }

        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: x.to_string(),
            msg: to_json_binary(
                &equinox_msg::voter::msg::ExecuteMsg::UpdateEssenceAllocation {
                    user_and_essence_list,
                },
            )?,
            funds: vec![],
        });

        response = response.add_message(msg);
    }

    Ok(response)
}

/// burns bECLIP and creates tier 4 vault
pub fn try_unbond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: Option<String>,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    helpers::check_pause_state(deps.as_ref())?;
    let (sender_address, asset_amount, asset_info) =
        check_funds(deps.as_ref(), &info, FundsType::Single { sender, amount })?;

    // TODO: remove this block in v3
    if env.block.chain_id == "neutron-1" {
        Err(ContractError::WrongMessageType)?;
    }

    let block_time = env.block.time.seconds();
    let bonded_vault_creation_date =
        BONDED_VAULT_CREATION_DATE.load(deps.storage, &sender_address)?;
    let Config {
        beclip_address,
        beclip_minter,
        lock_schedule,
        seconds_per_essence,
        eclip_per_second,
        eclip_per_second_multiplier,
        equinox_voter,
        ..
    } = CONFIG.load(deps.storage)?;
    let beclip_minter = unwrap_field(beclip_minter, "beclip_minter")?;
    let beclip_address = unwrap_field(beclip_address, "beclip_address")?;

    if asset_amount.is_zero() {
        Err(ContractError::ZeroCoinsAmount)?;
    }

    if asset_info.try_get_cw20()? != beclip_address {
        Err(ContractError::WrongToken)?;
    }

    // get eclip_per_second values
    let (decreasing_rewards_date, eclip_per_second_before, eclip_per_second_after) =
        helpers::v3::split_eclip_per_second(
            deps.storage,
            eclip_per_second_multiplier,
            eclip_per_second,
            block_time,
        )?;

    // accumulate rewards
    helpers::v3::accumulate_rewards(
        deps.storage,
        &lock_schedule,
        block_time,
        decreasing_rewards_date,
        eclip_per_second_before,
        eclip_per_second_after,
    )?;

    // core logic
    let total_essence =
        helpers::v3::get_total_essence(deps.storage, block_time, seconds_per_essence)?;

    let mut locker_infos = LOCKER_INFO
        .load(deps.storage, &sender_address)
        .map_err(|_| ContractError::LockerIsNotFound)?;
    let tier_4_vaults = locker_infos
        .get(TIER_4)
        .ok_or(ContractError::TierIsNotFound)?
        .vaults
        .clone();

    // don't allow multiple vaults with same creation date
    if tier_4_vaults.iter().any(|x| x.creation_date == block_time) {
        Err(ContractError::MultipleVaultsWithSameCreationDate)?;
    }

    let mut new_tier_4_vaults: Vec<Vault> = vec![];

    for vault in tier_4_vaults {
        // move regular tier 4 vaults unchanged
        if bonded_vault_creation_date != vault.creation_date {
            new_tier_4_vaults.push(vault);
            continue;
        }

        // check total bonded amount
        if asset_amount > vault.amount {
            Err(ContractError::ExceedingBondedAmount)?;
        }

        let (bonded_vault, tier_4_vault) = math::v3::split_bonded_vault(
            &vault,
            asset_amount,
            &lock_schedule,
            decreasing_rewards_date,
            block_time,
            seconds_per_essence,
            eclip_per_second_before,
            eclip_per_second_after,
            total_essence,
        );

        match bonded_vault {
            Some(x) => {
                new_tier_4_vaults.push(x);
            }
            None => {
                // clear if vault was consumed completely
                BONDED_VAULT_CREATION_DATE.remove(deps.storage, &sender_address);
            }
        }

        new_tier_4_vaults.push(tier_4_vault);
    }

    // sort from oldest to newest
    new_tier_4_vaults.sort_unstable_by_key(|x| x.creation_date);

    // don't allow too much vaults
    helpers::v3::check_vaults_amount(deps.storage, &sender_address)?;

    // update locker info storage
    locker_infos[TIER_4].vaults = new_tier_4_vaults;
    LOCKER_INFO.save(deps.storage, &sender_address, &locker_infos)?;

    BECLIP_SUPPLY.update(deps.storage, |x| -> StdResult<Uint128> {
        Ok(x - asset_amount)
    })?;

    // update essence storages
    let locking_essence_before = LOCKING_ESSENCE
        .load(deps.storage, &sender_address)
        .unwrap_or_default();
    let locking_essence_after = math::v3::calc_locking_essence(
        &LOCKER_INFO.load(deps.storage, &sender_address)?,
        &lock_schedule,
        seconds_per_essence,
    );

    LOCKING_ESSENCE.save(deps.storage, &sender_address, &locking_essence_after)?;
    TOTAL_LOCKING_ESSENCE.update(deps.storage, |x| -> StdResult<Uint128> {
        Ok(x + locking_essence_after - locking_essence_before)
    })?;

    helpers::v3::update_eclip_per_second(deps.storage, block_time, decreasing_rewards_date)?;

    // burn beclip
    let msg = add_funds_to_exec_msg(
        &WasmMsg::Execute {
            contract_addr: beclip_minter.to_string(),
            msg: to_json_binary(&eclipse_base::minter::msg::ExecuteMsg::Burn {})?,
            funds: vec![],
        },
        &[(asset_amount, asset_info)],
    )?;

    // send essence snapshot to equinox voter
    let mut response = Response::new().add_message(msg).add_attributes(vec![
        ("action", "try_unbond"),
        ("amount", &asset_amount.to_string()),
    ]);

    if let Some(x) = equinox_voter {
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: x.to_string(),
            msg: to_json_binary(
                &equinox_msg::voter::msg::ExecuteMsg::UpdateEssenceAllocation {
                    user_and_essence_list: helpers::get_essence_snapshot(
                        deps.storage,
                        &sender_address,
                    ),
                },
            )?,
            funds: vec![],
        });

        response = response.add_message(msg);
    }

    Ok(response)
}

// claim rewards for locking
pub fn try_claim(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    helpers::check_pause_state(deps.as_ref())?;

    let config = CONFIG.load(deps.storage)?;
    let sender = &info.sender;
    let block_time = env.block.time.seconds();
    let Config {
        lock_schedule,
        seconds_per_essence,
        eclip_per_second,
        eclip_per_second_multiplier,
        ..
    } = CONFIG.load(deps.storage)?;

    // get eclip_per_second values
    let (decreasing_rewards_date, eclip_per_second_before, eclip_per_second_after) =
        helpers::v3::split_eclip_per_second(
            deps.storage,
            eclip_per_second_multiplier,
            eclip_per_second,
            block_time,
        )?;

    // accumulate rewards
    helpers::v3::accumulate_rewards(
        deps.storage,
        &lock_schedule,
        block_time,
        decreasing_rewards_date,
        eclip_per_second_before,
        eclip_per_second_after,
    )?;

    // core logic
    let total_essence =
        helpers::v3::get_total_essence(deps.storage, block_time, seconds_per_essence)?;

    let mut lock_tier_and_rewards_list: Vec<(u64, Uint128)> = vec![];
    let mut amount_to_withdraw = Uint128::zero();

    // try claim staking rewards
    if let Ok(mut staker) = STAKER_INFO.load(deps.storage, sender) {
        amount_to_withdraw += math::v3::calc_staking_rewards(
            &staker.vaults,
            decreasing_rewards_date,
            block_time,
            seconds_per_essence,
            eclip_per_second_before,
            eclip_per_second_after,
            total_essence,
        );

        staker.vaults = staker
            .vaults
            .into_iter()
            .map(|mut vault| {
                vault.claim_date = block_time;
                vault.accumulated_rewards = Uint128::zero();

                vault
            })
            .collect();

        STAKER_INFO.save(deps.storage, sender, &staker)?;
    };

    STAKE_STATE.update(deps.storage, |mut stake_state| -> StdResult<State> {
        stake_state.distributed_rewards_per_tier += amount_to_withdraw.u128() as u64;
        Ok(stake_state)
    })?;

    // try claim locking rewards
    if let Ok(mut locker_infos) = LOCKER_INFO.load(deps.storage, sender) {
        // update claim dates
        locker_infos = locker_infos
            .iter()
            .map(|locker_info| {
                let LockerInfo { lock_tier, .. } = locker_info.to_owned();
                let (locking_period, _global_rewards_per_tier) = lock_schedule[lock_tier as usize];

                let locking_rewards_per_tier = math::v3::calc_locking_rewards_per_tier(
                    &locker_info.vaults,
                    locking_period,
                    decreasing_rewards_date,
                    block_time,
                    seconds_per_essence,
                    eclip_per_second_before,
                    eclip_per_second_after,
                    total_essence,
                );

                amount_to_withdraw += locking_rewards_per_tier;
                lock_tier_and_rewards_list.push((lock_tier, locking_rewards_per_tier));

                LockerInfo {
                    vaults: locker_info
                        .vaults
                        .iter()
                        .map(|vault| Vault {
                            claim_date: block_time,
                            accumulated_rewards: Uint128::zero(),
                            ..vault.to_owned()
                        })
                        .collect(),
                    ..locker_info.to_owned()
                }
            })
            .collect();

        LOCKER_INFO.save(deps.storage, sender, &locker_infos)?;
    }

    LOCK_STATES.update(deps.storage, |mut lock_states| -> StdResult<Vec<State>> {
        for (lock_tier, amount) in lock_tier_and_rewards_list {
            lock_states[lock_tier as usize].distributed_rewards_per_tier += amount.u128() as u64;
        }

        Ok(lock_states)
    })?;

    helpers::v3::update_eclip_per_second(deps.storage, block_time, decreasing_rewards_date)?;

    let msg = BankMsg::Send {
        to_address: sender.to_string(),
        amount: coins(amount_to_withdraw.u128(), config.staking_token),
    };

    Ok(Response::new().add_message(msg).add_attributes(vec![
        ("action", "try_claim"),
        ("amount", &amount_to_withdraw.to_string()),
    ]))
}

pub fn try_aggregate_vaults(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    tier: Option<u64>,
    vault_creation_date_list: Vec<u64>,
) -> Result<Response, ContractError> {
    helpers::check_pause_state(deps.as_ref())?;

    let sender = &info.sender;
    let block_time = env.block.time.seconds();
    let bonded_vault_creation_date = BONDED_VAULT_CREATION_DATE
        .load(deps.storage, sender)
        .unwrap_or_default();
    let Config {
        seconds_per_essence,
        eclip_per_second,
        lock_schedule,
        eclip_per_second_multiplier,
        equinox_voter,
        ..
    } = CONFIG.load(deps.storage)?;

    // get eclip_per_second values
    let (decreasing_rewards_date, eclip_per_second_before, eclip_per_second_after) =
        helpers::v3::split_eclip_per_second(
            deps.storage,
            eclip_per_second_multiplier,
            eclip_per_second,
            block_time,
        )?;

    // don't accumulate rewards to make this action cheap and attractive for users

    let total_essence =
        helpers::v3::get_total_essence(deps.storage, block_time, seconds_per_essence)?;

    let mut vaults_new: Vec<Vault> = vec![];
    let mut vaults_target: Vec<Vault> = vec![];

    match tier {
        Some(lock_tier) => {
            let lock_tier = lock_tier as usize;
            let max_lock_tier = CONFIG.load(deps.storage)?.lock_schedule.len();

            if lock_tier >= max_lock_tier {
                Err(ContractError::LockTierIsOutOfRange)?;
            }

            let (locking_period, _global_rewards_per_tier) = lock_schedule[lock_tier];
            let locker_infos = LOCKER_INFO.load(deps.storage, sender)?;

            let locker_infos_new = locker_infos
                .into_iter()
                .map(|mut x| -> StdResult<LockerInfo> {
                    if x.lock_tier != lock_tier as u64 {
                        Ok(x)
                    } else {
                        // split vaults
                        for vault in x.vaults {
                            if vault_creation_date_list.contains(&vault.creation_date) {
                                // bonded vault can't be merged with regular vaults
                                if lock_tier == TIER_4
                                    && vault.creation_date == bonded_vault_creation_date
                                {
                                    Err(ContractError::BondedVault)?;
                                }

                                vaults_target.push(vault);
                            } else {
                                vaults_new.push(vault);
                            }
                        }

                        let vault_aggregated = math::v3::calc_aggregated_vault(
                            &vaults_target,
                            locking_period,
                            decreasing_rewards_date,
                            block_time,
                            seconds_per_essence,
                            eclip_per_second_before,
                            eclip_per_second_after,
                            total_essence,
                        );

                        // don't allow multiple vaults with same creation date
                        if vaults_new
                            .iter()
                            .any(|x| x.creation_date == vault_aggregated.creation_date)
                        {
                            Err(ContractError::MultipleVaultsWithSameCreationDate)?;
                        }

                        vaults_new.push(vault_aggregated);
                        // sort vaults by creation_date
                        vaults_new.sort_by_key(|vault| vault.creation_date);

                        x.vaults = vaults_new.clone();

                        Ok(x)
                    }
                })
                .collect::<StdResult<Vec<LockerInfo>>>()?;

            LOCKER_INFO.save(deps.storage, sender, &locker_infos_new)?;

            // update essence storages
            let locking_essence_before = LOCKING_ESSENCE
                .load(deps.storage, sender)
                .unwrap_or_default();
            let locking_essence_after = math::v3::calc_locking_essence(
                &locker_infos_new,
                &lock_schedule,
                seconds_per_essence,
            );

            LOCKING_ESSENCE.save(deps.storage, sender, &locking_essence_after)?;
            TOTAL_LOCKING_ESSENCE.update(deps.storage, |x| -> StdResult<Uint128> {
                Ok(x + locking_essence_after - locking_essence_before)
            })?;
        }
        None => {
            let mut staker_info = STAKER_INFO.load(deps.storage, sender)?;

            // split vaults
            for vault in staker_info.vaults {
                if vault_creation_date_list.contains(&vault.creation_date) {
                    vaults_target.push(vault);
                } else {
                    vaults_new.push(vault);
                }
            }

            let vault_aggregated = math::v3::calc_aggregated_vault(
                &vaults_target,
                0,
                decreasing_rewards_date,
                block_time,
                seconds_per_essence,
                eclip_per_second_before,
                eclip_per_second_after,
                total_essence,
            );

            // don't allow multiple vaults with same creation date
            if vaults_new
                .iter()
                .any(|x| x.creation_date == vault_aggregated.creation_date)
            {
                Err(ContractError::MultipleVaultsWithSameCreationDate)?;
            }

            vaults_new.push(vault_aggregated);
            // sort vaults by creation_date
            vaults_new.sort_by_key(|vault| vault.creation_date);

            staker_info.vaults = vaults_new;

            STAKER_INFO.save(deps.storage, sender, &staker_info)?;

            // update essence storages
            let staking_vaults = &staker_info.vaults;
            let (a1, b1) = STAKING_ESSENCE_COMPONENTS
                .load(deps.storage, sender)
                .unwrap_or_default();
            let (a2, b2) = math::v3::calc_components_from_staking_vaults(staking_vaults);

            STAKING_ESSENCE_COMPONENTS.save(deps.storage, sender, &(a2, b2))?;
            TOTAL_STAKING_ESSENCE_COMPONENTS
                .update(deps.storage, |(a, b)| -> StdResult<(Uint128, Uint128)> {
                    Ok((a + a2 - a1, b + b2 - b1))
                })?;
        }
    }

    helpers::v3::update_eclip_per_second(deps.storage, block_time, decreasing_rewards_date)?;

    // send essence snapshot to equinox voter
    let mut response = Response::new().add_attributes(vec![("action", "try_aggregate_vaults")]);

    if let Some(x) = equinox_voter {
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: x.to_string(),
            msg: to_json_binary(
                &equinox_msg::voter::msg::ExecuteMsg::UpdateEssenceAllocation {
                    user_and_essence_list: helpers::get_essence_snapshot(deps.storage, sender),
                },
            )?,
            funds: vec![],
        });

        response = response.add_message(msg);
    }

    Ok(response)
}

pub fn try_accept_admin_role(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let sender = info.sender;
    let block_time = env.block.time.seconds();
    let TransferAdminState {
        new_admin,
        deadline,
    } = TRANSFER_ADMIN_STATE.load(deps.storage)?;

    if sender != new_admin {
        Err(ContractError::Unauthorized)?;
    }

    if block_time >= deadline {
        Err(ContractError::TransferAdminDeadline)?;
    }

    CONFIG.update(deps.storage, |mut x| -> StdResult<Config> {
        x.admin = sender;
        Ok(x)
    })?;

    TRANSFER_ADMIN_STATE.update(deps.storage, |mut x| -> StdResult<TransferAdminState> {
        x.deadline = block_time;
        Ok(x)
    })?;

    Ok(Response::new().add_attributes(vec![("action", "try_accept_admin_role")]))
}

#[allow(clippy::too_many_arguments)]
pub fn try_update_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    admin: Option<String>,
    equinox_voter: Option<String>,
    beclip_minter: Option<String>,
    beclip_address: Option<String>,
    beclip_whitelist: Option<Vec<String>>,
    lock_schedule: Option<Vec<(u64, u64)>>,
    dao_treasury_address: Option<String>,
    penalty_multiplier: Option<Decimal>,
    eclip_per_second_multiplier: Option<Decimal>,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        Err(ContractError::Unauthorized)?;
    }

    if let Some(x) = admin {
        let block_time = env.block.time.seconds();
        let new_admin = deps.api.addr_validate(&x)?;

        TRANSFER_ADMIN_STATE.save(
            deps.storage,
            &TransferAdminState {
                new_admin,
                deadline: block_time + TRANSFER_ADMIN_TIMEOUT,
            },
        )?;
    }

    if let Some(x) = equinox_voter {
        config.equinox_voter = Some(deps.api.addr_validate(&x)?);
    }

    if let Some(x) = beclip_minter {
        config.beclip_minter = Some(deps.api.addr_validate(&x)?);
    }

    if let Some(x) = beclip_address {
        config.beclip_address = Some(deps.api.addr_validate(&x)?);
    }

    if let Some(x) = beclip_whitelist {
        config.beclip_whitelist = x
            .iter()
            .map(|x| deps.api.addr_validate(x))
            .collect::<StdResult<Vec<Addr>>>()?;
    }

    if let Some(x) = lock_schedule {
        // don't allow to change tiers amount
        if x.len() != config.lock_schedule.len() {
            Err(ContractError::ImmutableTiersAmount)?;
        }

        config.lock_schedule = x;
    }

    if let Some(x) = dao_treasury_address {
        config.dao_treasury_address = deps.api.addr_validate(&x)?;
    }

    if let Some(x) = penalty_multiplier {
        config.penalty_multiplier = x;
    }

    if let Some(x) = eclip_per_second_multiplier {
        config.eclip_per_second_multiplier = x;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![("action", "try_update_config")]))
}

pub fn try_update_pagination_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    pagination_amount: Option<u32>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let pagination_config = PAGINATION_CONFIG.load(deps.storage)?;
    let mut current_pagination_amount = pagination_config.get_amount();
    let PaginationConfig {
        staking_pagination_index,
        locking_pagination_index,
        ..
    } = pagination_config;

    if info.sender != config.admin {
        Err(ContractError::Unauthorized)?;
    }

    if let Some(x) = pagination_amount {
        current_pagination_amount = x;
    }

    PAGINATION_CONFIG.save(
        deps.storage,
        &PaginationConfig::new(
            current_pagination_amount,
            &staking_pagination_index,
            &locking_pagination_index,
        ),
    )?;

    Ok(Response::new().add_attributes(vec![("action", "try_update_pagination_config")]))
}

pub fn try_decrease_balance(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let Config {
        staking_token,
        admin,
        ..
    } = CONFIG.load(deps.storage)?;

    if sender != admin {
        Err(ContractError::Unauthorized)?;
    }

    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: sender.to_string(),
        amount: coins(amount.u128(), staking_token),
    });

    Ok(Response::new()
        .add_message(msg)
        .add_attributes(vec![("action", "try_decrease_balance")]))
}

pub fn try_update_staking_essence_storages(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: u32,
    start_from: Option<String>,
) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let Config { admin, .. } = CONFIG.load(deps.storage)?;

    if sender != admin {
        Err(ContractError::Unauthorized)?;
    }

    let address;
    let start_bound = match start_from {
        None => None,
        Some(x) => {
            address = deps.api.addr_validate(&x)?;
            Some(Bound::exclusive(&address))
        }
    };

    let mut td_a = Uint128::zero();
    let mut td_b = Uint128::zero();

    let stakers_info = &STAKER_INFO
        .range(deps.storage, start_bound, None, Order::Ascending)
        .take(amount as usize)
        .flatten()
        .collect::<Vec<(Addr, StakerInfo)>>();

    for (sender, staker_info) in stakers_info {
        let (a1, b1) = STAKING_ESSENCE_COMPONENTS
            .load(deps.storage, sender)
            .unwrap_or_default();

        // skip if storage is updated
        if !a1.is_zero() || !b1.is_zero() {
            continue;
        }

        let (a2, b2) = math::v3::calc_components_from_staking_vaults(&staker_info.vaults);

        STAKING_ESSENCE_COMPONENTS.save(deps.storage, sender, &(a2, b2))?;

        td_a += a2;
        td_b += b2;
    }

    TOTAL_STAKING_ESSENCE_COMPONENTS
        .update(deps.storage, |(a, b)| -> StdResult<(Uint128, Uint128)> {
            Ok((a + td_a, b + td_b))
        })?;

    let last_address = &stakers_info
        .last()
        .map(|(a, _)| a.to_string())
        .unwrap_or_default();

    Ok(Response::new().add_attributes(vec![
        ("action", "try_update_staking_essence_storages"),
        ("last_address", last_address),
    ]))
}

pub fn try_update_locking_essence_storages(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: u32,
    start_from: Option<String>,
) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let Config {
        admin,
        lock_schedule,
        seconds_per_essence,
        ..
    } = CONFIG.load(deps.storage)?;

    if sender != admin {
        Err(ContractError::Unauthorized)?;
    }

    let address;
    let start_bound = match start_from {
        None => None,
        Some(x) => {
            address = deps.api.addr_validate(&x)?;
            Some(Bound::exclusive(&address))
        }
    };

    let mut total_locking_essence_difference = Uint128::zero();

    let locker_infos = &LOCKER_INFO
        .range(deps.storage, start_bound, None, Order::Ascending)
        .take(amount as usize)
        .flatten()
        .collect::<Vec<(Addr, Vec<LockerInfo>)>>();

    for (sender, locker_info) in locker_infos {
        let locking_essence_before = LOCKING_ESSENCE
            .load(deps.storage, sender)
            .unwrap_or_default();

        // skip if storage is updated
        if !locking_essence_before.is_zero() {
            continue;
        }

        let locking_essence_after =
            math::v3::calc_locking_essence(locker_info, &lock_schedule, seconds_per_essence);

        LOCKING_ESSENCE.save(deps.storage, sender, &locking_essence_after)?;

        total_locking_essence_difference += locking_essence_after;
    }

    TOTAL_LOCKING_ESSENCE.update(deps.storage, |x| -> StdResult<Uint128> {
        Ok(x + total_locking_essence_difference)
    })?;

    let last_address = &locker_infos
        .last()
        .map(|(a, _)| a.to_string())
        .unwrap_or_default();

    Ok(Response::new().add_attributes(vec![
        ("action", "try_update_locking_essence_storages"),
        ("last_address", last_address),
    ]))
}

pub fn try_filter_stakers(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: u32,
    start_from: Option<String>,
) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let Config { admin, .. } = CONFIG.load(deps.storage)?;

    if sender != admin {
        Err(ContractError::Unauthorized)?;
    }

    let address;
    let start_bound = match start_from {
        None => None,
        Some(x) => {
            address = deps.api.addr_validate(&x)?;
            Some(Bound::exclusive(&address))
        }
    };

    let stakers_info = &STAKER_INFO
        .range(deps.storage, start_bound, None, Order::Ascending)
        .take(amount as usize)
        .flatten()
        .collect::<Vec<(Addr, StakerInfo)>>();

    for (user, StakerInfo { vaults }) in stakers_info {
        if vaults.is_empty() {
            STAKER_INFO.remove(deps.storage, user);
        }
    }

    let last_address = &stakers_info
        .last()
        .map(|(a, _)| a.to_owned())
        .unwrap_or(Addr::unchecked("default"))
        .to_string();

    Ok(Response::new().add_attributes(vec![
        ("action", "try_filter_stakers"),
        ("last_address", last_address),
    ]))
}

pub fn try_filter_lockers(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: u32,
    start_from: Option<String>,
) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let Config { admin, .. } = CONFIG.load(deps.storage)?;

    if sender != admin {
        Err(ContractError::Unauthorized)?;
    }

    let address;
    let start_bound = match start_from {
        None => None,
        Some(x) => {
            address = deps.api.addr_validate(&x)?;
            Some(Bound::exclusive(&address))
        }
    };

    let locker_infos = &LOCKER_INFO
        .range(deps.storage, start_bound, None, Order::Ascending)
        .take(amount as usize)
        .flatten()
        .collect::<Vec<(Addr, Vec<LockerInfo>)>>();

    for (user, locker_info) in locker_infos {
        let vaults_amount = locker_info
            .iter()
            .fold(0, |acc, cur| acc + cur.vaults.len());

        if vaults_amount == 0 {
            LOCKER_INFO.remove(deps.storage, user);
        }
    }

    let last_address = &locker_infos
        .last()
        .map(|(a, _)| a.to_owned())
        .unwrap_or(Addr::unchecked("default"))
        .to_string();

    Ok(Response::new().add_attributes(vec![
        ("action", "try_filter_lockers"),
        ("last_address", last_address),
    ]))
}

pub fn try_pause(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let Config { admin, .. } = CONFIG.load(deps.storage)?;

    if sender != admin {
        Err(ContractError::Unauthorized)?;
    }

    IS_PAUSED.save(deps.storage, &true)?;

    Ok(Response::new().add_attributes(vec![("action", "try_pause")]))
}

pub fn try_unpause(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let Config { admin, .. } = CONFIG.load(deps.storage)?;

    if sender != admin {
        Err(ContractError::Unauthorized)?;
    }

    IS_PAUSED.save(deps.storage, &false)?;

    Ok(Response::new().add_attributes(vec![("action", "try_unpause")]))
}
