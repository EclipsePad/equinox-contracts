use cosmwasm_std::{coins, Addr, StdResult};
use cw_multi_test::{AppResponse, ContractWrapper, Executor};

use eclipse_base::error::parse_err;
use equinox_msg::token_converter::{ExecuteMsg, QueryMsg, RewardResponse};

use crate::suite_astro::helper::{Acc, ControllerHelper, Extension};

const NAME: &str = "token_converter";

pub trait TokenConverterExtension {
    fn token_converter_code_id(&self) -> u64;
    fn token_converter_contract_address(&self) -> Addr;

    fn token_converter_prepare_contract(
        &mut self,
        astro: &str,
        xastro: &str,
        staking_contract: &Addr,
        treasury: &Addr,
    );

    fn token_converter_try_convert(
        &mut self,
        sender: impl ToString,
        amount: u128,
        denom: &str,
        recipient: &Option<String>,
    ) -> StdResult<AppResponse>;

    fn token_converter_query_rewards(&self) -> StdResult<RewardResponse>;
}

impl TokenConverterExtension for ControllerHelper {
    fn token_converter_code_id(&self) -> u64 {
        self.extension_list
            .iter()
            .find(|x| x.name == NAME)
            .unwrap()
            .code_id
    }

    fn token_converter_contract_address(&self) -> Addr {
        self.extension_list
            .iter()
            .find(|x| x.name == NAME)
            .unwrap()
            .contract_address
            .to_owned()
    }

    fn token_converter_prepare_contract(
        &mut self,
        astro: &str,
        xastro: &str,
        staking_contract: &Addr,
        treasury: &Addr,
    ) {
        let code_id = self.app.store_code(Box::new(
            ContractWrapper::new_with_empty(
                token_converter::contract::execute,
                token_converter::contract::instantiate,
                token_converter::contract::query,
            )
            .with_migrate_empty(token_converter::contract::migrate),
        ));

        let contract_address = self
            .app
            .instantiate_contract(
                code_id,
                self.acc(Acc::Owner),
                &equinox_msg::token_converter::InstantiateMsg {
                    owner: self.acc(Acc::Owner).to_string(),
                    astro: astro.to_string(),
                    xastro: xastro.to_string(),
                    staking_contract: staking_contract.to_owned(),
                    treasury: treasury.to_string(),
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

    fn token_converter_try_convert(
        &mut self,
        sender: impl ToString,
        amount: u128,
        denom: &str,
        recipient: &Option<String>,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.token_converter_contract_address(),
                &ExecuteMsg::Convert {
                    recipient: recipient.to_owned(),
                },
                &coins(amount, denom.to_string()),
            )
            .map_err(parse_err)
    }

    fn token_converter_query_rewards(&self) -> StdResult<RewardResponse> {
        self.app.wrap().query_wasm_smart(
            self.token_converter_contract_address(),
            &QueryMsg::Rewards {},
        )
    }
}
