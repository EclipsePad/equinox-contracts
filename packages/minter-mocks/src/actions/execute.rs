use cosmwasm_std::{
    coins, to_json_binary, Addr, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Response,
    StdResult, Storage, SubMsg, SubMsgResult, Uint128, WasmMsg,
};

use cw20::Logo;

use eclipse_base::{
    assets::{Currency, Token, TokenUnverified},
    error::ContractError,
    minter::{
        state::{
            CONFIG, CURRENCIES, DEFAULT_DECIMALS, IS_PAUSED, SAVE_CW20_ADDRESS_REPLY,
            TEMPORARY_CURRENCY, TOKEN_COUNT, TRANSFER_ADMIN_STATE, TRANSFER_ADMIN_TIMEOUT,
            TRANSFER_OWNER_STATE,
        },
        types::{Config, CurrencyInfo, Metadata, TransferAdminState},
    },
    utils::{check_funds, unwrap_field, FundsType},
};

pub fn try_accept_admin_role(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let (sender_address, ..) = check_funds(deps.as_ref(), &info, FundsType::Empty)?;
    let block_time = env.block.time.seconds();
    let TransferAdminState {
        new_admin,
        deadline,
    } = TRANSFER_ADMIN_STATE.load(deps.storage)?;

    if sender_address != new_admin {
        Err(ContractError::Unauthorized)?;
    }

    if block_time >= deadline {
        Err(ContractError::TransferAdminDeadline)?;
    }

    CONFIG.update(deps.storage, |mut x| -> StdResult<Config> {
        x.admin = sender_address;
        Ok(x)
    })?;

    TRANSFER_ADMIN_STATE.update(deps.storage, |mut x| -> StdResult<TransferAdminState> {
        x.deadline = block_time;
        Ok(x)
    })?;

    Ok(Response::new().add_attribute("action", "try_accept_admin_role"))
}

pub fn try_accept_token_owner_role(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let (sender_address, ..) = check_funds(deps.as_ref(), &info, FundsType::Empty)?;
    let block_time = env.block.time.seconds();
    let config = CONFIG.load(deps.storage)?;
    let mut transfer_owner_state_list = TRANSFER_OWNER_STATE.load(deps.storage).unwrap_or_default();

    let (
        denom_or_address,
        TransferAdminState {
            new_admin: new_token_owner,
            deadline,
        },
    ) = transfer_owner_state_list
        .iter()
        .find(|(_, transfer_state)| transfer_state.new_admin == sender_address)
        .ok_or(ContractError::Unauthorized)?;

    if &block_time >= deadline {
        Err(ContractError::TransferAdminDeadline)?;
    }

    let mut currency_info = CURRENCIES.load(deps.storage, denom_or_address)?;
    let current_owner_token_count = TOKEN_COUNT.load(deps.storage, &currency_info.owner)? - 1;
    let new_owner_token_count = TOKEN_COUNT
        .load(deps.storage, new_token_owner)
        .unwrap_or_default()
        + 1;

    if !config.whitelist.contains(new_token_owner)
        && new_owner_token_count > config.max_tokens_per_owner
    {
        Err(ContractError::TokenLimit)?;
    }

    if current_owner_token_count == 0 {
        TOKEN_COUNT.remove(deps.storage, &currency_info.owner);
    } else {
        TOKEN_COUNT.save(
            deps.storage,
            &currency_info.owner,
            &current_owner_token_count,
        )?;
    }
    TOKEN_COUNT.save(deps.storage, new_token_owner, &new_owner_token_count)?;

    currency_info.owner = new_token_owner.to_owned();
    CURRENCIES.save(deps.storage, denom_or_address, &currency_info)?;

    transfer_owner_state_list
        .retain(|(_, transfer_state)| transfer_state.new_admin != sender_address);
    TRANSFER_OWNER_STATE.save(deps.storage, &transfer_owner_state_list)?;

    Ok(Response::new().add_attribute("action", "try_accept_token_owner_role"))
}

pub fn try_pause(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let (sender_address, ..) = check_funds(deps.as_ref(), &info, FundsType::Empty)?;
    let Config { admin, .. } = CONFIG.load(deps.storage)?;

    if sender_address != admin {
        Err(ContractError::Unauthorized)?;
    }

    IS_PAUSED.save(deps.storage, &true)?;

    Ok(Response::new().add_attribute("action", "try_pause"))
}

