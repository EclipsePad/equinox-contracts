use cosmwasm_std::{Addr, Decimal, Uint128};

use eclipse_base::{
    converters::{str_to_dec, u128_to_dec},
    staking::state::SECONDS_PER_ESSENCE,
};
use equinox_msg::voter::{
    state::{DAO_TREASURY_REWARDS_FRACTION, ELECTOR_ADDITIONAL_ESSENCE_FRACTION},
    types::{
        BribesAllocationItem, EssenceAllocationItem, EssenceInfo, PoolInfoItem,
        WeightAllocationItem,
    },
};

/// essence_allocation = essence * weights
pub fn calc_essence_allocation(
    essence: &EssenceInfo,
    weights: &Vec<WeightAllocationItem>,
) -> Vec<EssenceAllocationItem> {
    weights
        .into_iter()
        .map(|x| EssenceAllocationItem {
            lp_token: x.lp_token.to_string(),
            essence_info: essence.scale(x.weight),
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

    // offset is required when we have stake + lock msgs in single tx to avoid
    // essence_info.capture(block_time) == 0 and then div by zero and subtract with overflow errors
    let block_time = if essence_info.capture(block_time).is_zero() {
        block_time + SECONDS_PER_ESSENCE as u64
    } else {
        block_time
    };
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

/// pool_info_list_with_voter_rewards -> (pool_info_list_with_elector_rewards, dao_rewards)                                     \
/// dao_rewards_per_denom = sum_over_pools(voter_rewards * (dao_essence * dao_weight) / (voter_essence * voter_weight))
pub fn split_rewards(
    pool_info_list: &[PoolInfoItem],
    dao_weight_list: &[WeightAllocationItem],
    elector_essence: Uint128,
    dao_essence: Uint128,
) -> (Vec<PoolInfoItem>, Vec<(Uint128, String)>) {
    // scale and split rewards
    let essence_ratio = u128_to_dec(dao_essence) / u128_to_dec(elector_essence + dao_essence);
    let mut dao_rewards_raw: Vec<(Uint128, String)> = vec![];

    let pool_info_list_new: Vec<PoolInfoItem> = pool_info_list
        .iter()
        .cloned()
        .map(|mut pool_info_item| {
            let dao_weight = dao_weight_list
                .iter()
                .find(|x| x.lp_token == pool_info_item.lp_token)
                .unwrap_or(&WeightAllocationItem {
                    lp_token: String::default(),
                    weight: Decimal::zero(),
                })
                .weight;

            let rewards_ratio = essence_ratio * dao_weight / pool_info_item.weight;

            pool_info_item.rewards = pool_info_item
                .rewards
                .iter()
                .cloned()
                .map(|(amount, denom)| {
                    let dao_amount = (u128_to_dec(amount) * rewards_ratio).to_uint_floor();
                    dao_rewards_raw.push((dao_amount, denom.clone()));

                    (amount - dao_amount, denom)
                })
                .collect();

            pool_info_item
        })
        .collect();

    // get unique denom list
    let mut denom_list: Vec<String> = dao_rewards_raw
        .iter()
        .map(|(_, denom)| denom.to_owned())
        .collect();
    denom_list.sort_unstable();
    denom_list.dedup();

    // aggregate rewards by denom
    let dao_rewards: Vec<(Uint128, String)> = denom_list
        .iter()
        .map(|denom| {
            let amount =
                dao_rewards_raw
                    .iter()
                    .fold(Uint128::zero(), |acc, (cur_amount, cur_denom)| {
                        if cur_denom != denom {
                            acc
                        } else {
                            acc + cur_amount
                        }
                    });

            (amount, denom.to_owned())
        })
        .collect();

    (pool_info_list_new, dao_rewards)
}

/// personal_rewards = elector_rewards * (personal_elector_essence * personal_weight) / (elector_self_essence * elector_weight)     \
/// elector_self_essence = elector_essence - ELECTOR_ADDITIONAL_ESSENCE_FRACTION * slacker_essence
pub fn calc_personal_elector_rewards(
    pool_info_list: &[PoolInfoItem],
    elector_weight_list: &[WeightAllocationItem],
    personal_elector_weight_list: &[WeightAllocationItem],
    slacker_essence: Uint128,
    elector_essence: Uint128,
    personal_elector_essence: Uint128,
) -> Vec<(Uint128, String)> {
    let essence_ratio = u128_to_dec(personal_elector_essence)
        / (u128_to_dec(elector_essence)
            - str_to_dec(ELECTOR_ADDITIONAL_ESSENCE_FRACTION) * u128_to_dec(slacker_essence));

    let personal_elector_rewards_raw: Vec<(Uint128, String)> = personal_elector_weight_list
        .into_iter()
        .map(|personal_weight| {
            // it's safe to unwrap find results as personal elector votes are included in elector votes
            let elector_weight = elector_weight_list
                .iter()
                .find(|x| x.lp_token == personal_weight.lp_token)
                .unwrap()
                .weight;

            let elector_rewards_per_pool = &pool_info_list
                .iter()
                .find(|x| x.lp_token == personal_weight.lp_token)
                .unwrap()
                .rewards;

            let amount_ratio = essence_ratio * personal_weight.weight / elector_weight;
            let personal_elector_rewards_per_pool: Vec<(Uint128, String)> =
                elector_rewards_per_pool
                    .iter()
                    .cloned()
                    .map(|(amount, denom)| {
                        ((u128_to_dec(amount) * amount_ratio).to_uint_floor(), denom)
                    })
                    .collect();

            personal_elector_rewards_per_pool
        })
        .flatten()
        .collect();

    let mut denom_list: Vec<String> = pool_info_list
        .iter()
        .map(|x| x.lp_token.to_string())
        .collect();
    denom_list.sort_unstable();
    denom_list.dedup();

    let personal_elector_rewards: Vec<(Uint128, String)> = denom_list
        .iter()
        .map(|denom| {
            let amount = personal_elector_rewards_raw.iter().fold(
                Uint128::zero(),
                |acc, (cur_amount, cur_denom)| {
                    if cur_denom != denom {
                        acc
                    } else {
                        acc + cur_amount
                    }
                },
            );

            (amount, denom.to_owned())
        })
        .collect();

    personal_elector_rewards
}

/// dao_eclip_rewards -> (dao_treasury_eclip_rewards, delegator_rewards)                \
/// delegator_rewards = (1 - DAO_TREASURY_REWARDS_FRACTION) * dao_eclip_rewards         \
/// dao_treasury_eclip_rewards = dao_eclip_rewards - delegator_rewards
pub fn slpit_dao_eclip_rewards(dao_eclip_rewards: Uint128) -> (Uint128, Uint128) {
    let delegator_rewards = ((Decimal::one() - str_to_dec(DAO_TREASURY_REWARDS_FRACTION))
        * u128_to_dec(dao_eclip_rewards))
    .to_uint_floor();
    let dao_treasury_eclip_rewards = dao_eclip_rewards - delegator_rewards;

    (dao_treasury_eclip_rewards, delegator_rewards)
}

/// delegator_rewards = dao_eclip_rewards * delegator_essence / dao_self_essence                    \
/// dao_self_essence = dao_essence - (1 - ELECTOR_ADDITIONAL_ESSENCE_FRACTION) * slacker_essence
pub fn calc_delegator_rewards(
    dao_eclip_rewards: Uint128,
    slacker_essence: Uint128,
    dao_essence: Uint128,
    delegator_essence: Uint128,
) -> Uint128 {
    let dao_self_essence = dao_essence
        - ((Decimal::one() - str_to_dec(ELECTOR_ADDITIONAL_ESSENCE_FRACTION))
            * u128_to_dec(slacker_essence))
        .to_uint_floor();

    dao_eclip_rewards * delegator_essence / dao_self_essence
}
