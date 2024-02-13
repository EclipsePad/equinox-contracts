use cosmwasm_schema::cw_serde;
use cosmwasm_std::{from_json, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError};
use cw20::Cw20ReceiveMsg;
use cw_multi_test::{Contract, ContractWrapper};

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
}

#[cw_serde]
pub enum Cw20HookMsg {
    Stake {},
}

fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> Result<Response, StdError> {
    Ok(Response::default())
}

fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, StdError> {
    match msg {
        ExecuteMsg::Receive(msg) => {
            let message: Cw20HookMsg = from_json(&msg.msg)?;
            match message {
                Cw20HookMsg::Stake {} => {
                }
            }
        }
    }
    Ok(Response::new())
}

#[allow(dead_code)]
fn query(_deps: Deps, _env: Env, _msg: Empty) -> Result<Binary, StdError> {
    unimplemented!()
}

pub fn voter_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}
