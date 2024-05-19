use astroport::{asset::PairInfo, pair::QueryMsg as AstroportPairQueryMsg};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};
use cw2::set_contract_version;
use equinox_msg::lockdrop::{Config, InstantiateMsg, LpLockupState, SingleLockupState};

use crate::{
    config::{
        DEFAULT_DEPOSIT_WINDOW, DEFAULT_LOCK_CONFIGS, DEFAULT_REWARD_DISTRIBUTION_CONFIG,
        DEFAULT_WITHDRAW_WINDOW,
    },
    error::ContractError,
    state::{
        CONFIG, CONTRACT_NAME, CONTRACT_VERSION, LP_LOCKUP_STATE, OWNER,
        REWARD_DISTRIBUTION_CONFIG, SINGLE_LOCKUP_STATE,
    },
};

pub fn try_instantiate(
    mut deps: DepsMut,
    _env: Env,
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
        astro_token: msg.astro_token,
        xastro_token: msg.xastro_token,
        beclip: msg.beclip,
        eclipastro_token: msg.eclipastro_token,
        converter: msg.converter,
        single_sided_staking: msg.single_sided_staking,
        lp_staking: msg.lp_staking,
        liquidity_pool: msg.liquidity_pool,
        lp_token: pool_info.liquidity_token,
        dao_treasury_address: msg.dao_treasury_address,
        lock_configs: msg.lock_configs.unwrap_or(DEFAULT_LOCK_CONFIGS.to_vec()),
        init_timestamp: msg.init_timestamp,
        deposit_window: msg.deposit_window.unwrap_or(DEFAULT_DEPOSIT_WINDOW),
        withdrawal_window: msg.withdrawal_window.unwrap_or(DEFAULT_WITHDRAW_WINDOW),
        lockdrop_incentives: Uint128::zero(),
        astro_staking: msg.astro_staking,
        claims_allowed: false,
        countdown_start_at: 0u64,
    };

    REWARD_DISTRIBUTION_CONFIG.save(deps.storage, &DEFAULT_REWARD_DISTRIBUTION_CONFIG)?;

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