pub fn try_unpause(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let (sender_address, ..) = check_funds(deps.as_ref(), &info, FundsType::Empty)?;
    let Config { admin, .. } = CONFIG.load(deps.storage)?;

    if sender_address != admin {
        Err(ContractError::Unauthorized)?;
    }

    IS_PAUSED.save(deps.storage, &false)?;

    Ok(Response::new().add_attribute("action", "try_unpause"))
}

#[allow(clippy::too_many_arguments)]
pub fn try_update_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    admin: Option<String>,
    whitelist: Option<Vec<String>>,
    cw20_code_id: Option<u64>,
    permissionless_token_creation: Option<bool>,
    permissionless_token_registration: Option<bool>,
    max_tokens_per_owner: Option<u16>,
) -> Result<Response, ContractError> {
    let (sender_address, ..) = check_funds(deps.as_ref(), &info, FundsType::Empty)?;
    let mut config = CONFIG.load(deps.storage)?;
    let mut is_config_updated = false;

    if sender_address != config.admin {
        Err(ContractError::Unauthorized)?;
    }

    if let Some(x) = admin {
        let block_time = env.block.time.seconds();
        let new_admin = &deps.api.addr_validate(&x)?;

        TRANSFER_ADMIN_STATE.save(
            deps.storage,
            &TransferAdminState {
                new_admin: new_admin.to_owned(),
                deadline: block_time + TRANSFER_ADMIN_TIMEOUT,
            },
        )?;

        is_config_updated = true;
    }

    if let Some(x) = whitelist {
        config.whitelist = x
            .iter()
            .map(|x| deps.api.addr_validate(x))
            .collect::<StdResult<Vec<Addr>>>()?;
        is_config_updated = true;
    }

    if let Some(x) = cw20_code_id {
        config.cw20_code_id = Some(x);
        is_config_updated = true;
    }

    if let Some(x) = permissionless_token_creation {
        config.permissionless_token_creation = x;
        is_config_updated = true;
    }

    if let Some(x) = permissionless_token_registration {
        config.permissionless_token_registration = x;
        is_config_updated = true;
    }

    if let Some(x) = max_tokens_per_owner {
        config.max_tokens_per_owner = x;
        is_config_updated = true;
    }

    // don't allow empty messages
    if !is_config_updated {
        Err(ContractError::NoParameters)?;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "try_update_config"))
}

#[allow(clippy::too_many_arguments)]
pub fn try_create_native(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: Option<String>,
    whitelist: Option<Vec<String>>,
    permissionless_burning: Option<bool>,
    subdenom: String,
    decimals: Option<u8>,
) -> Result<Response, ContractError> {
    check_pause_state(deps.storage)?;
    let (sender_address, ..) = check_funds(
        deps.as_ref(),
        &info,
        FundsType::Single {
            sender: None,
            amount: None,
        },
    )?;
    let config = CONFIG.load(deps.storage)?;

    if !config.permissionless_token_creation && !config.whitelist.contains(&sender_address) {
        Err(ContractError::Unauthorized)?;
    }

    let owner = owner
        .map(|x| deps.api.addr_validate(&x))
        .transpose()?
        .unwrap_or(sender_address.clone());
    let creator = env.contract.address;
    let full_denom = &get_full_denom(&creator, &subdenom);
    let decimals = decimals.unwrap_or(DEFAULT_DECIMALS);
    let currency = Currency::new(&Token::new_native(full_denom), decimals);
    let whitelist = whitelist
        .unwrap_or(vec![sender_address.to_string()])
        .iter()
        .map(|x| deps.api.addr_validate(x))
        .collect::<StdResult<Vec<Addr>>>()?;

    CURRENCIES.update(deps.storage, full_denom, |x| -> StdResult<CurrencyInfo> {
        match x {
            Some(_) => Err(ContractError::DenomExists)?,
            None => Ok(CurrencyInfo {
                currency,
                cw20_code_id: None,
                owner,
                whitelist,
                permissionless_burning: permissionless_burning.unwrap_or_default(),
            }),
        }
    })?;

    TOKEN_COUNT.update(deps.storage, &sender_address, |x| -> StdResult<u16> {
        let token_count = x.unwrap_or_default() + 1;

        if !config.whitelist.contains(&sender_address) && token_count > config.max_tokens_per_owner
        {
            Err(ContractError::TokenLimit)?;
        }

        Ok(token_count)
    })?;

    Ok(Response::new().add_attribute("action", "try_create_native"))
}

