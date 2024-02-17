use cosmwasm_std::{
    ensure, ensure_eq, from_json, to_json_binary, DepsMut, Env, MessageInfo, Response, Uint128,
    WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::{
    error::ContractError,
    state::{CONFIG, OWNER, STAKING, TOTAL_STAKING},
};
use equinox_msg::{
    flexible_staking::{Cw20HookMsg, UpdateConfigMsg},
    reward_distributor::ExecuteMsg as RewardDistributorExecuteMsg,
};

/// Update config
pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_config: UpdateConfigMsg,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let mut config = CONFIG.load(deps.storage)?;
    let mut res: Response = Response::new().add_attribute("action", "update config");
    if let Some(token) = new_config.token {
        config.token = deps.api.addr_validate(&token)?;
        res = res.add_attribute("token", token);
    }
    if let Some(reward_contract) = new_config.reward_contract {
        config.reward_contract = deps.api.addr_validate(&reward_contract)?;
        res = res.add_attribute("reward_contract", reward_contract);
    }
    CONFIG.save(deps.storage, &config)?;
    Ok(res)
}

/// Update owner
pub fn update_owner(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;
    let new_owner_addr = deps.api.addr_validate(&new_owner)?;
    OWNER.set(deps.branch(), Some(new_owner_addr))?;
    Ok(Response::new()
        .add_attribute("action", "update owner")
        .add_attribute("to", new_owner))
}

/// Cw20 Receive hook msg handler.
pub fn receive_cw20(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_json(&msg.msg)? {
        Cw20HookMsg::Stake {} => {
            let config = CONFIG.load(deps.storage)?;
            let mut total_staking = TOTAL_STAKING.load(deps.storage).unwrap_or_default();
            // only ASTRO token contract can execute this message
            ensure_eq!(
                config.token,
                info.sender,
                ContractError::Cw20AddressesNotMatch {
                    got: info.sender.to_string(),
                    expected: config.token.to_string(),
                }
            );
            let mut user_staking = STAKING
                .load(deps.storage, &info.sender.to_string())
                .unwrap_or_default();
            user_staking = user_staking.checked_add(msg.amount).unwrap();
            STAKING.save(deps.storage, &msg.sender, &user_staking)?;
            total_staking = total_staking.checked_add(msg.amount).unwrap();
            TOTAL_STAKING.save(deps.storage, &total_staking)?;

            // send stake message to reward_contract
            let stake_msg = WasmMsg::Execute {
                contract_addr: config.reward_contract.to_string(),
                msg: to_json_binary(&RewardDistributorExecuteMsg::FlexibleStake {
                    user: msg.sender,
                    amount: msg.amount,
                })?,
                funds: vec![],
            };
            Ok(Response::new().add_message(stake_msg))
        }
    }
}

/// Claim user rewards
pub fn claim(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let claim_msg = WasmMsg::Execute {
        contract_addr: config.reward_contract.to_string(),
        msg: to_json_binary(&RewardDistributorExecuteMsg::Claim {
            user: info.sender.to_string(),
        })?,
        funds: vec![],
    };
    Ok(Response::new().add_message(claim_msg))
}

/// Unstake amount and claim rewards of user
pub fn unstake(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut user_staking = STAKING.load(deps.storage, &info.sender.to_string())?;
    let mut total_staking = TOTAL_STAKING.load(deps.storage)?;
    ensure!(
        amount.le(&user_staking),
        ContractError::ExeedingUnstakeAmount {
            got: amount.u128(),
            expected: user_staking.u128()
        }
    );
    user_staking = user_staking.checked_sub(amount).unwrap();
    total_staking = total_staking.checked_sub(amount).unwrap();
    STAKING.save(deps.storage, &info.sender.to_string(), &user_staking)?;
    TOTAL_STAKING.save(deps.storage, &total_staking)?;
    let msg = vec![
        WasmMsg::Execute {
            contract_addr: config.token.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount,
            })?,
            funds: vec![],
        },
        WasmMsg::Execute {
            contract_addr: config.reward_contract.to_string(),
            msg: to_json_binary(&RewardDistributorExecuteMsg::FlexibleUnstake {
                user: info.sender.to_string(),
                amount,
            })?,
            funds: vec![],
        },
    ];
    Ok(Response::new().add_messages(msg))
}
