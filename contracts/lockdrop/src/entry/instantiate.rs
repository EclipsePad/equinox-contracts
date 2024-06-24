use astroport::{asset::PairInfo, pair::QueryMsg as AstroportPairQueryMsg};
use cosmwasm_std::{ensure, Addr, DepsMut, Env, MessageInfo, Response, Uint128};
use cw2::set_contract_version;
use equinox_msg::lockdrop::{
    Config, InstantiateMsg, LockConfig, LpLockupState, RewardDistributionConfig, SingleLockupState,
};

use crate::{
    error::ContractError,
    state::{
        CONFIG, CONTRACT_NAME, CONTRACT_VERSION, LP_LOCKUP_STATE, OWNER,
        REWARD_DISTRIBUTION_CONFIG, SINGLE_LOCKUP_STATE,
    },
};

pub fn try_instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // CHECK :: init_timestamp needs to be valid
    // ensure!(
    //     msg.init_timestamp >= env.block.time.seconds(),
    //     ContractError::InvalidInitTimestamp(env.block.time.seconds())
    // );

    let pool_info: PairInfo = deps
        .querier
        .query_wasm_smart(&msg.liquidity_pool, &AstroportPairQueryMsg::Pair {})?;

    let config = Config {
        astro_token: deps.api.addr_validate(&msg.astro_token)?,
        xastro_token: deps.api.addr_validate(&msg.xastro_token)?,
        eclip: msg.eclip,
        eclipastro_token: deps.api.addr_validate(&msg.eclipastro_token)?,
        converter: deps.api.addr_validate(&msg.converter)?,
        flexible_staking: None,
        timelock_staking: None,
        lp_staking: None,
        reward_distributor: None,
        liquidity_pool: deps.api.addr_validate(&msg.liquidity_pool)?,
        lp_token: Addr::unchecked(pool_info.liquidity_token),
        dao_treasury_address: deps.api.addr_validate(&msg.dao_treasury_address)?,
        lock_configs: msg.lock_configs.unwrap_or(vec![
            LockConfig {
                duration: 0,
                multiplier: 1,
                early_unlock_penalty_bps: 5000,
            },
            LockConfig {
                duration: 86400 * 30,
                multiplier: 2,
                early_unlock_penalty_bps: 5000,
            },
            LockConfig {
                duration: 86400 * 30 * 3,
                multiplier: 6,
                early_unlock_penalty_bps: 5000,
            },
            LockConfig {
                duration: 86400 * 30 * 6,
                multiplier: 12,
                early_unlock_penalty_bps: 5000,
            },
            LockConfig {
                duration: 86400 * 30 * 9,
                multiplier: 18,
                early_unlock_penalty_bps: 5000,
            },
            LockConfig {
                duration: 86400 * 365,
                multiplier: 24,
                early_unlock_penalty_bps: 5000,
            },
        ]),
        init_timestamp: msg.init_timestamp,
        deposit_window: msg.deposit_window.unwrap_or(86400 * 5),
        withdrawal_window: msg.withdrawal_window.unwrap_or(86400 * 2),
        lockdrop_incentives: Uint128::zero(),
        astro_staking: deps.api.addr_validate(&msg.astro_staking)?,
    };

    REWARD_DISTRIBUTION_CONFIG.save(
        deps.storage,
        &RewardDistributionConfig {
            instant: 3000,           // bps
            vesting_period: 7776000, // 3 months
        },
    )?;

    let owner = msg
        .owner
        .map(|v| deps.api.addr_validate(&v))
        .transpose()?
        .unwrap_or(info.sender);
    OWNER.set(deps.branch(), Some(owner))?;

    CONFIG.save(deps.storage, &config)?;
    SINGLE_LOCKUP_STATE.save(deps.storage, &SingleLockupState::default())?;
    LP_LOCKUP_STATE.save(deps.storage, &LpLockupState::default())?;
    Ok(Response::default())
}
