use cosmwasm_std::{coins, Addr, Decimal, StdResult, Uint128};
use cw_multi_test::{AppResponse, ContractWrapper, Executor};

use eclipse_base::{converters::str_to_dec, error::parse_err};
use equinox_msg::voter::{
    msg::{
        DaoResponse, ExecuteMsg, InstantiateMsg, QueryMsg, SudoMsg, UserListResponse, UserResponse,
        VoterInfoResponse,
    },
    types::{
        AddressConfig, BribesAllocationItem, DateConfig, EpochInfo, EssenceInfo, RouteListItem,
        TokenConfig, WeightAllocationItem,
    },
};

use crate::suite_astro::helper::{Acc, ControllerHelper, Extension};

const NAME: &str = "voter";

pub trait VoterExtension {
    fn voter_code_id(&self) -> u64;
    fn voter_contract_address(&self) -> Addr;

    #[allow(clippy::too_many_arguments)]
    fn voter_prepare_contract(
        &mut self,

        worker_list: Option<Vec<&str>>,

        eclipse_dao: &Addr,
        eclipsepad_foundry: Option<String>,
        eclipsepad_minter: &Addr,
        eclipsepad_staking: &Addr,
        eclipsepad_tribute_market: Option<String>,
        eclipse_single_sided_vault: Option<String>,
        astroport_staking: &Addr,
        astroport_assembly: &Addr,
        astroport_voting_escrow: &Addr,
        astroport_emission_controller: &Addr,
        astroport_router: &Addr,
        astroport_tribute_market: Option<String>,

        eclip: &str,
        astro: &str,
        xastro: &str,
        eclip_astro: &str,

        genesis_epoch_start_date: u64,
        epoch_length: u64,
        vote_delay: u64,
    );

    fn voter_try_accept_admin_role(&mut self, sender: impl ToString) -> StdResult<AppResponse>;

    #[allow(clippy::too_many_arguments)]
    fn voter_try_update_address_config(
        &mut self,
        sender: impl ToString,
        admin: Option<impl ToString>,
        worker_list: Option<Vec<impl ToString>>,
        eclipse_dao: Option<Addr>,
        eclipsepad_foundry: Option<Addr>,
        eclipsepad_minter: Option<Addr>,
        eclipsepad_staking: Option<Addr>,
        eclipsepad_tribute_market: Option<Addr>,
        eclipse_single_sided_vault: Option<Addr>,
        astroport_staking: Option<Addr>,
        astroport_assembly: Option<Addr>,
        astroport_voting_escrow: Option<Addr>,
        astroport_emission_controller: Option<Addr>,
        astroport_router: Option<Addr>,
        astroport_tribute_market: Option<Addr>,
    ) -> StdResult<AppResponse>;

    fn voter_try_update_token_config(
        &mut self,
        sender: impl ToString,
        eclip: Option<&str>,
        astro: Option<&str>,
        xastro: Option<&str>,
        eclip_astro: Option<&str>,
    ) -> StdResult<AppResponse>;

    fn voter_try_update_date_config(
        &mut self,
        sender: impl ToString,
        genesis_epoch_start_date: Option<u64>,
        epoch_length: Option<u64>,
        vote_delay: Option<u64>,
    ) -> StdResult<AppResponse>;

    fn voter_try_update_essence_allocation(
        &mut self,
        sender: impl ToString,
        user_and_essence_list: &[(impl ToString, EssenceInfo)],
    ) -> StdResult<AppResponse>;

    fn voter_try_swap_to_eclip_astro(
        &mut self,
        sender: impl ToString,
        amount: u128,
        denom: &str,
    ) -> StdResult<AppResponse>;

    fn voter_try_set_delegation(
        &mut self,
        sender: impl ToString,
        weight: &str,
    ) -> StdResult<AppResponse>;

