use cosmwasm_std::{coins, Addr, StdResult};
use cw_multi_test::{AppResponse, ContractWrapper, Executor};

use eclipse_base::error::parse_err;
use equinox_msg::voter::{DateConfig, ExecuteMsg, QueryMsg};

use crate::suite_astro::helper::{ControllerHelper, Extension};

const NAME: &str = "voter";

pub trait VoterExtension {
    fn voter_code_id(&self) -> u64;
    fn voter_contract_address(&self) -> Addr;

    fn voter_prepare_contract(
        &mut self,

        worker_list: Option<Vec<&str>>,

        eclipsepad_minter: &Addr,
        eclipsepad_staking: &Addr,
        eclipsepad_tribute_market: Option<String>,
        astroport_staking: &Addr,
        astroport_assembly: &Addr,
        astroport_voting_escrow: &Addr,
        astroport_emission_controller: &Addr,
        astroport_tribute_market: Option<String>,

        astro: &str,
        xastro: &str,
        eclip_astro: &str,

        epochs_start: u64,
        epoch_length: u64,
        vote_cooldown: u64,
    );

    fn voter_try_update_date_config(
        &mut self,
        sender: &str,
        epochs_start: Option<u64>,
        epoch_length: Option<u64>,
        vote_cooldown: Option<u64>,
    ) -> StdResult<AppResponse>;

    fn voter_try_swap_to_eclip_astro(
        &mut self,
        sender: impl ToString,
        amount: u128,
        denom: &str,
    ) -> StdResult<AppResponse>;

    fn voter_query_date_config(&self) -> StdResult<DateConfig>;
}

impl VoterExtension for ControllerHelper {
    fn voter_code_id(&self) -> u64 {
        self.extension_list
            .iter()
            .find(|x| x.name == NAME)
            .unwrap()
            .code_id
    }

    fn voter_contract_address(&self) -> Addr {
        self.extension_list
            .iter()
            .find(|x| x.name == NAME)
            .unwrap()
            .contract_address
            .to_owned()
    }

    fn voter_prepare_contract(
        &mut self,

        worker_list: Option<Vec<&str>>,

        eclipsepad_minter: &Addr,
        eclipsepad_staking: &Addr,
        eclipsepad_tribute_market: Option<String>,
        astroport_staking: &Addr,
        astroport_assembly: &Addr,
        astroport_voting_escrow: &Addr,
        astroport_emission_controller: &Addr,
        astroport_tribute_market: Option<String>,

        astro: &str,
        xastro: &str,
        eclip_astro: &str,

        epochs_start: u64,
        epoch_length: u64,
        vote_cooldown: u64,
    ) {
        let code_id = self.app.store_code(Box::new(
            ContractWrapper::new_with_empty(
                voter::contract::execute,
                voter::contract::instantiate,
                voter::contract::query,
            )
            .with_reply_empty(voter::contract::reply)
            .with_migrate_empty(voter::contract::migrate),
        ));

        let contract_address = self
            .app
            .instantiate_contract(
                code_id,
                self.owner.clone(),
                &equinox_msg::voter::InstantiateMsg {
                    worker_list: worker_list
                        .map(|x| x.into_iter().map(|y| y.to_string()).collect()),

                    eclipsepad_minter: eclipsepad_minter.to_string(),
                    eclipsepad_staking: eclipsepad_staking.to_string(),
                    eclipsepad_tribute_market,
                    astroport_staking: astroport_staking.to_string(),
                    astroport_assembly: astroport_assembly.to_string(),
                    astroport_voting_escrow: astroport_voting_escrow.to_string(),
                    astroport_emission_controller: astroport_emission_controller.to_string(),
                    astroport_tribute_market,

                    astro: astro.to_string(),
                    xastro: xastro.to_string(),
                    eclip_astro: eclip_astro.to_string(),

                    epochs_start,
                    epoch_length,
                    vote_cooldown,
                },
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

    fn voter_try_update_date_config(
        &mut self,
        sender: &str,
        epochs_start: Option<u64>,
        epoch_length: Option<u64>,
        vote_cooldown: Option<u64>,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender),
                self.voter_contract_address(),
                &ExecuteMsg::UpdateDateConfig {
                    epochs_start,
                    epoch_length,
                    vote_cooldown,
                },
                &[],
            )
            .map_err(parse_err)
    }

    fn voter_try_swap_to_eclip_astro(
        &mut self,
        sender: impl ToString,
        amount: u128,
        denom: &str,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(&sender.to_string()),
                self.voter_contract_address(),
                &equinox_msg::voter::ExecuteMsg::SwapToEclipAstro {},
                &coins(amount, denom),
            )
            .map_err(parse_err)
    }

    fn voter_query_date_config(&self) -> StdResult<DateConfig> {
        self.app
            .wrap()
            .query_wasm_smart(self.voter_contract_address(), &QueryMsg::DateConfig {})
    }
}
