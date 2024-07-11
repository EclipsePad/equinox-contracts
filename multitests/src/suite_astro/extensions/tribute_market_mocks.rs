use cosmwasm_std::{coin, Addr, Coin, StdResult, Uint128};
use cw_multi_test::{AppResponse, ContractWrapper, Executor};

use eclipse_base::error::parse_err;

use tribute_market_mocks::msg::{ExecuteMsg, QueryMsg};

use crate::suite_astro::helper::{Acc, ControllerHelper, Extension};

const NAME: &str = "tribute_market_mocks";

pub trait TributeMarketExtension {
    fn tribute_market_code_id(&self) -> u64;
    fn tribute_market_contract_address(&self) -> Addr;

    fn tribute_market_prepare_contract(&mut self);

    fn tribute_market_try_deposit_rewards(
        &mut self,
        sender: impl ToString,
        denom_and_amount_list: &Vec<(impl ToString, u128)>,
    ) -> StdResult<AppResponse>;

    fn tribute_market_try_claim_rewards(&mut self, sender: impl ToString)
        -> StdResult<AppResponse>;

    fn tribute_market_query_rewards(
        &self,
        user: impl ToString,
    ) -> StdResult<Vec<(String, Uint128)>>;
}

impl TributeMarketExtension for ControllerHelper {
    fn tribute_market_code_id(&self) -> u64 {
        self.extension_list
            .iter()
            .find(|x| x.name == NAME)
            .unwrap()
            .code_id
    }

    fn tribute_market_contract_address(&self) -> Addr {
        self.extension_list
            .iter()
            .find(|x| x.name == NAME)
            .unwrap()
            .contract_address
            .to_owned()
    }

    fn tribute_market_prepare_contract(&mut self) {
        let code_id = self.app.store_code(Box::new(
            ContractWrapper::new_with_empty(
                tribute_market_mocks::contract::execute,
                tribute_market_mocks::contract::instantiate,
                tribute_market_mocks::contract::query,
            )
            .with_migrate_empty(tribute_market_mocks::contract::migrate),
        ));

        let contract_address = self
            .app
            .instantiate_contract(
                code_id,
                self.acc(Acc::Owner),
                &tribute_market_mocks::msg::InstantiateMsg {},
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

    fn tribute_market_try_deposit_rewards(
        &mut self,
        sender: impl ToString,
        denom_and_amount_list: &Vec<(impl ToString, u128)>,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.tribute_market_contract_address(),
                &ExecuteMsg::DepositRewards {},
                &denom_and_amount_list
                    .into_iter()
                    .map(|(denom, amount)| coin(amount.to_owned(), denom.to_string()))
                    .collect::<Vec<Coin>>(),
            )
            .map_err(parse_err)
    }

    fn tribute_market_try_claim_rewards(
        &mut self,
        sender: impl ToString,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.tribute_market_contract_address(),
                &ExecuteMsg::ClaimRewards {},
                &[],
            )
            .map_err(parse_err)
    }

    fn tribute_market_query_rewards(
        &self,
        user: impl ToString,
    ) -> StdResult<Vec<(String, Uint128)>> {
        self.app.wrap().query_wasm_smart(
            self.tribute_market_contract_address(),
            &QueryMsg::QueryRewards {
                user: user.to_string(),
            },
        )
    }
}
