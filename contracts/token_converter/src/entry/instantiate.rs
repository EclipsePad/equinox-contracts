use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;
use equinox_msg::token_converter::{Config, InstantiateMsg, RewardConfig};

use crate::{
    error::ContractError,
    state::{CONFIG, CONTRACT_NAME, CONTRACT_VERSION, OWNER, REWARD_CONFIG},
    utils::get_full_denom,
};

use osmosis_std::types::osmosis::tokenfactory::v1beta1 as OsmosisFactory;

/// eclipASTRO information.
const SUB_DENOM: &str = "eclipASTRO";

pub fn try_instantiate(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    REWARD_CONFIG.save(
        deps.storage,
        &RewardConfig {
            users: 8000,         // 80%
            treasury: 1350,      // 20% * 67.5%
            ce_holders: 400,     // 20% * 20%
            stability_pool: 250, // 20% * 12.5%
        },
    )?;

    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.set(deps.branch(), Some(owner))?;

    let creator = env.contract.address;
    let full_denom = get_full_denom(&creator, SUB_DENOM);

    CONFIG.save(
        deps.storage,
        &Config {
            astro: msg.astro,
            xastro: msg.xastro,
            staking_contract: msg.staking_contract,
            eclipastro: full_denom,
            vxastro_holder: None,
            treasury: deps.api.addr_validate(&msg.treasury)?,
            stability_pool: None,
            single_staking_contract: None,
            ce_reward_distributor: None,
        },
    )?;

    let msg: CosmosMsg = OsmosisFactory::MsgCreateDenom {
        sender: creator.to_string(),
        subdenom: SUB_DENOM.to_string(),
    }
    .into();

    Ok(Response::new().add_message(msg))
}
