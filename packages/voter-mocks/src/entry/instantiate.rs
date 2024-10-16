use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response, StdResult, Uint128};
use cw2::set_contract_version;

use eclipse_base::{
    error::ContractError,
    voter::{
        msg::InstantiateMsg,
        state::{
            ADDRESS_CONFIG, ASTRO_STAKING_REWARD_CONFIG, CONTRACT_NAME, DAO_ESSENCE_ACC,
            DAO_WEIGHTS_ACC, DATE_CONFIG, ECLIP_ASTRO_MINTED_BY_VOTER, ELECTOR_ESSENCE_ACC,
            ELECTOR_WEIGHTS_ACC, EPOCH_COUNTER, IS_PAUSED, REWARDS_CLAIM_STAGE,
            SLACKER_ESSENCE_ACC, SWAP_REWARDS_REPLY_ID_CNT, TEMPORARY_REWARDS, TOKEN_CONFIG,
            TRANSFER_ADMIN_STATE, VOTE_RESULTS,
        },
        types::{
            AddressConfig, AstroStakingRewardConfig, DateConfig, EpochInfo, EssenceInfo,
            RewardsClaimStage, TokenConfig, TransferAdminState,
        },
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

    SWAP_REWARDS_REPLY_ID_CNT.save(deps.storage, &0)?;
    IS_PAUSED.save(deps.storage, &false)?;
    REWARDS_CLAIM_STAGE.save(deps.storage, &RewardsClaimStage::default())?;
    ECLIP_ASTRO_MINTED_BY_VOTER.save(deps.storage, &Uint128::zero())?;

    ADDRESS_CONFIG.save(
        deps.storage,
        &AddressConfig {
            admin: sender.clone(),
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
            eclipse_single_sided_vault: msg
                .eclipse_single_sided_vault
                .map(|x| deps.api.addr_validate(&x))
                .transpose()?,
            astroport_staking: deps.api.addr_validate(&msg.astroport_staking)?,
            astroport_assembly: deps.api.addr_validate(&msg.astroport_assembly)?,
            astroport_voting_escrow: deps.api.addr_validate(&msg.astroport_voting_escrow)?,
            // ignored
            astroport_emission_controller: deps
                .api
                .addr_validate(&msg.astroport_emission_controller)?,
            astroport_router: deps.api.addr_validate(&msg.astroport_router)?,
            astroport_tribute_market: msg
                .astroport_tribute_market
                .map(|x| deps.api.addr_validate(&x))
                .transpose()?,
        },
    )?;

    TOKEN_CONFIG.save(
        deps.storage,
        &TokenConfig {
            eclip: msg.eclip,
            astro: msg.astro,
            xastro: msg.xastro,
            eclip_astro: msg.eclip_astro,
        },
    )?;

    DATE_CONFIG.save(
        deps.storage,
        &DateConfig {
            genesis_epoch_start_date: msg.genesis_epoch_start_date,
            epoch_length: msg.epoch_length,
            vote_delay: msg.vote_delay,
        },
    )?;

    TRANSFER_ADMIN_STATE.save(
        deps.storage,
        &TransferAdminState {
            new_admin: sender,
            deadline: block_time,
        },
    )?;

    ELECTOR_WEIGHTS_ACC.save(deps.storage, &vec![])?;
    ELECTOR_ESSENCE_ACC.save(deps.storage, &EssenceInfo::default())?;

    DAO_WEIGHTS_ACC.save(deps.storage, &vec![])?;
    DAO_ESSENCE_ACC.save(deps.storage, &EssenceInfo::default())?;

    SLACKER_ESSENCE_ACC.save(deps.storage, &EssenceInfo::default())?;

    VOTE_RESULTS.save(deps.storage, &vec![])?;

    TEMPORARY_REWARDS.save(deps.storage, &Uint128::zero())?;

    EPOCH_COUNTER.save(
        deps.storage,
        &EpochInfo {
            start_date: msg.genesis_epoch_start_date,
            id: 1,
        },
    )?;

    ASTRO_STAKING_REWARD_CONFIG.save(
        deps.storage,
        &AstroStakingRewardConfig {
            users: 8000,
            treasury: 2000,
        },
    )?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}
