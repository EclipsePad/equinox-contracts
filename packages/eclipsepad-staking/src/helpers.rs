use cosmwasm_std::{Addr, Decimal, Deps, Order, StdResult, Storage, Uint128};
use cw_storage_plus::Bound;

use eclipse_base::{
    converters::u128_to_dec,
    error::ContractError,
    staking::{
        state::{
            CONFIG, DECREASING_REWARDS_DATE, DECREASING_REWARDS_PERIOD, IS_PAUSED, LOCKER_INFO,
            LOCKING_ESSENCE, LOCK_STATES, PAGINATION_CONFIG, STAKER_INFO,
            STAKING_ESSENCE_COMPONENTS, TOTAL_LOCKING_ESSENCE, TOTAL_STAKING_ESSENCE_COMPONENTS,
            VAULTS_LIMIT,
        },
        types::{Config, LockerInfo, PaginationConfig, StakerInfo, State},
    },
};
use equinox_msg::voter::{msg::QueryEssenceListResponse, types::EssenceInfo};

use crate::math;

pub fn check_pause_state(deps: Deps) -> StdResult<()> {
    if IS_PAUSED.load(deps.storage)? {
        Err(ContractError::ContractIsPaused)?;
    }

    Ok(())
}

pub fn get_essence_snapshot(storage: &dyn Storage, user: &Addr) -> QueryEssenceListResponse {
    let (a1, b1) = STAKING_ESSENCE_COMPONENTS
        .load(storage, user)
        .unwrap_or_default();
    // let staking_essence =
    //     math::v3::calc_staking_essence_from_components(a, b, block_time, seconds_per_essence);
    let locking_essence = LOCKING_ESSENCE.load(storage, &user).unwrap_or_default();

    let (a2, b2) = TOTAL_STAKING_ESSENCE_COMPONENTS
        .load(storage)
        .unwrap_or_default();
    // let total_staking_essence =
    //     math::v3::calc_staking_essence_from_components(a, b, block_time, seconds_per_essence);
    let total_locking_essence = TOTAL_LOCKING_ESSENCE.load(storage).unwrap_or_default();

    QueryEssenceListResponse {
        user_and_essence_list: vec![(
            user.to_string(),
            EssenceInfo {
                staking_components: (a1, b1),
                locking_amount: locking_essence,
            },
        )],
        total_essence: EssenceInfo {
            staking_components: (a2, b2),
            locking_amount: total_locking_essence,
        },
    }
}

pub mod v2 {
    use super::*;

    pub fn update_eclip_per_second(
        _storage: &mut dyn Storage,
        _block_time: u64,
        _decreasing_rewards_date: u64,
    ) -> StdResult<()> {
        Ok(())
    }

    pub fn accumulate_rewards(storage: &mut dyn Storage, block_time: u64) -> StdResult<()> {
        let mut pagination_config = PAGINATION_CONFIG.load(storage)?;
        let pagination_amount = pagination_config.get_amount();
        let PaginationConfig {
            locking_pagination_index,
            ..
        } = pagination_config.clone();
        let lock_states = LOCK_STATES.load(storage)?;
        let Config { lock_schedule, .. } = CONFIG.load(storage)?;

        let locking_start_bound = locking_pagination_index.as_ref().map(Bound::exclusive);

        let lockers: Vec<(Addr, Vec<LockerInfo>)> = LOCKER_INFO
            .range(storage, locking_start_bound, None, Order::Ascending)
            .take(pagination_amount as usize)
            .flatten()
            .collect();

        // update locking vaults
        for (locker_address, locker_infos) in lockers.clone() {
            let locker_infos_new = locker_infos
                .into_iter()
                .map(|mut locker_info| {
                    if locker_info.vaults.is_empty() {
                        return locker_info;
                    }

                    let (locking_period, global_rewards_per_tier) =
                        lock_schedule[locker_info.lock_tier as usize];
                    let State {
                        total_bond_amount, ..
                    } = lock_states[locker_info.lock_tier as usize];

                    locker_info.vaults = locker_info
                        .vaults
                        .into_iter()
                        .map(|mut vault| {
                            vault.accumulated_rewards =
                                math::v2::calc_accumulated_locking_rewards_per_vault(
                                    &vault,
                                    global_rewards_per_tier,
                                    locking_period,
                                    block_time,
                                    total_bond_amount,
                                );
                            vault.claim_date = block_time;

                            vault
                        })
                        .collect();

                    locker_info
                })
                .collect();

            LOCKER_INFO.save(storage, &locker_address, &locker_infos_new)?;
        }

        let lockers_last = lockers
            .last()
            .map(|(pagination_index_new, _)| pagination_index_new.to_owned());
        let storage_last = LOCKER_INFO
            .last(storage)?
            .map(|(pagination_index_new, _)| pagination_index_new);

        let locking_pagination_index_new = if lockers_last == storage_last {
            None
        } else {
            lockers_last
        };

        pagination_config.locking_pagination_index = locking_pagination_index_new;

        PAGINATION_CONFIG.save(storage, &pagination_config)?;

        Ok(())
    }
}

