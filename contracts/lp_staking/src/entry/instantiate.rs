use astroport::asset::AssetInfo::{NativeToken, Token};
use cosmwasm_std::{ensure, DepsMut, Env, MessageInfo, Response, Uint128};
use cw2::set_contract_version;
use equinox_msg::lp_staking::{Config, InstantiateMsg, RewardConfig, RewardDetail, RewardDetails};

use crate::{
    config::{
        DEFAULT_BECLIP_DAILY_REWARD, DEFAULT_ECLIP_DAILY_REWARD, DEFAULT_REWARD_DISTRIBUTION,
        DEFAULT_REWARD_PERIOD,
    },
    entry::query::check_native_token_denom,
    error::ContractError,
    state::{CONFIG, CONTRACT_NAME, CONTRACT_VERSION, OWNER, REWARD_CONFIG},
};

pub fn try_instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    msg.lp_token.check(deps.api)?;
    ensure!(
        check_native_token_denom(&deps.querier, msg.astro.clone()).unwrap_or_default(),
        ContractError::InvalidDenom(msg.astro.clone())
    );
    ensure!(
        check_native_token_denom(&deps.querier, msg.xastro.clone()).unwrap_or_default(),
        ContractError::InvalidDenom(msg.xastro.clone())
    );
    // update config
    CONFIG.save(
        deps.storage,
        &Config {
            lp_token: msg.lp_token,
            lp_contract: deps.api.addr_validate(msg.lp_contract.as_str())?,
            astro: msg.astro,
            xastro: msg.xastro,
            astro_staking: deps.api.addr_validate(msg.astro_staking.as_str())?,
            converter: deps.api.addr_validate(msg.converter.as_str())?,
            astroport_incentives: deps.api.addr_validate(msg.astroport_incentives.as_str())?,
            treasury: deps.api.addr_validate(msg.treasury.as_str())?,
            stability_pool: deps.api.addr_validate(msg.stability_pool.as_str())?,
            ce_reward_distributor: deps.api.addr_validate(msg.ce_reward_distributor.as_str())?,
        },
    )?;
    // update reward config
    let reward_end_time = env.block.time.seconds() + DEFAULT_REWARD_PERIOD;
    let reward_config = RewardConfig {
        distribution: DEFAULT_REWARD_DISTRIBUTION,
        reward_end_time,
        details: RewardDetails {
            eclip: RewardDetail {
                info: NativeToken { denom: msg.eclip },
                daily_reward: Uint128::from(DEFAULT_ECLIP_DAILY_REWARD),
            },
            beclip: RewardDetail {
                info: Token {
                    contract_addr: deps.api.addr_validate(&msg.beclip)?,
                },
                daily_reward: Uint128::from(DEFAULT_BECLIP_DAILY_REWARD),
            },
        },
    };
    REWARD_CONFIG.save(deps.storage, &reward_config)?;
    // update owner
    let owner = deps
        .api
        .addr_validate(msg.owner.unwrap_or(info.sender.to_string()).as_str())?;
    OWNER.set(deps.branch(), Some(owner))?;
    Ok(Response::new())
}
