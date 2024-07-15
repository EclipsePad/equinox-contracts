use cosmwasm_std::{coin, coins, Addr, Coin, StdResult, Uint128};
use cw_multi_test::{AppResponse, ContractWrapper, Executor};

use eclipse_base::error::parse_err;

use astroport::{
    asset::AssetInfo,
    router::{ExecuteMsg, QueryMsg, SimulateSwapOperationsResponse, SwapOperation},
};

use crate::suite_astro::helper::{Acc, ControllerHelper, Extension};

const NAME: &str = "astroport_router";

pub trait AstroportRouterExtension {
    fn astroport_router_code_id(&self) -> u64;
    fn astroport_router_contract_address(&self) -> Addr;

    fn astroport_router_prepare_contract(&mut self);

    fn astroport_router_try_execute_swap_operations(
        &mut self,
        sender: impl ToString,
        denom: impl ToString,
        amount: u128,
        operations: &Vec<SwapOperation>,
    ) -> StdResult<AppResponse>;

    fn astroport_router_try_execute_batch_swap(
        &mut self,
        sender: impl ToString,
        operations: &Vec<SwapOperation>,
        funds_in: &Vec<(u128, impl ToString)>,
    ) -> StdResult<AppResponse>;

    fn astroport_router_query_simulate_swap_operations(
        &self,
        amount_in: u128,
        denom_in: impl ToString,
        denom_out: impl ToString,
    ) -> StdResult<SimulateSwapOperationsResponse>;
}

impl AstroportRouterExtension for ControllerHelper {
    fn astroport_router_code_id(&self) -> u64 {
        self.extension_list
            .iter()
            .find(|x| x.name == NAME)
            .unwrap()
            .code_id
    }

    fn astroport_router_contract_address(&self) -> Addr {
        self.extension_list
            .iter()
            .find(|x| x.name == NAME)
            .unwrap()
            .contract_address
            .to_owned()
    }

    fn astroport_router_prepare_contract(&mut self) {
        let code_id = self.app.store_code(Box::new(
            ContractWrapper::new_with_empty(
                astroport_router::contract::execute,
                astroport_router::contract::instantiate,
                astroport_router::contract::query,
            )
            .with_reply_empty(astroport_router::contract::reply),
        ));

        let contract_address = self
            .app
            .instantiate_contract(
                code_id,
                self.acc(Acc::Owner),
                &astroport::router::InstantiateMsg {
                    astroport_factory: self.factory.to_string(),
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

    fn astroport_router_try_execute_swap_operations(
        &mut self,
        sender: impl ToString,
        denom: impl ToString,
        amount: u128,
        operations: &Vec<SwapOperation>,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.astroport_router_contract_address(),
                &ExecuteMsg::ExecuteSwapOperations {
                    operations: operations.to_owned(),
                    minimum_receive: None,
                    to: None,
                    max_spread: None,
                },
                &coins(amount, denom.to_string()),
            )
            .map_err(parse_err)
    }

    fn astroport_router_try_execute_batch_swap(
        &mut self,
        sender: impl ToString,
        operations: &Vec<SwapOperation>,
        funds_in: &Vec<(u128, impl ToString)>,
    ) -> StdResult<AppResponse> {
        let send_funds = &funds_in
            .into_iter()
            .map(|(amount, denom)| coin(amount.to_owned(), denom.to_string()))
            .collect::<Vec<Coin>>();

        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.astroport_router_contract_address(),
                &ExecuteMsg::ExecuteSwapOperations {
                    operations: operations.to_owned(),
                    minimum_receive: None,
                    to: None,
                    max_spread: None,
                },
                send_funds,
            )
            .map_err(parse_err)
    }

    fn astroport_router_query_simulate_swap_operations(
        &self,
        amount_in: u128,
        denom_in: impl ToString,
        denom_out: impl ToString,
    ) -> StdResult<SimulateSwapOperationsResponse> {
        self.app.wrap().query_wasm_smart(
            self.astroport_router_contract_address(),
            &QueryMsg::SimulateSwapOperations {
                offer_amount: Uint128::new(amount_in),
                operations: vec![SwapOperation::AstroSwap {
                    offer_asset_info: AssetInfo::NativeToken {
                        denom: denom_in.to_string(),
                    },
                    ask_asset_info: AssetInfo::NativeToken {
                        denom: denom_out.to_string(),
                    },
                }],
            },
        )
    }
}