#[allow(clippy::too_many_arguments)]
pub fn try_create_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: Option<String>,
    whitelist: Option<Vec<String>>,
    permissionless_burning: Option<bool>,
    cw20_code_id: Option<u64>,
    name: String,
    symbol: String,
    decimals: Option<u8>,
    marketing: Option<cw20_base::msg::InstantiateMarketingInfo>,
) -> Result<Response, ContractError> {
    check_pause_state(deps.storage)?;
    let (sender_address, ..) = check_funds(deps.as_ref(), &info, FundsType::Empty)?;
    let config = CONFIG.load(deps.storage)?;

    if !config.permissionless_token_creation && !config.whitelist.contains(&sender_address) {
        Err(ContractError::Unauthorized)?;
    }

    let owner = owner
        .map(|x| deps.api.addr_validate(&x))
        .transpose()?
        .unwrap_or(sender_address.clone());
    let creator = env.contract.address;
    let decimals = decimals.unwrap_or(DEFAULT_DECIMALS);
    let currency = Currency::new(&Token::new_cw20(&sender_address), decimals);
    let cw20_code_id = cw20_code_id.unwrap_or(unwrap_field(config.cw20_code_id, "cw20_code_id")?);
    let whitelist = whitelist
        .unwrap_or(vec![sender_address.to_string()])
        .iter()
        .map(|x| deps.api.addr_validate(x))
        .collect::<StdResult<Vec<Addr>>>()?;
    let mut marketing_info = marketing.unwrap_or(cw20_base::msg::InstantiateMarketingInfo {
        project: None,
        description: None,
        marketing: None,
        logo: None,
    });
    marketing_info.marketing = Some(creator.to_string());

    TOKEN_COUNT.update(deps.storage, &sender_address, |x| -> StdResult<u16> {
        let token_count = x.unwrap_or_default() + 1;

        if !config.whitelist.contains(&sender_address) && token_count > config.max_tokens_per_owner
        {
            Err(ContractError::TokenLimit)?;
        }

        Ok(token_count)
    })?;

    TEMPORARY_CURRENCY.push_back(
        deps.storage,
        &CurrencyInfo {
            currency, // will be rewritten later
            cw20_code_id: Some(cw20_code_id),
            owner,
            whitelist,
            permissionless_burning: permissionless_burning.unwrap_or_default(),
        },
    )?;

    let cw20_msg = cw20_base::msg::InstantiateMsg {
        name,
        symbol: symbol.to_string(),
        decimals,
        initial_balances: vec![],
        mint: Some(cw20::MinterResponse {
            minter: creator.to_string(),
            cap: None,
        }),
        marketing: Some(marketing_info),
    };

    let msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Instantiate {
        admin: Some(config.admin.to_string()), // to allow migrate special cw20 based contracts
        code_id: unwrap_field(CONFIG.load(deps.storage)?.cw20_code_id, "cw20_code_id")?,
        label: format!("CW20 based token {}", symbol),
        msg: to_json_binary(&cw20_msg)?,
        funds: vec![],
    });

    let submsg = SubMsg::reply_on_success(msg, SAVE_CW20_ADDRESS_REPLY);

    Ok(Response::new()
        .add_submessage(submsg)
        .add_attribute("action", "try_create_cw20"))
}

pub fn save_cw20_address(
    deps: DepsMut,
    _env: Env,
    result: &SubMsgResult,
) -> Result<Response, ContractError> {
    let res = result
        .to_owned()
        .into_result()
        .map_err(|e| ContractError::CustomError { val: e })?;

    let instantiate_event = unwrap_field(
        res.events.iter().find(|x| x.ty == "instantiate"),
        "instantiate_event",
    )?;

    let cw20_address = &unwrap_field(
        instantiate_event
            .attributes
            .iter()
            .find(|x| x.key == "_contract_address"),
        "cw20_address",
    )?
    .value;

    let mut currency_info = unwrap_field(
        TEMPORARY_CURRENCY.pop_front(deps.storage)?,
        "owner_and_decimals",
    )?;
    currency_info.currency = Currency::new(
        &Token::new_cw20(&deps.api.addr_validate(cw20_address)?),
        currency_info.currency.decimals,
    );

    CURRENCIES.update(deps.storage, cw20_address, |x| -> StdResult<CurrencyInfo> {
        match x {
            Some(_) => Err(ContractError::DenomExists)?,
            None => Ok(currency_info),
        }
    })?;

    Ok(Response::new().add_attribute("cw20_address", cw20_address))
}

