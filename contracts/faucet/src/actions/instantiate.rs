use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::{Config, InstantiateMsg},
    state::{CONFIG, CONTRACT_NAME},
};

const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn try_instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(deps.storage, &Config {
        astro: msg.astro,
        xastro: msg.xastro,
        astro_staking: msg.astro_staking,
        daily_amount: Uint128::from(1_000_000_000u128),
    })?;

    Ok(Response::new()
        .add_attributes([("action", "try_instantiate")]))
}
