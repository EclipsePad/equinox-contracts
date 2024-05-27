use cosmwasm_std::{coin, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Response};
use cw_utils::nonpayable;

use osmosis_std::types::osmosis::tokenfactory::v1beta1 as OsmosisFactory;

use crate::{
    error::ContractError,
    state::{LAST_CLAIMED, OWNER, TOKEN},
};

pub fn try_claim(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let sender_address = &info.sender;
    let is_admin = OWNER
        .is_admin(deps.as_ref(), sender_address)
        .unwrap_or_default();
    let now_in_seconds = env.block.time.seconds();
    let creator = &env.contract.address;
    let denom = TOKEN.load(deps.storage)?;
    let mut amount = coin(10_000_000_000, denom.clone());
    if is_admin {
        amount = coin(1_000_000_000_000, denom);
    }

    let last_claimed = LAST_CLAIMED
        .load(deps.storage, sender_address)
        .unwrap_or_default();

    if now_in_seconds < last_claimed + 3600 && !is_admin {
        Err(cosmwasm_std::StdError::GenericErr {
            msg: "Come back later".to_owned(),
        })?;
    }

    LAST_CLAIMED.save(deps.storage, sender_address, &now_in_seconds)?;

    let msg_list = vec![
        OsmosisFactory::MsgMint {
            sender: creator.to_string(),
            amount: Some(amount.clone().into()),
            mint_to_address: creator.to_string(),
        }
        .into(),
        CosmosMsg::Bank(BankMsg::Send {
            to_address: sender_address.to_string(),
            amount: vec![amount],
        }),
    ];

    Ok(Response::new()
        .add_messages(msg_list)
        .add_attributes([("action", "try_claim")]))
}
