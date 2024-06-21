use cosmwasm_std::{coin, ensure, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Response};
use cw_utils::nonpayable;

use osmosis_std::types::osmosis::tokenfactory::v1beta1 as OsmosisFactory;

use crate::{
    error::ContractError,
    state::{LAST_CLAIMED, OWNER, TOKEN},
    variable::{CLAIM_DURATION, DEFAULT_ADMIN_CLAIM_AMOUNT, DEFAULT_USER_CLAIM_AMOUNT},
};

pub fn try_claim(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let sender_address = &info.sender;
    let is_admin = OWNER
        .is_admin(deps.as_ref(), sender_address)
        .unwrap_or_default();
    let last_claimed = LAST_CLAIMED
        .load(deps.storage, sender_address)
        .unwrap_or_default();
    let denom = TOKEN.load(deps.storage)?;

    let now_in_seconds = env.block.time.seconds();
    let creator = &env.contract.address;
    let claimable = now_in_seconds < last_claimed + CLAIM_DURATION;

    let mut amount = coin(DEFAULT_USER_CLAIM_AMOUNT, denom.clone());
    if is_admin {
        amount = coin(DEFAULT_ADMIN_CLAIM_AMOUNT, denom);
    }

    if !is_admin {
        ensure!(claimable, ContractError::ClaimDurationError {});
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
