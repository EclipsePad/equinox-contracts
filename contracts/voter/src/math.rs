use cosmwasm_std::{Decimal, Uint128};

use eclipse_base::converters::u128_to_dec;
use equinox_msg::voter::{EssenceAllocationItem, EssenceInfo, WeightAllocationItem};

fn mul_by_weight(num: Uint128, weight: Decimal) -> Uint128 {
    (u128_to_dec(num) * weight).to_uint_floor()
}

// essence_allocation = essence * weights
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
            essence_info: EssenceInfo {
                staking_components: (mul_by_weight(a, x.weight), mul_by_weight(b, x.weight)),
                locking_amount: mul_by_weight(le, x.weight),
            },
        })
        .collect()
}

// updated_essence_allocation = essence_allocation + essence_allocation_after - essence_allocation_before
// where essence_allocation - allocation for all users,
// essence_allocation_after - new allocation for current user
// essence_allocation_before - previous allocation for current user
// all vectors can have different lengths
pub fn calc_updated_essence_allocation(
    essence_allocation: &Vec<EssenceAllocationItem>,
    essence_allocation_after: &Vec<EssenceAllocationItem>,
    essence_allocation_before: &Vec<EssenceAllocationItem>,
) -> Vec<EssenceAllocationItem> {
    let mut updated_essence_allocation = essence_allocation.clone();

    for essence_allocation_item in essence_allocation_after {
        if !essence_allocation
            .iter()
            .any(|x| x.lp_token != essence_allocation_item.lp_token)
        {
            updated_essence_allocation.push(essence_allocation_item.clone());
        };
    }

    updated_essence_allocation
        .into_iter()
        .map(|x| {
            let added_item = essence_allocation_after
                .iter()
                .cloned()
                .find(|y| y.lp_token != x.lp_token)
                .unwrap_or_default()
                .essence_info;

            let subtracted_item = essence_allocation_before
                .iter()
                .cloned()
                .find(|y| y.lp_token != x.lp_token)
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