/// current token admin must send MsgChangeAdmin before/after this action to complete registration
#[allow(clippy::too_many_arguments)]
pub fn try_register_native(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    denom: String,
    owner: Option<String>,
    whitelist: Option<Vec<String>>,
    permissionless_burning: Option<bool>,
    decimals: Option<u8>,
) -> Result<Response, ContractError> {
    check_pause_state(deps.storage)?;
    let (sender_address, ..) = check_funds(deps.as_ref(), &info, FundsType::Empty)?;
    let config = CONFIG.load(deps.storage)?;

    if !config.permissionless_token_registration && !config.whitelist.contains(&sender_address) {
        Err(ContractError::Unauthorized)?;
    }

    let owner = owner
        .map(|x| deps.api.addr_validate(&x))
        .transpose()?
        .unwrap_or(sender_address.clone());
    let decimals = decimals.unwrap_or(DEFAULT_DECIMALS);
    let currency = Currency::new(&Token::new_native(&denom), decimals);
    let whitelist = whitelist
        .unwrap_or(vec![sender_address.to_string()])
        .iter()
        .map(|x| deps.api.addr_validate(x))
        .collect::<StdResult<Vec<Addr>>>()?;

    CURRENCIES.update(deps.storage, &denom, |x| -> StdResult<CurrencyInfo> {
        match x {
            Some(_) => Err(ContractError::DenomExists)?,
            None => Ok(CurrencyInfo {
                currency,
                cw20_code_id: None,
                owner,
                whitelist,
                permissionless_burning: permissionless_burning.unwrap_or_default(),
            }),
        }
    })?;

    TOKEN_COUNT.update(deps.storage, &sender_address, |x| -> StdResult<u16> {
        let token_count = x.unwrap_or_default() + 1;

        if !config.whitelist.contains(&sender_address) && token_count > config.max_tokens_per_owner
        {
            Err(ContractError::TokenLimit)?;
        }

        Ok(token_count)
    })?;

    Ok(Response::new().add_attribute("action", "try_register_currency"))
}

/// current token admin must update contract admin, minter, marketing before/after this action  \
/// to complete registration
#[allow(clippy::too_many_arguments)]
pub fn try_register_cw20(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
    owner: Option<String>,
    whitelist: Option<Vec<String>>,
    permissionless_burning: Option<bool>,
    cw20_code_id: Option<u64>,
    decimals: Option<u8>,
) -> Result<Response, ContractError> {
    check_pause_state(deps.storage)?;
    let (sender_address, ..) = check_funds(deps.as_ref(), &info, FundsType::Empty)?;
    let config = CONFIG.load(deps.storage)?;

    if !config.permissionless_token_registration && !config.whitelist.contains(&sender_address) {
        Err(ContractError::Unauthorized)?;
    }

    let owner = owner
        .map(|x| deps.api.addr_validate(&x))
        .transpose()?
        .unwrap_or(sender_address.clone());
    let decimals = decimals.unwrap_or(DEFAULT_DECIMALS);
    let currency = Currency::new(
        &TokenUnverified::new_cw20(&address).verify(deps.api)?,
        decimals,
    );
    let cw20_code_id = cw20_code_id.unwrap_or(unwrap_field(config.cw20_code_id, "cw20_code_id")?);
    let whitelist = whitelist
        .unwrap_or(vec![sender_address.to_string()])
        .iter()
        .map(|x| deps.api.addr_validate(x))
        .collect::<StdResult<Vec<Addr>>>()?;

    CURRENCIES.update(deps.storage, &address, |x| -> StdResult<CurrencyInfo> {
        match x {
            Some(_) => Err(ContractError::DenomExists)?,
            None => Ok(CurrencyInfo {
                currency,
                cw20_code_id: Some(cw20_code_id),
                owner,
                whitelist,
                permissionless_burning: permissionless_burning.unwrap_or_default(),
            }),
        }
    })?;

    TOKEN_COUNT.update(deps.storage, &sender_address, |x| -> StdResult<u16> {
        let token_count = x.unwrap_or_default() + 1;

        if !config.whitelist.contains(&sender_address) && token_count > config.max_tokens_per_owner
        {
            Err(ContractError::TokenLimit)?;
        }

        Ok(token_count)
    })?;

    Ok(Response::new().add_attribute("action", "try_register_currency"))
}