    fn voter_try_place_vote(
        &mut self,
        sender: impl ToString,
        weight_allocation: &[WeightAllocationItem],
    ) -> StdResult<AppResponse>;

    fn voter_try_place_vote_as_dao(
        &mut self,
        sender: impl ToString,
        weight_allocation: &[WeightAllocationItem],
    ) -> StdResult<AppResponse>;

    fn voter_try_update_route_list(
        &mut self,
        sender: impl ToString,
        route_list: &[RouteListItem],
    ) -> StdResult<AppResponse>;

    fn voter_try_push(&mut self) -> StdResult<AppResponse>;

    fn voter_try_claim_rewards(&mut self, sender: impl ToString) -> StdResult<AppResponse>;

    fn voter_query_address_config(&self) -> StdResult<AddressConfig>;

    fn voter_query_token_config(&self) -> StdResult<TokenConfig>;

    fn voter_query_date_config(&self) -> StdResult<DateConfig>;

    fn voter_query_rewards(&self) -> StdResult<Vec<(Uint128, String)>>;

    fn voter_query_bribes_allocation(&self) -> StdResult<Vec<BribesAllocationItem>>;

    fn voter_query_voting_power(&self, address: impl ToString) -> StdResult<Uint128>;

    fn voter_query_xastro_price(&self) -> StdResult<Decimal>;

    fn voter_query_eclip_astro_minted_by_voter(&self) -> StdResult<Uint128>;

    fn voter_query_user(
        &self,
        address: impl ToString,
        block_time: Option<u64>,
    ) -> StdResult<Vec<UserResponse>>;

    fn voter_query_user_list(
        &self,
        block_time: Option<u64>,
        amount: u32,
        start_from: Option<String>,
    ) -> StdResult<Vec<UserListResponse>>;

    fn voter_query_dao_info(&self, block_time: Option<u64>) -> StdResult<DaoResponse>;

    fn voter_query_voter_info(&self, block_time: Option<u64>) -> StdResult<VoterInfoResponse>;

    fn voter_query_epoch_info(&self) -> StdResult<EpochInfo>;

    fn voter_query_route_list(
        &self,
        amount: u32,
        start_from: Option<String>,
    ) -> StdResult<Vec<RouteListItem>>;
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

        eclipse_dao: &Addr,
        eclipsepad_foundry: Option<String>,
        eclipsepad_minter: &Addr,
        eclipsepad_staking: &Addr,
        eclipsepad_tribute_market: Option<String>,
        eclipse_single_sided_vault: Option<String>,
        astroport_staking: &Addr,
        astroport_assembly: &Addr,
        astroport_voting_escrow: &Addr,
        astroport_emission_controller: &Addr,
        astroport_router: &Addr,
        astroport_tribute_market: Option<String>,

        eclip: &str,
        astro: &str,
        xastro: &str,
        eclip_astro: &str,