pub mod v3 {
    use super::*;

    pub fn split_eclip_per_second(
        storage: &dyn Storage,
        eclip_per_second_multiplier: Decimal,
        eclip_per_second: u64,
        block_time: u64,
    ) -> StdResult<(u64, u64, u64)> {
        let decreasing_rewards_date = DECREASING_REWARDS_DATE.load(storage)?;

        let eclip_per_second_before = eclip_per_second;
        let eclip_per_second_after = if block_time < decreasing_rewards_date {
            eclip_per_second
        } else {
            (u128_to_dec(eclip_per_second) * eclip_per_second_multiplier)
                .to_uint_floor()
                .u128() as u64
        };

        Ok((
            decreasing_rewards_date,
            eclip_per_second_before,
            eclip_per_second_after,
        ))
    }

    pub fn update_eclip_per_second(
        storage: &mut dyn Storage,
        block_time: u64,
        decreasing_rewards_date: u64,
    ) -> StdResult<()> {
        if block_time < decreasing_rewards_date {
            return Ok(());
        }

        DECREASING_REWARDS_DATE.save(
            storage,
            &(decreasing_rewards_date + DECREASING_REWARDS_PERIOD),
        )?;

        CONFIG.update(storage, |mut x| -> StdResult<Config> {
            x.eclip_per_second = (u128_to_dec(x.eclip_per_second) * x.eclip_per_second_multiplier)
                .to_uint_floor()
                .u128() as u64;

            Ok(x)
        })?;

        Ok(())
    }

