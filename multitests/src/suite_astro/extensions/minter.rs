use cosmwasm_std::{Addr, StdResult};
use cw_multi_test::{AppResponse, ContractWrapper, Executor};

use eclipse_base::{
    assets::{Currency, Token},
    error::parse_err,
    minter::{
        msg::{ExecuteMsg, QueryCurrenciesFromCreatorResponse, QueryMsg},
        types::Config,
    },
};

use crate::suite_astro::helper::{ControllerHelper, Extension};

const NAME: &str = "minter";

pub trait MinterExtension {
    fn minter_code_id(&self) -> u64;
    fn minter_contract_address(&self) -> Addr;

    fn minter_prepare_contract(&mut self);

    fn minter_try_register_currency(
        &mut self,
        sender: &str,
        currency: &Currency<Token>,
        creator: &Addr,
    ) -> StdResult<AppResponse>;

    fn minter_query_currencies_by_creator(
        &self,
        creator: &Addr,
    ) -> StdResult<QueryCurrenciesFromCreatorResponse>;

    fn minter_query_config(&self) -> StdResult<Config>;

    fn minter_query_token_owner(&self, denom_or_address: &str) -> StdResult<Addr>;
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

    fn minter_prepare_contract(&mut self) {
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
                self.owner.clone(),
                &eclipse_base::minter::msg::InstantiateMsg { cw20_code_id: None },
                &[],
                NAME,
                Some(self.owner.to_string()),
            )
            .unwrap();

        self.extension_list.push(Extension {
            name: NAME.to_string(),
            code_id,
            contract_address,
        });
    }

    fn minter_try_register_currency(
        &mut self,
        sender: &str,
        currency: &Currency<Token>,
        creator: &Addr,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender),
                self.minter_contract_address(),
                &ExecuteMsg::RegisterCurrency {
                    currency: Currency::new(&currency.clone().token.into(), currency.decimals),
                    creator: creator.to_string(),
                },
                &[],
            )
            .map_err(parse_err)
    }

    fn minter_query_currencies_by_creator(
        &self,
        creator: &Addr,
    ) -> StdResult<QueryCurrenciesFromCreatorResponse> {
        self.app.wrap().query_wasm_smart(
            self.minter_contract_address(),
            &QueryMsg::QueryCurrenciesByCreator {
                creator: creator.to_string(),
            },
        )
    }

    fn minter_query_config(&self) -> StdResult<Config> {
        self.app
            .wrap()
            .query_wasm_smart(self.minter_contract_address(), &QueryMsg::QueryConfig {})
    }

    fn minter_query_token_owner(&self, denom_or_address: &str) -> StdResult<Addr> {
        self.app.wrap().query_wasm_smart(
            self.minter_contract_address(),
            &QueryMsg::QueryTokenOwner {
                denom: denom_or_address.to_string(),
            },
        )
    }
}
