use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use equinox_msg::voter::{
    AddressConfig, DateConfig, EssenceInfo, InstantiateMsg, TokenConfig, TransferAdminState,
};

use crate::{
    error::ContractError,
    state::{
        ADDRESS_CONFIG, CONTRACT_NAME, DAO_ESSENCE, DAO_WEIGHTS, DATE_CONFIG, ELECTOR_VOTES,
        EPOCH_ID, TOKEN_CONFIG, TOTAL_VOTES, TRANSFER_ADMIN_STATE, VOTE_RESULTS,
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

    TRANSFER_ADMIN_STATE.save(
        deps.storage,
        &TransferAdminState {
            new_admin: sender.clone(),
            deadline: block_time,
        },
    )?;

    ADDRESS_CONFIG.save(
        deps.storage,
        &AddressConfig {
            admin: sender,
            worker_list: msg
                .worker_list
                .map(|x| {
                    x.iter()
                        .map(|y| deps.api.addr_validate(y))
                        .collect::<StdResult<Vec<Addr>>>()
                })
                .transpose()?
                .unwrap_or_default(),
            eclipse_dao: deps.api.addr_validate(&msg.eclipse_dao)?,
            eclipsepad_foundry: msg
                .eclipsepad_foundry
                .map(|x| deps.api.addr_validate(&x))
                .transpose()?,
            eclipsepad_minter: deps.api.addr_validate(&msg.eclipsepad_minter)?,
            eclipsepad_staking: deps.api.addr_validate(&msg.eclipsepad_staking)?,
            eclipsepad_tribute_market: msg
                .eclipsepad_tribute_market
                .map(|x| deps.api.addr_validate(&x))
                .transpose()?,
            astroport_staking: deps.api.addr_validate(&msg.astroport_staking)?,
            astroport_assembly: deps.api.addr_validate(&msg.astroport_assembly)?,
            astroport_voting_escrow: deps.api.addr_validate(&msg.astroport_voting_escrow)?,
            astroport_emission_controller: deps
                .api
                .addr_validate(&msg.astroport_emission_controller)?,
            astroport_tribute_market: msg
                .astroport_tribute_market
                .map(|x| deps.api.addr_validate(&x))
                .transpose()?,
        },
    )?;

    TOKEN_CONFIG.save(
        deps.storage,
        &TokenConfig {
            astro: msg.astro,
            xastro: msg.xastro,
            eclip_astro: msg.eclip_astro,
        },
    )?;

    DATE_CONFIG.save(
        deps.storage,
        &DateConfig {
            epochs_start: msg.epochs_start,
            epoch_length: msg.epoch_length,
            vote_cooldown: msg.vote_cooldown,
            vote_delay: msg.vote_delay,
        },
    )?;

    DAO_WEIGHTS.save(deps.storage, &vec![])?;
    DAO_ESSENCE.save(deps.storage, &EssenceInfo::default())?;
    ELECTOR_VOTES.save(deps.storage, &vec![])?;
    TOTAL_VOTES.save(deps.storage, &vec![])?;
    EPOCH_ID.save(deps.storage, &1)?;
    VOTE_RESULTS.save(deps.storage, &vec![])?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}