    pub fn accumulate_rewards(
        storage: &mut dyn Storage,
        lock_schedule: &[(u64, u64)],
        block_time: u64,
        decreasing_rewards_date: u64,
        eclip_per_second_before: u64,
        eclip_per_second_after: u64,
    ) -> StdResult<()> {
        let mut pagination_config = PAGINATION_CONFIG.load(storage)?;
        let pagination_amount = pagination_config.get_amount();
        let PaginationConfig {
            staking_pagination_index,
            locking_pagination_index,
            ..
        } = pagination_config;

        let Config {
            seconds_per_essence,
            ..
        } = CONFIG.load(storage)?;

        // get paginated users
        let staking_start_bound = staking_pagination_index.as_ref().map(Bound::exclusive);
        let locking_start_bound = locking_pagination_index.as_ref().map(Bound::exclusive);

        let mut stakers: Vec<(Addr, StakerInfo)> = STAKER_INFO
            .range(storage, staking_start_bound, None, Order::Ascending)
            .take(pagination_amount as usize)
            .flatten()
            .collect();

        let mut lockers: Vec<(Addr, Vec<LockerInfo>)> = LOCKER_INFO
            .range(storage, locking_start_bound, None, Order::Ascending)
            .take(pagination_amount as usize)
            .flatten()
            .collect();

        // if the end of map storage was reached add items from the start
        // final list length will be limited by storage length and duplicated items will be removed
        let stakers_length = stakers.len() as u32;

        if stakers_length < pagination_amount {
            let staker_addresses: Vec<Addr> = stakers
                .iter()
                .cloned()
                .map(|(address, _)| address)
                .collect();

            let additional_stakers: Vec<(Addr, StakerInfo)> = STAKER_INFO
                .range(storage, None, None, Order::Ascending)
                .take((pagination_amount - stakers_length) as usize)
                .flatten()
                .filter(|(address, _)| !staker_addresses.contains(address))
                .collect();

            stakers = vec![stakers, additional_stakers].concat();
        }

        let lockers_length = lockers.len() as u32;

        if lockers_length < pagination_amount {
            let locker_addresses: Vec<Addr> = lockers
                .iter()
                .cloned()
                .map(|(address, _)| address)
                .collect();

            let additional_lockers: Vec<(Addr, Vec<LockerInfo>)> = LOCKER_INFO
                .range(storage, None, None, Order::Ascending)
                .take((pagination_amount - lockers_length) as usize)
                .flatten()
                .filter(|(address, _)| !locker_addresses.contains(address))
                .collect();

            lockers = vec![lockers, additional_lockers].concat();
        }

        pagination_config.staking_pagination_index =
            stakers.last().map(|(index, _)| index.to_owned());
        pagination_config.locking_pagination_index =
            lockers.last().map(|(index, _)| index.to_owned());

        PAGINATION_CONFIG.save(storage, &pagination_config)?;

        // get total essence
        let (a, b) = TOTAL_STAKING_ESSENCE_COMPONENTS.load(storage)?;
        let total_essence =
            math::v3::calc_staking_essence_from_components(a, b, block_time, seconds_per_essence)
                + TOTAL_LOCKING_ESSENCE.load(storage)?;

        // update staking vaults
        for (staker_address, mut staker_info) in stakers {
            staker_info.vaults = staker_info
                .vaults
                .iter()
                .cloned()
                .map(|mut vault| {
                    let staking_essence_per_vault = math::v3::calc_staking_essence_per_vault(
                        vault.amount,
                        vault.creation_date,
                        block_time,
                        seconds_per_essence,
                    );

                    vault.accumulated_rewards = math::v3::calc_staking_rewards_per_vault(
                        vault.accumulated_rewards,
                        staking_essence_per_vault,
                        vault.claim_date,
                        decreasing_rewards_date,
                        block_time,
                        eclip_per_second_before,
                        eclip_per_second_after,
                        total_essence,
                    );
                    vault.claim_date = block_time;

                    vault
                })
                .collect();

            STAKER_INFO.save(storage, &staker_address, &staker_info)?;
        }

        // update locking vaults
        for (locker_address, locker_infos) in lockers {
            let locker_infos_new = locker_infos
                .iter()
                .cloned()
                .map(|mut locker_info| {
                    if locker_info.vaults.is_empty() {
                        return locker_info;
                    }

                    let (locking_period, _global_rewards_per_tier) =
                        lock_schedule[locker_info.lock_tier as usize];

                    locker_info.vaults = locker_info
                        .vaults
                        .into_iter()
                        .map(|mut vault| {
                            let locking_essence_per_vault =
                                math::v3::calc_locking_essence_per_vault(
                                    vault.amount,
                                    locking_period,
                                    seconds_per_essence,
                                );

                            vault.accumulated_rewards = math::v3::calc_locking_rewards_per_vault(
                                vault.accumulated_rewards,
                                locking_essence_per_vault,
                                vault.claim_date,
                                decreasing_rewards_date,
                                block_time,
                                eclip_per_second_before,
                                eclip_per_second_after,
                                total_essence,
                            );
                            vault.claim_date = block_time;

                            vault
                        })
                        .collect();

                    locker_info
                })
                .collect();

            LOCKER_INFO.save(storage, &locker_address, &locker_infos_new)?;
        }

        Ok(())
    }

    pub fn get_total_essence(
        storage: &dyn Storage,
        block_time: u64,
        seconds_per_essence: Uint128,
    ) -> StdResult<Uint128> {
        let total_staking_essence_components = TOTAL_STAKING_ESSENCE_COMPONENTS.load(storage)?;
        let (a, b) = total_staking_essence_components;
        let total_staking_essence =
            math::v3::calc_staking_essence_from_components(a, b, block_time, seconds_per_essence);
        let total_locking_essence = TOTAL_LOCKING_ESSENCE.load(storage).unwrap_or_default();

        Ok(total_staking_essence + total_locking_essence)
    }

    // don't allow too much vaults
    pub fn check_vaults_amount(storage: &dyn Storage, user: &Addr) -> StdResult<()> {
        let staker_info = STAKER_INFO.load(storage, user).unwrap_or_default();
        let locker_infos = LOCKER_INFO.load(storage, user).unwrap_or_default();
        let vaults_amount = locker_infos
            .iter()
            .fold(staker_info.vaults.len(), |acc, cur| acc + cur.vaults.len());

        if vaults_amount >= VAULTS_LIMIT {
            Err(ContractError::TooMuchVaults)?;
        }

        Ok(())
    }
}
