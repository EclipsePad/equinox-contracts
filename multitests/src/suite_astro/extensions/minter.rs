use cosmwasm_std::{Addr, StdResult};
use cw_multi_test::{AppResponse, ContractWrapper, Executor};

use eclipse_base::{
    error::parse_err,
    minter::{
        msg::{ExecuteMsg, QueryMsg},
        types::{Config, CurrencyInfo},
    },
};

use crate::suite_astro::helper::{Acc, ControllerHelper, Extension};

const NAME: &str = "minter";

pub trait MinterExtension {
    fn minter_code_id(&self) -> u64;
    fn minter_contract_address(&self) -> Addr;

    fn minter_prepare_contract(
        &mut self,
        whitelist: &Option<Vec<Addr>>,
        cw20_code_id: &Option<u64>,
        permissionless_token_creation: &Option<bool>,
        permissionless_token_registration: &Option<bool>,
        max_tokens_per_owner: &Option<u16>,
    );

    fn minter_try_register_native(
        &mut self,
        sender: impl ToString,
        denom: &str,
        owner: &Option<Addr>,
        whitelist: &Option<Vec<Addr>>,
        permissionless_burning: &Option<bool>,
        decimals: &Option<u8>,
    ) -> StdResult<AppResponse>;

    fn minter_query_currency_info_list_by_owner(
        &self,
        owner: impl ToString,
        amount: u32,
        start_from: &Option<&str>,
    ) -> StdResult<Vec<CurrencyInfo>>;

    fn minter_query_config(&self) -> StdResult<Config>;

    fn minter_query_currency_info(&self, denom_or_address: &str) -> StdResult<CurrencyInfo>;
}

impl MinterExtension for ControllerHelper {
    fn minter_code_id(&self) -> u64 {
        self.extension_list
            .iter()
            .find(|x| x.name == NAME)
            .unwrap()
            .code_id
    }

    fn minter_contract_address(&self) -> Addr {
        self.extension_list
            .iter()
            .find(|x| x.name == NAME)
            .unwrap()
            .contract_address
            .to_owned()
    }

    fn minter_prepare_contract(
        &mut self,
        whitelist: &Option<Vec<Addr>>,
        cw20_code_id: &Option<u64>,
        permissionless_token_creation: &Option<bool>,
        permissionless_token_registration: &Option<bool>,
        max_tokens_per_owner: &Option<u16>,
    ) {
        let code_id = self.app.store_code(Box::new(
            ContractWrapper::new_with_empty(
                minter_mocks::contract::execute,
                minter_mocks::contract::instantiate,
                minter_mocks::contract::query,
            )
            .with_migrate_empty(minter_mocks::contract::migrate),
        ));

        let contract_address = self
            .app
            .instantiate_contract(
                code_id,
                self.acc(Acc::Owner),
                &eclipse_base::minter::msg::InstantiateMsg {
                    whitelist: whitelist
                        .as_ref()
                        .map(|x| x.iter().map(|y| y.to_string()).collect::<Vec<String>>()),
                    cw20_code_id: cw20_code_id.to_owned(),
                    permissionless_token_creation: permissionless_token_creation.to_owned(),
                    permissionless_token_registration: permissionless_token_registration.to_owned(),
                    max_tokens_per_owner: max_tokens_per_owner.to_owned(),
                },
                &[],
                NAME,
                Some(self.acc(Acc::Owner).to_string()),
            )
            .unwrap();

        self.extension_list.push(Extension {
            name: NAME.to_string(),
            code_id,
            contract_address,
        });
    }

    fn minter_try_register_native(
        &mut self,
        sender: impl ToString,
        denom: &str,
        owner: &Option<Addr>,
        whitelist: &Option<Vec<Addr>>,
        permissionless_burning: &Option<bool>,
        decimals: &Option<u8>,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.minter_contract_address(),
                &ExecuteMsg::RegisterNative {
                    denom: denom.to_string(),
                    owner: owner.as_ref().map(|x| x.to_string()),
                    whitelist: whitelist
                        .as_ref()
                        .map(|x| x.iter().map(|y| y.to_string()).collect::<Vec<String>>()),
                    permissionless_burning: permissionless_burning.to_owned(),
                    decimals: decimals.to_owned(),
                },
                &[],
            )
            .map_err(parse_err)
    }

    fn minter_query_currency_info_list_by_owner(
        &self,
        owner: impl ToString,
        amount: u32,
        start_from: &Option<&str>,
    ) -> StdResult<Vec<CurrencyInfo>> {
        self.app.wrap().query_wasm_smart(
            self.minter_contract_address(),
            &QueryMsg::CurrencyInfoListByOwner {
                owner: owner.to_string(),
                amount,
                start_from: start_from.as_ref().map(|x| x.to_string()),
            },
        )
    }

    fn minter_query_config(&self) -> StdResult<Config> {
        self.app
            .wrap()
            .query_wasm_smart(self.minter_contract_address(), &QueryMsg::Config {})
    }

    fn minter_query_currency_info(&self, denom_or_address: &str) -> StdResult<CurrencyInfo> {
        self.app.wrap().query_wasm_smart(
            self.minter_contract_address(),
            &QueryMsg::CurrencyInfo {
                denom_or_address: denom_or_address.to_string(),
            },
        )
    }
}