pub fn try_update_currency_info(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom_or_address: String,
    owner: Option<String>,
    whitelist: Option<Vec<String>>,
    permissionless_burning: Option<bool>,
) -> Result<Response, ContractError> {
    check_pause_state(deps.storage)?;
    let (sender_address, ..) = check_funds(deps.as_ref(), &info, FundsType::Empty)?;
    let mut is_currency_info_updated = false;
    let mut currency_info = CURRENCIES
        .load(deps.storage, &denom_or_address)
        .map_err(|_| ContractError::AssetIsNotFound)?;

    if currency_info.owner != sender_address {
        Err(ContractError::Unauthorized)?;
    }

    if let Some(x) = owner {
        let block_time = env.block.time.seconds();
        let new_owner = &deps.api.addr_validate(&x)?;
        let mut transfer_owner_state_list =
            TRANSFER_OWNER_STATE.load(deps.storage).unwrap_or_default();
        transfer_owner_state_list
            .retain(|(current_denom_or_address, _)| current_denom_or_address != &denom_or_address);

        transfer_owner_state_list.push((
            denom_or_address.clone(),
            TransferAdminState {
                new_admin: new_owner.to_owned(),
                deadline: block_time + TRANSFER_ADMIN_TIMEOUT,
            },
        ));

        is_currency_info_updated = true;
    }

    if let Some(x) = whitelist {
        currency_info.whitelist = x
            .iter()
            .map(|x| deps.api.addr_validate(x))
            .collect::<StdResult<Vec<Addr>>>()?;
        is_currency_info_updated = true;
    }

    if let Some(x) = permissionless_burning {
        currency_info.permissionless_burning = x;
        is_currency_info_updated = true;
    }

    // don't allow empty messages
    if !is_currency_info_updated {
        Err(ContractError::NoParameters)?;
    }

    CURRENCIES.save(deps.storage, &denom_or_address, &currency_info)?;

    Ok(Response::new().add_attribute("action", "try_update_currency_info"))
}

pub fn try_update_metadata_native(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    denom: String,
    _metadata: Metadata,
) -> Result<Response, ContractError> {
    check_pause_state(deps.storage)?;
    let (sender_address, ..) = check_funds(deps.as_ref(), &info, FundsType::Empty)?;
    let currency_info = CURRENCIES
        .load(deps.storage, &denom)
        .map_err(|_| ContractError::AssetIsNotFound)?;

    if currency_info.owner != sender_address {
        Err(ContractError::Unauthorized)?;
    }

    Ok(Response::new().add_attribute("action", "try_update_metadata_native"))
}

pub fn try_update_metadata_cw20(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
    project: Option<String>,
    description: Option<String>,
    logo: Option<Logo>,
) -> Result<Response, ContractError> {
    check_pause_state(deps.storage)?;
    let mut response = Response::new().add_attribute("action", "try_update_metadata_cw20");
    let (sender_address, ..) = check_funds(deps.as_ref(), &info, FundsType::Empty)?;
    let currency_info = CURRENCIES
        .load(deps.storage, &address)
        .map_err(|_| ContractError::AssetIsNotFound)?;

    if currency_info.owner != sender_address {
        Err(ContractError::Unauthorized)?;
    }

    if project.is_some() || description.is_some() {
        response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: address.to_owned(),
            msg: to_json_binary(&cw20::Cw20ExecuteMsg::UpdateMarketing {
                project,
                description,
                marketing: None,
            })?,
            funds: vec![],
        }));
    }

    if let Some(x) = logo {
        response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: address,
            msg: to_json_binary(&cw20::Cw20ExecuteMsg::UploadLogo(x))?,
            funds: vec![],
        }));
    }

    // don't allow empty messages
    if response.messages.is_empty() {
        Err(ContractError::NoParameters)?;
    }

    Ok(response)
}

pub fn try_exclude_native(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    denom: String,
) -> Result<Response, ContractError> {
    check_pause_state(deps.storage)?;
    let (sender_address, ..) = check_funds(deps.as_ref(), &info, FundsType::Empty)?;
    let currency_info = CURRENCIES
        .load(deps.storage, &denom)
        .map_err(|_| ContractError::AssetIsNotFound)?;

    if currency_info.owner != sender_address {
        Err(ContractError::Unauthorized)?;
    }

    let token_count = TOKEN_COUNT.load(deps.storage, &currency_info.owner)? - 1;
    if token_count == 0 {
        TOKEN_COUNT.remove(deps.storage, &currency_info.owner);
    } else {
        TOKEN_COUNT.save(deps.storage, &currency_info.owner, &token_count)?;
    }

    CURRENCIES.remove(deps.storage, &denom);

    Ok(Response::new().add_attribute("action", "try_exclude_native"))
}

