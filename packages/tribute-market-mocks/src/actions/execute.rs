use astroport_governance::emissions_controller::hub::{UserInfoResponse, VotedPoolInfo};
use cosmwasm_std::{
    coin, Addr, BankMsg, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Uint128,
};

use eclipse_base::{converters::u128_to_dec, error::ContractError};
use equinox_msg::voter::types::{BribesAllocationItem, PoolInfoItem};

use crate::{
    state::{BRIBES_ALLOCATION, CONFIG, INSTANTIATION_DATE, REWARDS, REWARDS_DISTRIBUTION_DELAY},
    types::Config,
};

pub fn try_set_bribes_allocation(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    bribes_allocation: Vec<BribesAllocationItem>,
) -> Result<Response, ContractError> {
    BRIBES_ALLOCATION.save(deps.storage, &bribes_allocation)?;

    Ok(Response::new().add_attribute("action", "try_set_bribes_allocation"))
}

pub fn try_allocate_rewards(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    users: Vec<String>,
) -> Result<Response, ContractError> {
    let Config {
        astroport_emission_controller,
        astroport_voting_escrow,
    } = CONFIG.load(deps.storage)?;

    // get user bribes allocation:
    // 1) query tribute market bribes allocation
    let tribute_market_bribe_allocation = BRIBES_ALLOCATION.load(deps.storage)?;

    for user in users {
        // 2) query user voting power
        let user_voting_power = deps.querier.query_wasm_smart::<Uint128>(
            astroport_voting_escrow.clone(),
            &astroport_governance::voting_escrow::QueryMsg::UserVotingPower {
                user: user.clone(),
                timestamp: None,
            },
        )?;
        let user_voting_power_decimal = u128_to_dec(user_voting_power);

        // 3) get voter to tribute market voting power ratio allocation
        let user_to_tribute_voting_power_ratio_allocation = deps
            .querier
            .query_wasm_smart::<UserInfoResponse>(
                astroport_emission_controller.clone(),
                &astroport_governance::emissions_controller::hub::QueryMsg::UserInfo {
                    user: user.clone(),
                    timestamp: None,
                },
            )?
            .applied_votes
            .iter()
            .map(|(lp_token, weight)| -> StdResult<(String, Decimal)> {
                let tribute_market_voting_power = deps
                    .querier
                    .query_wasm_smart::<VotedPoolInfo>(
                        astroport_emission_controller.clone(),
                        &astroport_governance::emissions_controller::hub::QueryMsg::VotedPool {
                            pool: lp_token.to_owned(),
                            timestamp: None,
                        },
                    )?
                    .voting_power;

                let ratio = if tribute_market_voting_power.is_zero() {
                    Decimal::zero()
                } else {
                    user_voting_power_decimal * weight / u128_to_dec(tribute_market_voting_power)
                };

                Ok((lp_token.to_owned(), ratio))
            })
            .collect::<StdResult<Vec<(String, Decimal)>>>()?;

        // 4) get rewards
        let pool_info_list: Vec<PoolInfoItem> = user_to_tribute_voting_power_ratio_allocation
            .iter()
            .cloned()
            .map(|(lp_token, weight)| PoolInfoItem {
                lp_token,
                weight,
                rewards: vec![],
            })
            .collect();

        let pool_info_list_with_rewards = calc_pool_info_list_with_rewards(
            &pool_info_list,
            &tribute_market_bribe_allocation,
            &user_to_tribute_voting_power_ratio_allocation,
        );

        let rewards_raw: Vec<(Uint128, String)> = pool_info_list_with_rewards
            .iter()
            .map(|x| x.rewards.clone())
            .flatten()
            .collect();

        let mut denom_list: Vec<String> = rewards_raw
            .iter()
            .map(|(_, denom)| denom.to_owned())
            .collect();
        denom_list.sort_unstable();
        denom_list.dedup();

        let rewards: Vec<(Uint128, String)> = denom_list
            .iter()
            .map(|denom| {
                let amount =
                    rewards_raw
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

        REWARDS.save(deps.storage, &Addr::unchecked(user), &rewards)?;
    }

    Ok(Response::new().add_attribute("action", "try_deposit_rewards"))
}

pub fn try_claim_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let block_time = env.block.time.seconds();
    let instantiation_date = INSTANTIATION_DATE.load(deps.storage)?;

    if block_time < instantiation_date + REWARDS_DISTRIBUTION_DELAY {
        Err(StdError::generic_err("Rewards are not distributed!"))?;
    }

    let msg: CosmosMsg = CosmosMsg::Bank(BankMsg::Send {
        to_address: sender.to_string(),
        amount: REWARDS
            .load(deps.storage, sender)?
            .into_iter()
            .map(|(amount, denom)| coin(amount.u128(), denom))
            .collect(),
    });

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "try_claim_rewards"))
}

/// voter_bribe_allocation = tribute_market_bribe_allocation * voter_voting_power_allocation / tribute_market_voting_power_allocation
fn calc_pool_info_list_with_rewards(
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
