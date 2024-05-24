use cosmwasm_std::{coin, Addr, DepsMut, Env, MessageInfo, Response, Uint128};

use cw_utils::nonpayable;
use osmosis_std::types::osmosis::tokenfactory::v1beta1 as OsmosisFactory;

use crate::{
    error::ContractError,
    state::{OWNER, TOKEN},
};

pub fn update_owner(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_owner: Addr,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    OWNER.set(deps.branch(), Some(new_owner.clone()))?;
    Ok(Response::new()
        .add_attribute("action", "update owner")
        .add_attribute("to", new_owner.to_string()))
}

pub fn try_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let creator = &env.contract.address;
    let denom = TOKEN.load(deps.storage)?;
    let amount = coin(amount.u128(), denom);

    Ok(Response::new()
        .add_message(OsmosisFactory::MsgMint {
            sender: creator.to_string(),
            amount: Some(amount.clone().into()),
            mint_to_address: info.sender.to_string(),
        })
        .add_attributes([
            ("action", "mint"),
            ("amount", &amount.to_string()),
            ("recipient", &info.sender.as_ref()),
        ]))
}
