use cosmwasm_std::{Addr, Empty, Uint128};
use cw20::{Cw20Coin, MinterResponse};
use cw20_base::msg::ExecuteMsg as Cw20ExecuteMsg;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use equinox_msg::token_converter::InstantiateMsg;
use token::token::InstantiateMsg as TokenInstantiateMsg;

use super::voter::voter_contract;

pub fn contract_token() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(
        token::contract::execute,
        token::contract::instantiate,
        token::contract::query,
    );

    Box::new(contract)
}

pub fn contract_converter() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_migrate(crate::contract::migrate);

    Box::new(contract)
}

#[derive(Debug, Default)]
pub struct SuiteBuilder {
    pub token_in: String,
    pub token_out: String,
    pub vxtoken_holder: String,
    pub treasury: String,
    pub lp_staking_vault: String,
    pub staking_reward_distributor: String,
    pub ce_reward_distributor: String,
}

impl SuiteBuilder {
    pub fn new() -> Self {
        Self {
            token_in: "".to_owned(),
            token_out: "".to_owned(),
            vxtoken_holder: "".to_owned(),
            treasury: "".to_owned(),
            lp_staking_vault: "".to_owned(),
            staking_reward_distributor: "".to_owned(),
            ce_reward_distributor: "".to_owned(),
        }
    }

    #[track_caller]
    pub fn build(self) -> Suite {
        let mut app: App = App::default();

        let admin = Addr::unchecked("admin");

        let token_id = app.store_code(contract_token());
        let astro_contract = app
            .instantiate_contract(
                token_id,
                admin.clone(),
                &TokenInstantiateMsg {
                    name: "astro".to_owned(),
                    symbol: "ASTRO".to_owned(),
                    decimals: 6,
                    initial_balances: [
                        Cw20Coin {
                            address: "user1".to_owned(),
                            amount: Uint128::from(1000u128),
                        },
                        Cw20Coin {
                            address: "user2".to_owned(),
                            amount: Uint128::from(1000u128),
                        },
                    ]
                    .to_vec(),
                    mint: Some(MinterResponse {
                        minter: "minter".to_owned(),
                        cap: None,
                    }),
                    marketing: None,
                },
                &[],
                "astro",
                None,
            )
            .unwrap();
        let eclipastro_contract = app
            .instantiate_contract(
                token_id,
                admin.clone(),
                &TokenInstantiateMsg {
                    name: "eclipastro".to_owned(),
                    symbol: "eclipASTRO".to_owned(),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: "minter".to_owned(),
                        cap: None,
                    }),
                    marketing: None,
                },
                &[],
                "eclipastro",
                None,
            )
            .unwrap();
        let xastro_contract = app
        .instantiate_contract(
            token_id,
            admin.clone(),
            &TokenInstantiateMsg {
                name: "xastro".to_owned(),
                symbol: "xASTRO".to_owned(),
                decimals: 6,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: "minter".to_owned(),
                    cap: None,
                }),
                marketing: None,
            },
            &[],
            "xastro",
            None,
        )
        .unwrap();

        let voter_id = app.store_code(voter_contract());
        let voter_contract = app
            .instantiate_contract(voter_id, admin.clone(), &Empty {}, &[], "voter", None)
            .unwrap();

        let converter_id = app.store_code(contract_converter());
        let converter_contract = app
            .instantiate_contract(
                converter_id,
                admin.clone(),
                &InstantiateMsg {
                    owner: "admin".to_owned(),
                    token_in: astro_contract.to_string(),
                    token_out: eclipastro_contract.to_string(),
                    xtoken: xastro_contract.to_string(),
                    vxtoken_holder: voter_contract.to_string(),
                    treasury: "treasury".to_owned(),
                    stability_pool: "stability_pool".to_owned(),
                    staking_reward_distributor: "staking_reward_distributor".to_owned(),
                    ce_reward_distributor: "ce_reward_disgributor".to_owned(),
                },
                &[],
                "converter",
                None,
            )
            .unwrap();

        // now update minter of eclipASTRO token to converter contract
        app.execute_contract(
            Addr::unchecked("minter"),
            eclipastro_contract.clone(),
            &Cw20ExecuteMsg::UpdateMinter {
                new_minter: Some(converter_contract.to_string()),
            },
            &[],
        )
        .unwrap();

        Suite {
            app,
            converter_contract,
            astro_contract,
            eclipastro_contract,
            voter_contract,
        }
    }
}

pub struct Suite {
    app: App,
    converter_contract: Addr,
    astro_contract: Addr,
    eclipastro_contract: Addr,
    voter_contract: Addr,
}

impl Suite {
    pub fn converter_contract(&self) -> String {
        self.converter_contract.to_string()
    }

    pub fn astro_contract(&self) -> String {
        self.astro_contract.to_string()
    }

    pub fn eclipastro_contract(&self) -> String {
        self.eclipastro_contract.to_string()
    }
    
    pub fn voter_contract(&self) -> String {
        self.voter_contract.to_string()
    }

    // update block's time to simulate passage of time
    pub fn update_time(&mut self, time_update: u64) {
        let mut block = self.app.block_info();
        block.time = block.time.plus_seconds(time_update);
        self.app.set_block(block);
    }
}
