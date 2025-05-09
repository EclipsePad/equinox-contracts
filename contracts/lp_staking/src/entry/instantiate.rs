use cosmwasm_std::{ensure, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;
use equinox_msg::{
    lp_staking::{Config, InstantiateMsg},
    utils::has_unique_elements,
};

use crate::{
    config::DEFAULT_REWARD_DISTRIBUTION,
    entry::query::check_native_token_denom,
    error::ContractError,
    state::{BLACK_LIST, CONFIG, CONTRACT_NAME, CONTRACT_VERSION, OWNER, REWARD_DISTRIBUTION},
};

pub fn try_instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    msg.lp_token.check(deps.api)?;
    ensure!(
        check_native_token_denom(&deps.querier, msg.astro.clone()).unwrap_or_default(),
        ContractError::InvalidDenom(msg.astro)
    );
    ensure!(
        check_native_token_denom(&deps.querier, msg.xastro.clone()).unwrap_or_default(),
        ContractError::InvalidDenom(msg.xastro)
    );
    // update config
    CONFIG.save(
        deps.storage,
        &Config {
            lp_token: msg.lp_token,
            lp_contract: deps.api.addr_validate(msg.lp_contract.as_str())?,
            astro: msg.astro,
            xastro: msg.xastro,
            astro_staking: deps.api.addr_validate(msg.astro_staking.as_str())?,
            eclip_staking: deps.api.addr_validate(msg.eclip_staking.as_str())?,
            lockdrop: msg
                .lockdrop
                .map(|x| deps.api.addr_validate(&x))
                .transpose()?
                .unwrap_or(info.sender.clone()),
            astroport_incentives: deps.api.addr_validate(msg.astroport_incentives.as_str())?,
            treasury: deps.api.addr_validate(msg.treasury.as_str())?,
            funding_dao: deps.api.addr_validate(msg.funding_dao.as_str())?,
            eclip: msg.eclip,
            beclip: deps.api.addr_validate(&msg.beclip)?,
        },
    )?;
    REWARD_DISTRIBUTION.save(deps.storage, &DEFAULT_REWARD_DISTRIBUTION)?;
    // update owner
    let owner = deps
        .api
        .addr_validate(msg.owner.unwrap_or(info.sender.to_string()).as_str())?;
    OWNER.set(deps.branch(), Some(owner))?;

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

    Ok(Response::new())
}
