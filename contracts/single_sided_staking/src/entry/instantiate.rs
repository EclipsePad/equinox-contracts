use astroport::asset::AssetInfo;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};
use cw2::set_contract_version;

use crate::{
    config::{DEFAULT_BECLIP_DAILY_REWARD, DEFAULT_ECLIP_DAILY_REWARD, DEFAULT_TIMELOCK_CONFIG},
    error::ContractError,
    state::{CONFIG, CONTRACT_NAME, CONTRACT_VERSION, OWNER, REWARD_CONFIG},
};
use equinox_msg::single_sided_staking::{
    Config, InstantiateMsg, RewardConfig, RewardDetail, RewardDetails,
};

pub fn try_instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(
        deps.storage,
        &Config {
            token: msg.token,
            timelock_config: msg
                .timelock_config
                .unwrap_or(DEFAULT_TIMELOCK_CONFIG.to_vec()),
            voter: deps.api.addr_validate(&msg.voter)?,
            treasury: deps.api.addr_validate(&msg.treasury)?,
        },
    )?;

    let reward_config = RewardConfig {
        details: RewardDetails {
            eclip: RewardDetail {
                info: AssetInfo::NativeToken { denom: msg.eclip },
                daily_reward: Uint128::from(DEFAULT_ECLIP_DAILY_REWARD),
            },
            beclip: RewardDetail {
                info: AssetInfo::Token {
                    contract_addr: deps.api.addr_validate(&msg.beclip)?,
                },
                daily_reward: Uint128::from(DEFAULT_BECLIP_DAILY_REWARD),
            },
        },
        reward_end_time: None,
    };
    REWARD_CONFIG.save(deps.storage, &reward_config)?;
    let owner = deps.api.addr_validate(msg.owner.as_str())?;
    OWNER.set(deps.branch(), Some(owner))?;
    Ok(Response::new().add_attributes([("action", "instantiate token converter")]))
}
