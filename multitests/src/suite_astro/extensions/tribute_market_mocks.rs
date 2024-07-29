use cosmwasm_std::{coin, coins, Addr, Coin, StdResult, Uint128};
use cw_multi_test::{AppResponse, BankSudo, ContractWrapper, Executor};

use eclipse_base::error::parse_err;

use equinox_msg::voter::types::BribesAllocationItem;
use tribute_market_mocks::msg::{ExecuteMsg, QueryMsg};

use crate::suite_astro::helper::{Acc, ControllerHelper, Extension};

const NAME: &str = "tribute_market_mocks";

pub trait TributeMarketExtension {
    fn tribute_market_code_id(&self) -> u64;
    fn tribute_market_contract_address(&self) -> Addr;

    fn tribute_market_prepare_contract(
        &mut self,
        astroport_voting_escrow: &Addr,
        astroport_emission_controller: &Addr,
    );

    fn tribute_market_try_set_bribes_allocation(
        &mut self,
        sender: impl ToString,
        bribes_allocation: &[BribesAllocationItem],
    ) -> StdResult<AppResponse>;

    fn tribute_market_try_allocate_rewards(
        &mut self,
        sender: impl ToString,
        users: &[impl ToString],
    ) -> StdResult<AppResponse>;

    fn tribute_market_try_claim_rewards(&mut self, sender: impl ToString)
        -> StdResult<AppResponse>;

    fn tribute_market_query_rewards(
        &self,
        user: impl ToString,
    ) -> StdResult<Vec<(Uint128, String)>>;

    fn query_bribes_allocation(&self) -> StdResult<Vec<BribesAllocationItem>>;
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

    fn tribute_market_prepare_contract(
        &mut self,
        astroport_voting_escrow: &Addr,
        astroport_emission_controller: &Addr,
    ) {
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
                &tribute_market_mocks::msg::InstantiateMsg {
                    astroport_voting_escrow: astroport_voting_escrow.to_owned(),
                    astroport_emission_controller: astroport_emission_controller.to_owned(),
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

    fn tribute_market_try_set_bribes_allocation(
        &mut self,
        sender: impl ToString,
        bribes_allocation: &[BribesAllocationItem],
    ) -> StdResult<AppResponse> {
        let mut funds: Vec<Coin> = vec![];

        let rewards: Vec<(Uint128, String)> = bribes_allocation
            .iter()
            .flat_map(|x| x.rewards.clone())
            .collect();

        for (_amount, denom) in &rewards {
            if funds.iter().all(|x| &x.denom != denom) {
                funds.push(coin(0, denom));
            }
        }

        funds = funds
            .into_iter()
            .map(|mut x| {
                x.amount = rewards
                    .iter()
                    .fold(Uint128::zero(), |acc, (cur_amount, cur_denom)| {
                        if cur_denom != &x.denom {
                            acc
                        } else {
                            acc + Uint128::new(10) * cur_amount // mul by 10 to have enough for multiple epochs
                        }
                    });
                x
            })
            .collect();

        for Coin { denom, amount } in &funds {
            self.app
                .sudo(
                    BankSudo::Mint {
                        to_address: sender.to_string(),
                        amount: coins(amount.u128(), denom),
                    }
                    .into(),
                )
                .unwrap();
        }

        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.tribute_market_contract_address(),
                &ExecuteMsg::SetBribesAllocation {
                    bribes_allocation: bribes_allocation.to_owned(),
                },
                &funds,
            )
            .map_err(parse_err)
    }

    fn tribute_market_try_allocate_rewards(
        &mut self,
        sender: impl ToString,
        users: &[impl ToString],
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.tribute_market_contract_address(),
                &ExecuteMsg::AllocateRewards {
                    users: users.iter().map(|x| x.to_string()).collect(),
                },
                &[],
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
    ) -> StdResult<Vec<(Uint128, String)>> {
        self.app.wrap().query_wasm_smart(
            self.tribute_market_contract_address(),
            &QueryMsg::Rewards {
                user: user.to_string(),
            },
        )
    }

    fn query_bribes_allocation(&self) -> StdResult<Vec<BribesAllocationItem>> {
        self.app.wrap().query_wasm_smart(
            self.tribute_market_contract_address(),
            &QueryMsg::BribesAllocation {},
        )
    }
}
