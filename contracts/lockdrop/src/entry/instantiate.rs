use std::str::FromStr;

use astroport::asset::AssetInfo;
use cosmwasm_std::{ensure, Decimal, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;
use equinox_msg::{
    lockdrop::{Config, InstantiateMsg, LockConfig, LpLockupState, SingleLockupState},
    utils::has_unique_elements,
};

use crate::{
    config::{
        DEFAULT_DEPOSIT_WINDOW, DEFAULT_INIT_EARLY_UNLOCK_PENALTY, DEFAULT_LOCK_CONFIGS, DEFAULT_REWARD_DISTRIBUTION_CONFIG, DEFAULT_WITHDRAW_WINDOW, MINIMUM_WINDOW
    },
    entry::execute::check_native_token_denom,
    error::ContractError,
    state::{
        BLACK_LIST, CONFIG, CONTRACT_NAME, CONTRACT_VERSION, LP_LOCKUP_STATE, OWNER, REWARD_DISTRIBUTION_CONFIG, SINGLE_LOCKUP_STATE
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
    ensure!(
        msg.init_timestamp >= env.block.time.seconds(),
        ContractError::InvalidInitTimestamp(env.block.time.seconds())
    );

    ensure!(
        check_native_token_denom(&deps.querier, msg.astro_token.clone()).unwrap_or_default(),
        ContractError::InvalidDenom(msg.astro_token)
    );
    ensure!(
        check_native_token_denom(&deps.querier, msg.xastro_token.clone()).unwrap_or_default(),
        ContractError::InvalidDenom(msg.xastro_token)
    );
    ensure!(
        check_native_token_denom(&deps.querier, msg.eclip.clone()).unwrap_or_default(),
        ContractError::InvalidDenom(msg.eclip)
    );
    if let Some(mut lock_configs) = msg.lock_configs.clone() {
        lock_configs.sort_by(|a, b| a.duration.cmp(&b.duration));
        let mut prev_lock_config: Option<LockConfig> = None;
        for lock_config in lock_configs {
            ensure!(
                lock_config.multiplier != 0,
                ContractError::InvalidMultiplier(lock_config.multiplier)
            );
            if prev_lock_config.is_some() {
                ensure!(
                    lock_config
                        .duration
                        .gt(&prev_lock_config.clone().unwrap().duration),
                    ContractError::InvalidLockConfig {}
                );
                ensure!(
                    lock_config
                        .multiplier
                        .ge(&prev_lock_config.unwrap().multiplier),
                    ContractError::InvalidLockConfig {}
                );
            }
            prev_lock_config = Some(lock_config);
        }
    }

    if let Some(deposit_window) = msg.deposit_window {
        ensure!(
            deposit_window.ge(&MINIMUM_WINDOW),
            ContractError::InvalidTimeWindow(deposit_window)
        );
    }
    if let Some(withdrawal_window) = msg.withdrawal_window {
        ensure!(
            withdrawal_window.ge(&MINIMUM_WINDOW),
            ContractError::InvalidTimeWindow(withdrawal_window)
        );
    }

    let config = Config {
        astro_token: msg.astro_token,
        xastro_token: msg.xastro_token,
        beclip: AssetInfo::Token {
            contract_addr: deps.api.addr_validate(&msg.beclip)?,
        },
        eclip: AssetInfo::NativeToken { denom: msg.eclip },
        eclipastro_token: None,
        voter: None,
        eclip_staking: Some(deps.api.addr_validate(&msg.eclip_staking)?),
        single_sided_staking: None,
        lp_staking: None,
        liquidity_pool: None,
        lp_token: None,
        dao_treasury_address: None,
        lock_configs: msg.lock_configs.unwrap_or(DEFAULT_LOCK_CONFIGS.to_vec()),
        init_timestamp: msg.init_timestamp,
        deposit_window: msg.deposit_window.unwrap_or(DEFAULT_DEPOSIT_WINDOW),
        withdrawal_window: msg.withdrawal_window.unwrap_or(DEFAULT_WITHDRAW_WINDOW),
        astro_staking: deps.api.addr_validate(&msg.astro_staking)?,
        claims_allowed: false,
        countdown_start_at: 0u64,
        init_early_unlock_penalty: msg.init_early_unlock_penalty.unwrap_or(Decimal::from_str(&DEFAULT_INIT_EARLY_UNLOCK_PENALTY).unwrap_or_default())
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

    // check and update blacklist
    if let Some(blacklist) = msg.blacklist {
        ensure!(
            has_unique_elements(blacklist.clone()),
            ContractError::DuplicatedAssets {}
        );
        // validate each is correct address
        let _ = blacklist.iter().map(|b| deps.api.addr_validate(b).unwrap());
        BLACK_LIST.save(deps.storage, &blacklist)?;
    }
    Ok(Response::default())
}
