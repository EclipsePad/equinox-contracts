use std::str::FromStr;

use cosmwasm_std::{Decimal, DepsMut, Env, MessageInfo, Response, Uint128};
use cw2::set_contract_version;

use eclipse_base::{
    error::ContractError,
    staking::{
        msg::InstantiateMsg,
        state::{
            BECLIP_SUPPLY, CONFIG, CONTRACT_NAME, DAO_TREASURY_ADDRESS, DECREASING_REWARDS_DATE,
            DECREASING_REWARDS_PERIOD, ECLIP_MAINNET, ECLIP_PER_SECOND,
            ECLIP_PER_SECOND_MULTIPLIER, IS_PAUSED, LOCK_STATES, PAGINATION_AMOUNT,
            PAGINATION_CONFIG, PAGINATION_INDEX, PENALTY_MULTIPLIER, PERIOD_TIER_0, PERIOD_TIER_1,
            PERIOD_TIER_2, PERIOD_TIER_3, PERIOD_TIER_4, REWARDS_TIER_0, REWARDS_TIER_1,
            REWARDS_TIER_2, REWARDS_TIER_3, REWARDS_TIER_4, SECONDS_PER_ESSENCE, STAKE_STATE,
            TOTAL_LOCKING_ESSENCE, TOTAL_STAKING_ESSENCE_COMPONENTS, TRANSFER_ADMIN_STATE,
        },
        types::{Config, PaginationConfig, State, TransferAdminState},
    },
};

const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn try_instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let sender = info.sender;
    let block_time = env.block.time.seconds();
    let default_staking_token = ECLIP_MAINNET.to_string();

    let default_lock_schedule = &vec![
        (PERIOD_TIER_0, REWARDS_TIER_0),
        (PERIOD_TIER_1, REWARDS_TIER_1),
        (PERIOD_TIER_2, REWARDS_TIER_2),
        (PERIOD_TIER_3, REWARDS_TIER_3),
        (PERIOD_TIER_4, REWARDS_TIER_4),
    ];

    let default_seconds_per_essence = Uint128::from(SECONDS_PER_ESSENCE);

    let lock_count = msg
        .lock_schedule
        .clone()
        .map_or(default_lock_schedule.len(), |x| x.len());

    let dao_treasury_address = deps.api.addr_validate(
        &msg.dao_treasury_address
            .unwrap_or(DAO_TREASURY_ADDRESS.to_string()),
    )?;

    let mut pagination_config = msg.pagintaion_config.unwrap_or(PaginationConfig::new(
        PAGINATION_AMOUNT,
        &PAGINATION_INDEX,
        &PAGINATION_INDEX,
    ));
    pagination_config.staking_pagination_index = PAGINATION_INDEX; // reset pagination counter
    pagination_config.locking_pagination_index = PAGINATION_INDEX; // reset pagination counter

    PAGINATION_CONFIG.save(deps.storage, &pagination_config)?;

    TRANSFER_ADMIN_STATE.save(
        deps.storage,
        &TransferAdminState {
            new_admin: sender.clone(),
            deadline: block_time,
        },
    )?;

    CONFIG.save(
        deps.storage,
        &Config {
            admin: sender,
            equinox_voter: msg
                .equinox_voter
                .map(|x| deps.api.addr_validate(&x))
                .transpose()?,
            beclip_minter: msg
                .beclip_minter
                .map(|x| deps.api.addr_validate(&x))
                .transpose()?,
            staking_token: msg.staking_token.unwrap_or(default_staking_token),
            beclip_address: msg
                .beclip_address
                .map(|x| deps.api.addr_validate(&x))
                .transpose()?,
            beclip_whitelist: msg
                .beclip_whitelist
                .map(|x| x.iter().map(|y| deps.api.addr_validate(y)).collect())
                .transpose()?
                .unwrap_or_default(),
            lock_schedule: msg
                .lock_schedule
                .unwrap_or(default_lock_schedule.to_owned()),
            seconds_per_essence: msg
                .seconds_per_essence
                .unwrap_or(default_seconds_per_essence),
            dao_treasury_address,
            penalty_multiplier: msg
                .penalty_multiplier
                .unwrap_or(Decimal::from_str(PENALTY_MULTIPLIER)?),
            eclip_per_second: msg.eclip_per_second.unwrap_or(ECLIP_PER_SECOND),
            eclip_per_second_multiplier: msg
                .eclip_per_second_multiplier
                .unwrap_or(Decimal::from_str(ECLIP_PER_SECOND_MULTIPLIER)?),
        },
    )?;

    IS_PAUSED.save(deps.storage, &false)?;

    STAKE_STATE.save(
        deps.storage,
        &State {
            total_bond_amount: Uint128::zero(),
            distributed_rewards_per_tier: 0,
        },
    )?;

    LOCK_STATES.save(
        deps.storage,
        &vec![
            State {
                total_bond_amount: Uint128::zero(),
                distributed_rewards_per_tier: 0,
            };
            lock_count
        ],
    )?;

    BECLIP_SUPPLY.save(deps.storage, &Uint128::zero())?;

    TOTAL_STAKING_ESSENCE_COMPONENTS.save(deps.storage, &(Uint128::zero(), Uint128::zero()))?;
    TOTAL_LOCKING_ESSENCE.save(deps.storage, &Uint128::zero())?;

    DECREASING_REWARDS_DATE.save(
        deps.storage,
        &(env.block.time.seconds() + DECREASING_REWARDS_PERIOD),
    )?;

    Ok(Response::new().add_attributes([("action", "try_instantiate")]))
}
