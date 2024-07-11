use cosmwasm_std::{
    coin, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, Uint128,
};

use eclipse_base::error::ContractError;

use crate::state::{
    CLAIMABLE_REWARDS_PER_TX, INSTANTIATION_DATE, REWARDS, REWARDS_DISTRIBUTION_DELAY,
    REWARDS_DIVIDER,
};

pub fn try_deposit_rewards(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let divider = Uint128::new(REWARDS_DIVIDER);
    let claimable_rewards: Vec<(String, Uint128)> = info
        .funds
        .into_iter()
        .map(|x| (x.denom, x.amount / divider))
        .collect();
    let rewards: Vec<(String, Uint128)> = claimable_rewards
        .iter()
        .cloned()
        .map(|(denom, amount)| (denom, amount * divider))
        .collect();

    CLAIMABLE_REWARDS_PER_TX.save(deps.storage, &claimable_rewards)?;
    REWARDS.save(deps.storage, &rewards)?;

    Ok(Response::new().add_attribute("action", "try_deposit_rewards"))
}

pub fn try_claim_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let sender = &info.sender;
    let block_time = env.block.time.seconds();
    let instantiation_date = INSTANTIATION_DATE.load(deps.storage)?;

    if block_time < instantiation_date + REWARDS_DISTRIBUTION_DELAY {
        Err(StdError::generic_err("Rewards are not distributed!"))?;
    }

    let msg: CosmosMsg = CosmosMsg::Bank(BankMsg::Send {
        to_address: sender.to_string(),
        amount: CLAIMABLE_REWARDS_PER_TX
            .load(deps.storage)?
            .into_iter()
            .map(|(denom, amount)| coin(amount.u128(), denom))
            .collect(),
    });

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "try_claim_rewards"))
}
