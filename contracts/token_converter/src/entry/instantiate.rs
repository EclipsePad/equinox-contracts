use cosmwasm_std::{
    to_json_binary, Addr, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, StdError, SubMsg,
    SubMsgResponse, SubMsgResult, WasmMsg,
};
use cw2::set_contract_version;
use cw20::MinterResponse;
use cw_utils::parse_instantiate_response_data;
use equinox_msg::{
    token::InstantiateMsg as TokenInstantiateMsg,
    token_converter::{Config, InstantiateMsg, RewardConfig},
};

use crate::{
    contract::INSTANTIATE_TOKEN_REPLY_ID,
    error::ContractError,
    state::{CONFIG, CONTRACT_NAME, CONTRACT_VERSION, OWNER, REWARD_CONFIG},
};

/// eclipASTRO information.
const TOKEN_NAME: &str = "eclipASTRO";
const TOKEN_SYMBOL: &str = "eclipASTRO";

pub fn try_instantiate(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(
        deps.storage,
        &Config {
            astro: msg.astro,
            xastro: msg.xastro,
            staking_contract: msg.staking_contract,
            eclipastro: Addr::unchecked(""),
            vxastro_holder: None,
            treasury: deps.api.addr_validate(&msg.treasury)?,
            stability_pool: None,
            single_staking_contract: None,
            ce_reward_distributor: None,
        },
    )?;

    REWARD_CONFIG.save(
        deps.storage,
        &RewardConfig {
            users: 8000,         // 80%
            treasury: 1350,      // 20% * 67.5%
            ce_holders: 400,     // 20% * 20%
            stability_pool: 250, // 20% * 12.5%
        },
    )?;

    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.set(deps.branch(), Some(owner))?;

    // Create the eclipASTRO token
    let sub_msg: Vec<SubMsg> = vec![SubMsg {
        msg: WasmMsg::Instantiate {
            admin: Some(msg.owner),
            code_id: msg.token_code_id,
            msg: to_json_binary(&TokenInstantiateMsg {
                name: TOKEN_NAME.to_string(),
                symbol: TOKEN_SYMBOL.to_string(),
                decimals: 6,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: env.contract.address.to_string(),
                    cap: None,
                }),
                marketing: msg.marketing,
            })?,
            funds: vec![],
            label: String::from("Staked Astroport Token"),
        }
        .into(),
        id: INSTANTIATE_TOKEN_REPLY_ID,
        gas_limit: None,
        reply_on: ReplyOn::Success,
    }];

    Ok(Response::new().add_submessages(sub_msg))
}

pub fn handle_instantiate_reply(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    match msg.result {
        SubMsgResult::Ok(SubMsgResponse {
            data: Some(data), ..
        }) => {
            if config.eclipastro != Addr::unchecked("") {
                return Err(ContractError::Unauthorized {});
            }

            let init_response = parse_instantiate_response_data(data.as_slice())
                .map_err(|e| StdError::generic_err(format!("{e}")))?;

            config.eclipastro = deps.api.addr_validate(&init_response.contract_address)?;

            CONFIG.save(deps.storage, &config)?;

            Ok(Response::new())
        }
        _ => Err(ContractError::FailedToParseReply {}),
    }
}
