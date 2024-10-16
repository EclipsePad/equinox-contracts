use cosmwasm_std::{Addr, Decimal, Uint128};

use eclipse_base::{
    converters::{str_to_dec, u128_to_dec},
    staking::state::SECONDS_PER_ESSENCE,
    voter::{
        state::{
            DAO_TREASURY_REWARDS_FRACTION, ELECTOR_ADDITIONAL_ESSENCE_FRACTION,
            ELECTOR_BASE_ESSENCE_FRACTION,
        },
        types::{
            BribesAllocationItem, EssenceAllocationItem, EssenceInfo, PoolInfoItem,
            WeightAllocationItem,
        },
    },
};

/// xastro_price = astro_supply / xastro_supply
pub fn calc_xastro_price(astro_supply: Uint128, xastro_supply: Uint128) -> Decimal {
    if xastro_supply.is_zero() {
        return Decimal::zero();
    }

    u128_to_dec(astro_supply) / u128_to_dec(xastro_supply)
}

/// eclip_astro_for_xastro = xastro_amount * astro_supply / xastro_supply
pub fn calc_eclip_astro_for_xastro(
    xastro_amount: Uint128,
    astro_supply: Uint128,
    xastro_supply: Uint128,
) -> Uint128 {
    if xastro_supply.is_zero() {
        return Uint128::zero();
    }

    xastro_amount.multiply_ratio(astro_supply, xastro_supply)
}

/// voter_to_tribute_voting_power_ratio = voter_voting_power_decimal * applied_votes_weights_item / tribute_market_voting_power
pub fn calc_voter_to_tribute_voting_power_ratio(
    applied_votes_weights_item: &Decimal,
    voter_voting_power_decimal: Decimal,
    tribute_market_voting_power: Uint128,
) -> Decimal {
    if tribute_market_voting_power.is_zero() {
        return Decimal::zero();
    }

    std::cmp::min(
        voter_voting_power_decimal * applied_votes_weights_item
            / u128_to_dec(tribute_market_voting_power),
        Decimal::one(),
    )
}

/// voting_power = vxastro_amount * user_essence / total_essence
/// total_essence = elector_essence_acc + dao_essence_acc + slacker_essence_acc
pub fn calc_voting_power(
    vxastro_amount: Uint128,
    user_essence: Uint128,
    elector_essence_acc: Uint128,
    dao_essence_acc: Uint128,
    slacker_essence_acc: Uint128,
) -> Uint128 {
    let total_essence = elector_essence_acc + dao_essence_acc + slacker_essence_acc;

    if user_essence.is_zero() || total_essence.is_zero() {
        return Uint128::zero();
    }

    vxastro_amount * user_essence / total_essence
}

/// essence_allocation = essence * weights
pub fn calc_essence_allocation(
    essence: &EssenceInfo,
    weights: &[WeightAllocationItem],
) -> Vec<EssenceAllocationItem> {
    weights
        .iter()
        .map(|x| EssenceAllocationItem {
            lp_token: x.lp_token.to_string(),
            essence_info: essence.scale(x.weight),
        })
        .collect()
}

/// essence = sum(essence_allocation)                                                       \
/// weights = essence_allocation / essence
pub fn calc_weights_from_essence_allocation(
    essence_allocation: &[EssenceAllocationItem],
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
        .iter()
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
/// essence_allocation_before - previous allocation for current user
pub fn calc_updated_essence_allocation(
    essence_allocation: &[EssenceAllocationItem],
    essence_allocation_after: &[EssenceAllocationItem],
    essence_allocation_before: &[EssenceAllocationItem],
) -> Vec<EssenceAllocationItem> {
    let mut lp_token_list: Vec<String> = essence_allocation
        .iter()
        .map(|x| x.lp_token.to_owned())
        .chain(
            essence_allocation_after
                .iter()
                .map(|x| x.lp_token.to_owned()),
        )
        .collect();
    lp_token_list.sort_unstable();
    lp_token_list.dedup();

    lp_token_list
        .iter()
        .map(|lp_token| {
            let essence_info = essence_allocation
                .iter()
                .cloned()
                .find(|x| &x.lp_token == lp_token)
                .unwrap_or_default()
                .essence_info;

            let added_item = essence_allocation_after
                .iter()
                .cloned()
                .find(|x| &x.lp_token == lp_token)
                .unwrap_or_default()
                .essence_info;

            let subtracted_item = essence_allocation_before
                .iter()
                .cloned()
                .find(|x| &x.lp_token == lp_token)
                .unwrap_or_default()
                .essence_info;

            EssenceAllocationItem {
                lp_token: lp_token.to_owned(),
                essence_info: essence_info.add(&added_item).sub(&subtracted_item),
            }
        })
        .filter(|x| !x.essence_info.is_zero())
        .collect()
}

/// scaled_essence_allocation = (base_essence + additional_essence_fraction * additional_essence) * base_weights
pub fn calc_scaled_essence_allocation(
    base_essence: &EssenceInfo,
    base_weights: &[WeightAllocationItem],
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
                .unwrap_or_default()
                .rewards;

            let (_, voter_to_tribute_voting_power_ratio) =
                voter_to_tribute_voting_power_ratio_allocation
                    .iter()
                    .cloned()
                    .find(|(lp_token, _)| lp_token == &pool_info_item.lp_token)
                    .unwrap_or_default();

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
                .cloned()
                .find(|x| x.lp_token == pool_info_item.lp_token)
                .unwrap_or_default()
                .weight;

            let rewards_ratio = essence_ratio * dao_weight / pool_info_item.weight;

            pool_info_item.rewards = pool_info_item
                .rewards
                .iter()
                .cloned()
                .map(|(amount, denom)| {
                    let dao_amount = (u128_to_dec(amount) * rewards_ratio).to_uint_floor();
                    dao_rewards_raw.push((dao_amount, denom.clone()));

                    (amount - std::cmp::min(dao_amount, amount), denom)
                })
                .collect();

            pool_info_item
        })
        .collect();

    (
        pool_info_list_new,
        calc_rewards_aggregated_by_denom(&dao_rewards_raw),
    )
}

