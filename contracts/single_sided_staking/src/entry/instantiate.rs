use std::str::FromStr;

use cosmwasm_std::{ensure, Decimal, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;

use crate::{
    config::{DEFAULT_INIT_EARLY_UNLOCK_PENALTY, DEFAULT_TIMELOCK_CONFIG},
    error::ContractError,
    state::{BLACK_LIST, CONFIG, CONTRACT_NAME, CONTRACT_VERSION, OWNER},
};
use equinox_msg::{
    single_sided_staking::{Config, InstantiateMsg},
    utils::has_unique_elements,
};

pub fn try_instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(
        deps.storage,
        &Config {
            token: msg.token,
            timelock_config: msg
                .timelock_config
                .unwrap_or(DEFAULT_TIMELOCK_CONFIG.to_vec()),
            voter: deps.api.addr_validate(&msg.voter)?,
            lockdrop: msg
                .lockdrop
                .map(|x| deps.api.addr_validate(&x))
                .transpose()?
                .unwrap_or(info.sender),
            treasury: deps.api.addr_validate(&msg.treasury)?,
            eclip_staking: deps.api.addr_validate(&msg.eclip_staking)?,
            eclip: msg.eclip,
            beclip: deps.api.addr_validate(&msg.beclip)?,
            init_early_unlock_penalty: msg.init_early_unlock_penalty.unwrap_or(
                Decimal::from_str(DEFAULT_INIT_EARLY_UNLOCK_PENALTY).unwrap_or_default(),
            ),
        },
    )?;

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

    let owner = deps.api.addr_validate(msg.owner.as_str())?;
    OWNER.set(deps.branch(), Some(owner))?;
    Ok(Response::new().add_attributes([("action", "instantiate token converter")]))
}
