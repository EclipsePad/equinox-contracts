use cosmwasm_std::{coins, Addr, Decimal, StdResult, Uint128};
use cw_multi_test::{AppResponse, ContractWrapper, Executor};

use eclipse_base::error::parse_err;
use equinox_msg::voter::{
    AddressConfig, DateConfig, EssenceInfo, ExecuteMsg, QueryMsg, TokenConfig, VotingListItem,
};

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

    fn voter_try_accept_admin_role(&mut self, sender: impl ToString) -> StdResult<AppResponse>;

    fn voter_try_update_address_config(
        &mut self,
        sender: impl ToString,
        admin: Option<impl ToString>,
        worker_list: Option<Vec<impl ToString>>,
        eclipsepad_minter: Option<Addr>,
        eclipsepad_staking: Option<Addr>,
        eclipsepad_tribute_market: Option<Addr>,
        astroport_staking: Option<Addr>,
        astroport_assembly: Option<Addr>,
        astroport_voting_escrow: Option<Addr>,
        astroport_emission_controller: Option<Addr>,
        astroport_tribute_market: Option<Addr>,
    ) -> StdResult<AppResponse>;

    fn voter_try_update_token_config(
        &mut self,
        sender: impl ToString,
        astro: Option<&str>,
        xastro: Option<&str>,
        eclip_astro: Option<&str>,
    ) -> StdResult<AppResponse>;

    fn voter_try_update_date_config(
        &mut self,
        sender: impl ToString,
        epochs_start: Option<u64>,
        epoch_length: Option<u64>,
        vote_cooldown: Option<u64>,
    ) -> StdResult<AppResponse>;

    fn voter_try_capture_essence(
        &mut self,
        sender: impl ToString,
        user_and_essence_list: &[(impl ToString, EssenceInfo)],
        total_essence: EssenceInfo,
    ) -> StdResult<AppResponse>;

    fn voter_try_swap_to_eclip_astro(
        &mut self,
        sender: impl ToString,
        amount: u128,
        denom: &str,
    ) -> StdResult<AppResponse>;

    fn voter_try_vote(
        &mut self,
        sender: impl ToString,
        voting_list: &[VotingListItem],
    ) -> StdResult<AppResponse>;

    fn voter_query_address_config(&self) -> StdResult<AddressConfig>;

    fn voter_query_token_config(&self) -> StdResult<TokenConfig>;

    fn voter_query_date_config(&self) -> StdResult<DateConfig>;

    fn voter_query_voting_power(&self, address: impl ToString) -> StdResult<Uint128>;

    fn voter_query_xastro_price(&self) -> StdResult<Decimal>;
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

    fn voter_try_accept_admin_role(&mut self, sender: impl ToString) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.voter_contract_address(),
                &ExecuteMsg::AcceptAdminRole {},
                &[],
            )
            .map_err(parse_err)
    }

    fn voter_try_update_address_config(
        &mut self,
        sender: impl ToString,
        admin: Option<impl ToString>,
        worker_list: Option<Vec<impl ToString>>,
        eclipsepad_minter: Option<Addr>,
        eclipsepad_staking: Option<Addr>,
        eclipsepad_tribute_market: Option<Addr>,
        astroport_staking: Option<Addr>,
        astroport_assembly: Option<Addr>,
        astroport_voting_escrow: Option<Addr>,
        astroport_emission_controller: Option<Addr>,
        astroport_tribute_market: Option<Addr>,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.voter_contract_address(),
                &ExecuteMsg::UpdateAddressConfig {
                    admin: admin.map(|x| x.to_string()),
                    worker_list: worker_list
                        .map(|x| x.into_iter().map(|y| y.to_string()).collect()),
                    eclipsepad_minter: eclipsepad_minter.map(|x| x.to_string()),
                    eclipsepad_staking: eclipsepad_staking.map(|x| x.to_string()),
                    eclipsepad_tribute_market: eclipsepad_tribute_market.map(|x| x.to_string()),
                    astroport_staking: astroport_staking.map(|x| x.to_string()),
                    astroport_assembly: astroport_assembly.map(|x| x.to_string()),
                    astroport_voting_escrow: astroport_voting_escrow.map(|x| x.to_string()),
                    astroport_emission_controller: astroport_emission_controller
                        .map(|x| x.to_string()),
                    astroport_tribute_market: astroport_tribute_market.map(|x| x.to_string()),
                },
                &[],
            )
            .map_err(parse_err)
    }

    fn voter_try_update_token_config(
        &mut self,
        sender: impl ToString,
        astro: Option<&str>,
        xastro: Option<&str>,
        eclip_astro: Option<&str>,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.voter_contract_address(),
                &ExecuteMsg::UpdateTokenConfig {
                    astro: astro.map(|x| x.to_string()),
                    xastro: xastro.map(|x| x.to_string()),
                    eclip_astro: eclip_astro.map(|x| x.to_string()),
                },
                &[],
            )
            .map_err(parse_err)
    }

    fn voter_try_update_date_config(
        &mut self,
        sender: impl ToString,
        epochs_start: Option<u64>,
        epoch_length: Option<u64>,
        vote_cooldown: Option<u64>,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
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

    fn voter_try_capture_essence(
        &mut self,
        sender: impl ToString,
        user_and_essence_list: &[(impl ToString, EssenceInfo)],
        total_essence: EssenceInfo,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.voter_contract_address(),
                &ExecuteMsg::CaptureEssence {
                    user_and_essence_list: user_and_essence_list
                        .iter()
                        .map(|(user, essence)| (user.to_string(), essence.to_owned()))
                        .collect(),
                    total_essence,
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

    fn voter_try_vote(
        &mut self,
        sender: impl ToString,
        voting_list: &[VotingListItem],
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.voter_contract_address(),
                &ExecuteMsg::Vote {
                    voting_list: voting_list.to_owned(),
                },
                &[],
            )
            .map_err(parse_err)
    }

    fn voter_query_address_config(&self) -> StdResult<AddressConfig> {
        self.app
            .wrap()
            .query_wasm_smart(self.voter_contract_address(), &QueryMsg::AddressConfig {})
    }

    fn voter_query_token_config(&self) -> StdResult<TokenConfig> {
        self.app
            .wrap()
            .query_wasm_smart(self.voter_contract_address(), &QueryMsg::TokenConfig {})
    }

    fn voter_query_date_config(&self) -> StdResult<DateConfig> {
        self.app
            .wrap()
            .query_wasm_smart(self.voter_contract_address(), &QueryMsg::DateConfig {})
    }

    fn voter_query_voting_power(&self, address: impl ToString) -> StdResult<Uint128> {
        self.app.wrap().query_wasm_smart(
            self.voter_contract_address(),
            &QueryMsg::VotingPower {
                address: address.to_string(),
            },
        )
    }

    fn voter_query_xastro_price(&self) -> StdResult<Decimal> {
        self.app
            .wrap()
            .query_wasm_smart(self.voter_contract_address(), &QueryMsg::XastroPrice {})
    }
}
