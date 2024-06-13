#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

use eclipse_base::{
    error::ContractError,
    minter::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
};

use crate::actions::{
    execute as e, instantiate::try_instantiate, other::migrate_contract, query as q,
};

/// Creates a new contract with the specified parameters packed in the "msg" variable
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    try_instantiate(deps, env, info, msg)
}

/// Exposes all the execute functions available in the contract
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateNative {
            token_owner,
            subdenom,
            decimals,
        } => e::try_create_native(deps, env, info, token_owner, subdenom, decimals),

        ExecuteMsg::Mint {
            token,
            amount,
            recipient,
        } => e::try_mint(deps, env, info, token, amount, recipient),

        ExecuteMsg::Burn {} => e::try_burn(deps, env, info, None, None),

        ExecuteMsg::SetMetadataNative { token, metadata } => {
            e::try_set_metadata_native(deps, env, info, token, metadata)
        }

        ExecuteMsg::ChangeAdminNative { token } => {
            e::try_change_admin_native(deps, env, info, token)
        }

        ExecuteMsg::AcceptAdminRole {} => e::try_accept_admin_role(deps, env, info),

        ExecuteMsg::RegisterCurrency { currency, creator } => {
            e::try_register_currency(deps, env, info, currency, creator)
        }

        ExecuteMsg::UpdateConfig {
            admin,
            cw20_code_id,
        } => e::try_update_config(deps, env, info, admin, cw20_code_id),

        _ => unimplemented!(),
    }
}

/// Exposes all the queries available in the contract
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryCurrenciesByCreator { creator } => {
            to_json_binary(&q::query_currencies_by_creator(deps, env, creator)?)
        }

        QueryMsg::QueryConfig {} => to_json_binary(&q::query_config(deps, env)?),

        _ => unimplemented!(),
    }
}

/// Used for contract migration
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    migrate_contract(deps, env, msg)
}