pub fn try_exclude_cw20(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    check_pause_state(deps.storage)?;
    let (sender_address, ..) = check_funds(deps.as_ref(), &info, FundsType::Empty)?;
    let currency_info = CURRENCIES
        .load(deps.storage, &address)
        .map_err(|_| ContractError::AssetIsNotFound)?;

    if currency_info.owner != sender_address {
        Err(ContractError::Unauthorized)?;
    }

    let token_count = TOKEN_COUNT.load(deps.storage, &currency_info.owner)? - 1;
    if token_count == 0 {
        TOKEN_COUNT.remove(deps.storage, &currency_info.owner);
    } else {
        TOKEN_COUNT.save(deps.storage, &currency_info.owner, &token_count)?;
    }

    CURRENCIES.remove(deps.storage, &address);

    let new_admin = currency_info.owner.to_string();
    let msg_list = vec![
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: address.clone(),
            msg: to_json_binary(&cw20::Cw20ExecuteMsg::UpdateMinter {
                new_minter: Some(new_admin.clone()),
            })?,
            funds: vec![],
        }),
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: address.clone(),
            msg: to_json_binary(&cw20::Cw20ExecuteMsg::UpdateMarketing {
                project: None,
                description: None,
                marketing: Some(new_admin.clone()),
            })?,
            funds: vec![],
        }),
        CosmosMsg::Wasm(WasmMsg::UpdateAdmin {
            contract_addr: address,
            admin: new_admin,
        }),
    ];

    Ok(Response::new()
        .add_messages(msg_list)
        .add_attribute("action", "try_exclude_cw20"))
}

pub fn try_mint(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    denom_or_address: String,
    amount: Uint128,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    check_pause_state(deps.storage)?;
    let mut response = Response::new().add_attribute("action", "try_mint");
    let (sender_address, ..) = check_funds(deps.as_ref(), &info, FundsType::Empty)?;
    let currency_info = CURRENCIES
        .load(deps.storage, &denom_or_address)
        .map_err(|_| ContractError::AssetIsNotFound)?;

    if !currency_info.whitelist.contains(&sender_address) {
        Err(ContractError::Unauthorized)?;
    }

    let recipient = recipient
        .map(|x| deps.api.addr_validate(&x))
        .transpose()?
        .unwrap_or(sender_address)
        .to_string();

    if currency_info.currency.token.is_native() {
        let msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient,
            amount: coins(amount.u128(), denom_or_address),
        });

        response = response.add_message(msg);
    } else {
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: denom_or_address,
            msg: to_json_binary(&cw20::Cw20ExecuteMsg::Mint { recipient, amount })?,
            funds: vec![],
        });

        response = response.add_message(msg);
    }

    Ok(response)
}

pub fn try_burn(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    sender: Option<String>,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    check_pause_state(deps.storage)?;
    let mut response = Response::new().add_attribute("action", "try_burn");
    let (sender_address, asset_amount, asset_info) =
        check_funds(deps.as_ref(), &info, FundsType::Single { sender, amount })?;
    let denom_or_address = asset_info.get_denom_or_address();
    let currency_info = CURRENCIES
        .load(deps.storage, &denom_or_address)
        .map_err(|_| ContractError::AssetIsNotFound)?;

    if !currency_info.permissionless_burning && !currency_info.whitelist.contains(&sender_address) {
        Err(ContractError::Unauthorized)?;
    }

    if !asset_info.is_native() {
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: denom_or_address,
            msg: to_json_binary(&cw20::Cw20ExecuteMsg::Burn {
                amount: asset_amount,
            })?,
            funds: vec![],
        });

        response = response.add_message(msg);
    }

    Ok(response)
}

fn get_full_denom(creator: &Addr, subdenom: &str) -> String {
    format!("factory/{creator}/{subdenom}")
}

/// user actions are disabled when the contract is paused
fn check_pause_state(storage: &dyn Storage) -> Result<(), ContractError> {
    if IS_PAUSED.load(storage)? {
        Err(ContractError::ContractIsPaused)?;
    }

    Ok(())
}
