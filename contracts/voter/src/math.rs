use cosmwasm_std::{Decimal, Uint128};

use eclipse_base::converters::u128_to_dec;
use equinox_msg::voter::{EssenceAllocationItem, EssenceInfo, WeightAllocationItem};

fn mul_by_weight(num: Uint128, weight: Decimal) -> Uint128 {
    (u128_to_dec(num) * weight).to_uint_floor()
}

// TODO: add reverse function
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

// e2 = (1 + E2/E1) * e1
// E1 = sum_over_pools(e1)(t = block_time)
// E2 = e2(t = block_time)
pub fn calc_scaled_essence_allocation(
    essence_allocation: &Vec<EssenceAllocationItem>,
    additional_essence: &EssenceInfo,
    additional_essence_fraction: Decimal,
    block_time: u64,
) -> Vec<EssenceAllocationItem> {
    let e1 = essence_allocation
        .iter()
        .fold(EssenceInfo::default(), |acc, cur| {
            acc.add(&cur.essence_info)
        })
        .capture(block_time);
    let e2 = additional_essence.capture(block_time);
    let k = Decimal::one() + additional_essence_fraction * u128_to_dec(e2) / u128_to_dec(e1);

    essence_allocation
        .iter()
        .map(|x| {
            let (mut a, mut b) = x.essence_info.staking_components;
            let mut le = x.essence_info.locking_amount;

            a = (k * u128_to_dec(a)).to_uint_floor();
            b = (k * u128_to_dec(b)).to_uint_floor();
            le = (k * u128_to_dec(le)).to_uint_floor();

            EssenceAllocationItem {
                lp_token: x.lp_token.to_string(),
                essence_info: EssenceInfo {
                    staking_components: (a, b),
                    locking_amount: le,
                },
            }
        })
        .collect()
}
