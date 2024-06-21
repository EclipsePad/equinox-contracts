use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::InstantiateMsg,
    state::{CONTRACT_NAME, OWNER, TOKEN},
    utils::get_full_denom,
    variable::SUB_DENOM,
};

use osmosis_std::types::osmosis::tokenfactory::v1beta1 as OsmosisFactory;

const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn try_instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    OWNER.set(deps.branch(), Some(info.sender))?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let creator = env.contract.address;
    let full_denom = get_full_denom(&creator, SUB_DENOM);

    TOKEN.save(deps.storage, &full_denom)?;

    let msg: CosmosMsg = OsmosisFactory::MsgCreateDenom {
        sender: creator.to_string(),
        subdenom: SUB_DENOM.to_string(),
    }
    .into();

    Ok(Response::new()
        .add_message(msg)
        .add_attributes([("action", "try_instantiate")]))
}