/// personal_rewards = elector_rewards * (personal_elector_essence * personal_weight) / (elector_self_essence * elector_weight)     \
/// elector_self_essence = (elector_essence - ELECTOR_ADDITIONAL_ESSENCE_FRACTION * slacker_essence) / ELECTOR_BASE_ESSENCE_FRACTION
pub fn calc_personal_elector_rewards(
    pool_info_list: &[PoolInfoItem],
    elector_weight_list: &[WeightAllocationItem],
    personal_elector_weight_list: &[WeightAllocationItem],
    slacker_essence: Uint128,
    elector_essence: Uint128,
    personal_elector_essence: Uint128,
) -> Vec<(Uint128, String)> {
    let essence_ratio = u128_to_dec(personal_elector_essence)
        * str_to_dec(ELECTOR_BASE_ESSENCE_FRACTION)
        / (u128_to_dec(elector_essence)
            - str_to_dec(ELECTOR_ADDITIONAL_ESSENCE_FRACTION) * u128_to_dec(slacker_essence));

    let personal_elector_rewards_raw: Vec<(Uint128, String)> = personal_elector_weight_list
        .iter()
        .flat_map(|personal_weight| {
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
        .collect();

    calc_rewards_aggregated_by_denom(&personal_elector_rewards_raw)
}

/// dao_eclip_rewards -> (dao_treasury_eclip_rewards, delegator_rewards)                \
/// delegator_rewards = (1 - DAO_TREASURY_REWARDS_FRACTION) * dao_eclip_rewards         \
/// dao_treasury_eclip_rewards = dao_eclip_rewards - delegator_rewards
pub fn split_dao_eclip_rewards(dao_eclip_rewards: Uint128) -> (Uint128, Uint128) {
    let delegator_rewards = ((Decimal::one() - str_to_dec(DAO_TREASURY_REWARDS_FRACTION))
        * u128_to_dec(dao_eclip_rewards))
    .to_uint_floor();
    let dao_treasury_eclip_rewards = dao_eclip_rewards - delegator_rewards;

    (dao_treasury_eclip_rewards, delegator_rewards)
}

/// delegator_rewards = dao_delegator_eclip_rewards * delegator_essence / dao_self_essence                    \
///
/// dao_self_essence = dao_essence - (1 - ELECTOR_ADDITIONAL_ESSENCE_FRACTION) * slacker_essence -              
/// (1 - ELECTOR_BASE_ESSENCE_FRACTION) * elector_self_essence                                                \
///
/// elector_self_essence = (elector_essence - ELECTOR_ADDITIONAL_ESSENCE_FRACTION * slacker_essence) / ELECTOR_BASE_ESSENCE_FRACTION
pub fn calc_delegator_rewards(
    dao_delegators_eclip_rewards: Uint128,
    slacker_essence: Uint128,
    dao_essence: Uint128,
    delegator_essence: Uint128,
    elector_essence: Uint128,
) -> Uint128 {
    let elector_self_essence = (u128_to_dec(elector_essence)
        - str_to_dec(ELECTOR_ADDITIONAL_ESSENCE_FRACTION) * u128_to_dec(slacker_essence))
        / str_to_dec(ELECTOR_BASE_ESSENCE_FRACTION);

    let dao_self_essence = dao_essence
        - ((Decimal::one() - str_to_dec(ELECTOR_ADDITIONAL_ESSENCE_FRACTION))
            * u128_to_dec(slacker_essence))
        .to_uint_floor()
        - ((Decimal::one() - str_to_dec(ELECTOR_BASE_ESSENCE_FRACTION)) * elector_self_essence)
            .to_uint_floor();

    dao_delegators_eclip_rewards * delegator_essence / dao_self_essence
}

pub fn calc_rewards_aggregated_by_denom(
    raw_rewards: &[(Uint128, String)],
) -> Vec<(Uint128, String)> {
    let mut denom_list: Vec<String> = raw_rewards
        .iter()
        .map(|(_, denom)| denom.to_owned())
        .collect();
    denom_list.sort_unstable();
    denom_list.dedup();

    denom_list
        .iter()
        .map(|denom| {
            let amount =
                raw_rewards
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
        .collect()
}

pub fn calc_merged_rewards(
    rewards_a: &[(Uint128, String)],
    rewards_b: &[(Uint128, String)],
) -> Vec<(Uint128, String)> {
    let mut rewards_denom_list: Vec<String> = rewards_a
        .iter()
        .map(|(_, denom)| denom.to_owned())
        .chain(rewards_b.iter().map(|(_, denom)| denom.to_owned()))
        .collect();
    rewards_denom_list.sort_unstable();
    rewards_denom_list.dedup();

    rewards_denom_list
        .iter()
        .map(|rewards_denom| {
            let (amount_a, _) = rewards_a
                .iter()
                .cloned()
                .find(|(_, denom)| denom == rewards_denom)
                .unwrap_or_default();

            let (amount_b, _) = rewards_b
                .iter()
                .cloned()
                .find(|(_, denom)| denom == rewards_denom)
                .unwrap_or_default();

            (amount_a + amount_b, rewards_denom.to_owned())
        })
        .collect()
}

pub fn calc_merged_bribe_allocations(
    astroport_bribe_allocation: &[BribesAllocationItem],
    eclipsepad_bribe_allocation: &[BribesAllocationItem],
) -> Vec<BribesAllocationItem> {
    let mut lp_token_list: Vec<Addr> = astroport_bribe_allocation
        .iter()
        .map(|x| x.lp_token.to_owned())
        .chain(
            eclipsepad_bribe_allocation
                .iter()
                .map(|x| x.lp_token.to_owned()),
        )
        .collect();
    lp_token_list.sort_unstable();
    lp_token_list.dedup();

    lp_token_list
        .iter()
        .map(|lp_token| {
            let astroport_rewards = astroport_bribe_allocation
                .iter()
                .cloned()
                .find(|x| x.lp_token == lp_token)
                .unwrap_or_default()
                .rewards;

            let eclipsepad_rewards = eclipsepad_bribe_allocation
                .iter()
                .cloned()
                .find(|x| x.lp_token == lp_token)
                .unwrap_or_default()
                .rewards;

            BribesAllocationItem {
                lp_token: lp_token.to_owned(),
                rewards: calc_merged_rewards(&astroport_rewards, &eclipsepad_rewards),
            }
        })
        .collect()
}

pub fn calc_merged_pool_info_list_with_rewards(
    pool_info_list_with_rewards_a: &[PoolInfoItem],
    pool_info_list_with_rewards_b: &[PoolInfoItem],
) -> Vec<PoolInfoItem> {
    let mut lp_token_list: Vec<String> = pool_info_list_with_rewards_a
        .iter()
        .map(|x| x.lp_token.to_owned())
        .chain(
            pool_info_list_with_rewards_b
                .iter()
                .map(|x| x.lp_token.to_owned()),
        )
        .collect();
    lp_token_list.sort_unstable();
    lp_token_list.dedup();

    lp_token_list
        .iter()
        .map(|lp_token| {
            let pool_info_a = pool_info_list_with_rewards_a
                .iter()
                .cloned()
                .find(|x| &x.lp_token == lp_token)
                .unwrap_or_default();

            let pool_info_b = pool_info_list_with_rewards_b
                .iter()
                .cloned()
                .find(|x| &x.lp_token == lp_token)
                .unwrap_or_default();

            let weight = if !pool_info_a.weight.is_zero() {
                pool_info_a.weight
            } else {
                pool_info_b.weight
            };

            PoolInfoItem {
                lp_token: lp_token.to_owned(),
                weight,
                rewards: calc_merged_rewards(&pool_info_a.rewards, &pool_info_b.rewards),
            }
        })
        .collect()
}

/// returns (delegator_essence_info, elector_or_slacker_essence_info)
pub fn calc_splitted_user_essence_info(
    essence_info: &EssenceInfo,
    delegator_essence_fraction: Decimal,
) -> (EssenceInfo, EssenceInfo) {
    if delegator_essence_fraction.is_zero() {
        return (EssenceInfo::default(), essence_info.to_owned());
    }

    if delegator_essence_fraction == Decimal::one() {
        return (essence_info.to_owned(), EssenceInfo::default());
    }

    let delegator_essence_info = essence_info.scale(delegator_essence_fraction);
    let elector_or_slacker_essence_info = essence_info.sub(&delegator_essence_info);

    (delegator_essence_info, elector_or_slacker_essence_info)
}

pub fn calculate_claimable(
    xastro: Uint128,
    astro: Uint128,
    total_shares: Uint128,
    total_deposit: Uint128,
    cliamed_xastro: Uint128,
) -> Uint128 {
    xastro // total xASTRO amount
        .multiply_ratio(total_deposit, total_shares) // total ASTRO amount when withdraw all
        .checked_sub(astro) // total deposited ASTRO amount
        .unwrap_or_default()
        .multiply_ratio(total_shares, total_deposit)
        .checked_sub(cliamed_xastro)
        .unwrap_or_default()
}
