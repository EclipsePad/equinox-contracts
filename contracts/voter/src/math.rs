use cosmwasm_std::{Addr, Decimal, Uint128};

use eclipse_base::converters::u128_to_dec;
use equinox_msg::voter::types::{
    BribesAllocationItem, EssenceAllocationItem, EssenceInfo, PoolInfoItem, WeightAllocationItem,
};

/// essence_allocation = essence * weights
pub fn calc_essence_allocation(
    essence: &EssenceInfo,
    weights: &Vec<WeightAllocationItem>,
) -> Vec<EssenceAllocationItem> {
    let (a, b) = essence.staking_components;
    let le = essence.locking_amount;

    weights
        .into_iter()
        .map(|x| EssenceAllocationItem {
            lp_token: x.lp_token.to_string(),
            essence_info: EssenceInfo::new(a, b, le).scale(x.weight),
        })
        .collect()
}

/// essence = sum(essence_allocation)                                                       \
/// weights = essence_allocation / essence
pub fn calc_weights_from_essence_allocation(
    essence_allocation: &Vec<EssenceAllocationItem>,
    block_time: u64,
) -> (EssenceInfo, Vec<WeightAllocationItem>) {
    let essence_info = essence_allocation
        .iter()
        .fold(EssenceInfo::default(), |acc, cur| {
            acc.add(&cur.essence_info)
        });
    let essence_info_decimal = u128_to_dec(essence_info.capture(block_time));

    let weights: Vec<WeightAllocationItem> = essence_allocation
        .into_iter()
        .map(|x| WeightAllocationItem {
            lp_token: x.lp_token.clone(),
            weight: u128_to_dec(x.essence_info.capture(block_time)) / essence_info_decimal,
        })
        .collect();

    (essence_info, weights)
}

/// updated_essence_allocation = essence_allocation + essence_allocation_after - essence_allocation_before          \
/// where essence_allocation - allocation for all users,                                                            \
/// essence_allocation_after - new allocation for current user                                                      \
/// essence_allocation_before - previous allocation for current user                                                \
/// all vectors can have different lengths
pub fn calc_updated_essence_allocation(
    essence_allocation: &Vec<EssenceAllocationItem>,
    essence_allocation_after: &Vec<EssenceAllocationItem>,
    essence_allocation_before: &Vec<EssenceAllocationItem>,
) -> Vec<EssenceAllocationItem> {
    let mut updated_essence_allocation = essence_allocation.clone();

    for essence_allocation_item in essence_allocation_after {
        if essence_allocation
            .iter()
            .all(|x| x.lp_token != essence_allocation_item.lp_token)
        {
            updated_essence_allocation.push(EssenceAllocationItem {
                lp_token: essence_allocation_item.lp_token.clone(),
                essence_info: EssenceInfo::default(),
            });
        };
    }

    updated_essence_allocation
        .into_iter()
        .map(|x| {
            let added_item = essence_allocation_after
                .iter()
                .cloned()
                .find(|y| y.lp_token == x.lp_token)
                .unwrap_or_default()
                .essence_info;

            let subtracted_item = essence_allocation_before
                .iter()
                .cloned()
                .find(|y| y.lp_token == x.lp_token)
                .unwrap_or_default()
                .essence_info;

            EssenceAllocationItem {
                lp_token: x.lp_token,
                essence_info: x.essence_info.add(&added_item).sub(&subtracted_item),
            }
        })
        .filter(|x| !x.essence_info.is_zero())
        .collect()
}

/// scaled_essence_allocation = (base_essence + additional_essence_fraction * additional_essence) * base_weights
pub fn calc_scaled_essence_allocation(
    base_essence: &EssenceInfo,
    base_weights: &Vec<WeightAllocationItem>,
    additional_essence: &EssenceInfo,
    additional_essence_fraction: Decimal,
) -> Vec<EssenceAllocationItem> {
    let essence = &base_essence.add(&additional_essence.scale(additional_essence_fraction));
    calc_essence_allocation(essence, base_weights)
}

/// voter_bribe_allocation = tribute_market_bribe_allocation * voter_voting_power_allocation / tribute_market_voting_power_allocation
pub fn calc_pool_info_list_with_rewards(
    pool_info_list_without_rewards: &[PoolInfoItem],
    tribute_market_bribe_allocation: &[BribesAllocationItem],
    voter_to_tribute_voting_power_ratio_allocation: &[(String, Decimal)],
) -> Vec<PoolInfoItem> {
    pool_info_list_without_rewards
        .iter()
        .cloned()
        .map(|mut pool_info_item| {
            let tribute_rewards = tribute_market_bribe_allocation
                .iter()
                .cloned()
                .find(|x| x.lp_token == pool_info_item.lp_token)
                .unwrap_or(BribesAllocationItem {
                    lp_token: Addr::unchecked(String::default()),
                    rewards: vec![],
                })
                .rewards;

            let (_, voter_to_tribute_voting_power_ratio) =
                voter_to_tribute_voting_power_ratio_allocation
                    .iter()
                    .cloned()
                    .find(|(lp_token, _)| lp_token == &pool_info_item.lp_token)
                    .unwrap_or((String::default(), Decimal::zero()));

            if voter_to_tribute_voting_power_ratio.is_zero() {
                pool_info_item.rewards = vec![];
                return pool_info_item;
            }

            pool_info_item.rewards = tribute_rewards
                .into_iter()
                .map(|(amount, denom)| {
                    (
                        (u128_to_dec(amount) * voter_to_tribute_voting_power_ratio).to_uint_floor(),
                        denom,
                    )
                })
                .collect();
            pool_info_item
        })
        .collect()
}

// TODO: calc_elector_rewards, calc_delegator_rewards, calc_dao_rewards, split_rewards