        genesis_epoch_start_date: u64,
        epoch_length: u64,
        vote_delay: u64,
    ) {
        let code_id = self.app.store_code(Box::new(
            ContractWrapper::new_with_empty(
                voter::contract::execute,
                voter::contract::instantiate,
                voter::contract::query,
            )
            .with_reply_empty(voter::contract::reply)
            .with_migrate_empty(voter::contract::migrate)
            .with_sudo_empty(voter::contract::sudo),
        ));

        let contract_address = self
            .app
            .instantiate_contract(
                code_id,
                self.acc(Acc::Owner),
                &InstantiateMsg {
                    worker_list: worker_list
                        .map(|x| x.into_iter().map(|y| y.to_string()).collect()),

                    eclipse_dao: eclipse_dao.to_string(),
                    eclipsepad_foundry,
                    eclipsepad_minter: eclipsepad_minter.to_string(),
                    eclipsepad_staking: eclipsepad_staking.to_string(),
                    eclipsepad_tribute_market,
                    eclipse_single_sided_vault,
                    astroport_staking: astroport_staking.to_string(),
                    astroport_assembly: astroport_assembly.to_string(),
                    astroport_voting_escrow: astroport_voting_escrow.to_string(),
                    astroport_emission_controller: astroport_emission_controller.to_string(),
                    astroport_router: astroport_router.to_string(),
                    astroport_tribute_market,

                    eclip: eclip.to_string(),
                    astro: astro.to_string(),
                    xastro: xastro.to_string(),
                    eclip_astro: eclip_astro.to_string(),

                    genesis_epoch_start_date,
                    epoch_length,
                    vote_delay,
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
        eclipse_dao: Option<Addr>,
        eclipsepad_foundry: Option<Addr>,
        eclipsepad_minter: Option<Addr>,
        eclipsepad_staking: Option<Addr>,
        eclipsepad_tribute_market: Option<Addr>,
        eclipse_single_sided_vault: Option<Addr>,
        astroport_staking: Option<Addr>,
        astroport_assembly: Option<Addr>,
        astroport_voting_escrow: Option<Addr>,
        astroport_emission_controller: Option<Addr>,
        astroport_router: Option<Addr>,
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
                    eclipse_dao: eclipse_dao.map(|x| x.to_string()),
                    eclipsepad_foundry: eclipsepad_foundry.map(|x| x.to_string()),
                    eclipsepad_minter: eclipsepad_minter.map(|x| x.to_string()),
                    eclipsepad_staking: eclipsepad_staking.map(|x| x.to_string()),
                    eclipsepad_tribute_market: eclipsepad_tribute_market.map(|x| x.to_string()),
                    eclipse_single_sided_vault: eclipse_single_sided_vault.map(|x| x.to_string()),
                    astroport_staking: astroport_staking.map(|x| x.to_string()),
                    astroport_assembly: astroport_assembly.map(|x| x.to_string()),
                    astroport_voting_escrow: astroport_voting_escrow.map(|x| x.to_string()),
                    astroport_emission_controller: astroport_emission_controller
                        .map(|x| x.to_string()),
                    astroport_router: astroport_router.map(|x| x.to_string()),
                    astroport_tribute_market: astroport_tribute_market.map(|x| x.to_string()),
                },
                &[],
            )
            .map_err(parse_err)
    }

    fn voter_try_update_token_config(
        &mut self,
        sender: impl ToString,
        eclip: Option<&str>,
        astro: Option<&str>,
        xastro: Option<&str>,
        eclip_astro: Option<&str>,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.voter_contract_address(),
                &ExecuteMsg::UpdateTokenConfig {
                    eclip: eclip.map(|x| x.to_string()),
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
        genesis_epoch_start_date: Option<u64>,
        epoch_length: Option<u64>,
        vote_delay: Option<u64>,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.voter_contract_address(),
                &ExecuteMsg::UpdateDateConfig {
                    genesis_epoch_start_date,
                    epoch_length,
                    vote_delay,
                },
                &[],
            )
            .map_err(parse_err)
    }

    fn voter_try_update_essence_allocation(
        &mut self,
        sender: impl ToString,
        user_and_essence_list: &[(impl ToString, EssenceInfo)],
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.voter_contract_address(),
                &ExecuteMsg::UpdateEssenceAllocation {
                    user_and_essence_list: user_and_essence_list
                        .iter()
                        .map(|(user, essence)| (user.to_string(), essence.to_owned()))
                        .collect(),
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
                Addr::unchecked(sender.to_string()),
                self.voter_contract_address(),
                &ExecuteMsg::SwapToEclipAstro {},
                &coins(amount, denom),
            )
            .map_err(parse_err)
    }

    fn voter_try_set_delegation(
        &mut self,
        sender: impl ToString,
        weight: &str,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.voter_contract_address(),
                &ExecuteMsg::SetDelegation {
                    weight: str_to_dec(weight),
                },
                &[],
            )
            .map_err(parse_err)
    }

    fn voter_try_place_vote(
        &mut self,
        sender: impl ToString,
        weight_allocation: &[WeightAllocationItem],
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.voter_contract_address(),
                &ExecuteMsg::PlaceVote {
                    weight_allocation: weight_allocation.to_owned(),
                },
                &[],
            )
            .map_err(parse_err)
    }

    fn voter_try_place_vote_as_dao(
        &mut self,
        sender: impl ToString,
        weight_allocation: &[WeightAllocationItem],
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.voter_contract_address(),
                &ExecuteMsg::PlaceVoteAsDao {
                    weight_allocation: weight_allocation.to_owned(),
                },
                &[],
            )
            .map_err(parse_err)
    }

    fn voter_try_update_route_list(
        &mut self,
        sender: impl ToString,
        route_list: &[RouteListItem],
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.voter_contract_address(),
                &ExecuteMsg::UpdateRouteList {
                    route_list: route_list.to_owned(),
                },
                &[],
            )
            .map_err(parse_err)
    }

    fn voter_try_push(&mut self) -> StdResult<AppResponse> {
        self.app
            .wasm_sudo(self.voter_contract_address(), &SudoMsg::Push {})
            .map_err(parse_err)
    }

    fn voter_try_claim_rewards(&mut self, sender: impl ToString) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.voter_contract_address(),
                &ExecuteMsg::ClaimRewards {},
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

    fn voter_query_rewards(&self) -> StdResult<Vec<(Uint128, String)>> {
        self.app
            .wrap()
            .query_wasm_smart(self.voter_contract_address(), &QueryMsg::Rewards {})
    }

    fn voter_query_bribes_allocation(&self) -> StdResult<Vec<BribesAllocationItem>> {
        self.app.wrap().query_wasm_smart(
            self.voter_contract_address(),
            &QueryMsg::BribesAllocation {},
        )
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

    fn voter_query_eclip_astro_minted_by_voter(&self) -> StdResult<Uint128> {
        self.app.wrap().query_wasm_smart(
            self.voter_contract_address(),
            &QueryMsg::EclipAstroMintedByVoter {},
        )
    }

    fn voter_query_user(
        &self,
        address: impl ToString,
        block_time: Option<u64>,
    ) -> StdResult<Vec<UserResponse>> {
        self.app.wrap().query_wasm_smart(
            self.voter_contract_address(),
            &QueryMsg::User {
                address: address.to_string(),
                block_time,
            },
        )
    }

    fn voter_query_user_list(
        &self,
        block_time: Option<u64>,
        amount: u32,
        start_from: Option<String>,
    ) -> StdResult<Vec<UserListResponse>> {
        self.app.wrap().query_wasm_smart(
            self.voter_contract_address(),
            &QueryMsg::UserList {
                block_time,
                amount,
                start_from,
            },
        )
    }

    fn voter_query_dao_info(&self, block_time: Option<u64>) -> StdResult<DaoResponse> {
        self.app.wrap().query_wasm_smart(
            self.voter_contract_address(),
            &QueryMsg::DaoInfo { block_time },
        )
    }

    fn voter_query_voter_info(&self, block_time: Option<u64>) -> StdResult<VoterInfoResponse> {
        self.app.wrap().query_wasm_smart(
            self.voter_contract_address(),
            &QueryMsg::VoterInfo { block_time },
        )
    }

    fn voter_query_epoch_info(&self) -> StdResult<EpochInfo> {
        self.app
            .wrap()
            .query_wasm_smart(self.voter_contract_address(), &QueryMsg::EpochInfo {})
    }

    fn voter_query_route_list(
        &self,
        amount: u32,
        start_from: Option<String>,
    ) -> StdResult<Vec<RouteListItem>> {
        self.app.wrap().query_wasm_smart(
            self.voter_contract_address(),
            &QueryMsg::RouteList { amount, start_from },
        )
    }
}
