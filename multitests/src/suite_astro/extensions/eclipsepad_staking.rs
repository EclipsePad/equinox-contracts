use cosmwasm_std::{coins, Addr, StdResult, Uint128};
use cw_multi_test::{AppResponse, ContractWrapper, Executor};

use eclipse_base::{
    converters::str_to_dec,
    error::parse_err,
    staking::msg::{ExecuteMsg, QueryEssenceResponse, QueryMsg},
};

use crate::suite_astro::helper::{ControllerHelper, Extension};

const NAME: &str = "eclipsepad_staking";

pub trait EclipsepadStakingExtension {
    fn eclipsepad_staking_code_id(&self) -> u64;
    fn eclipsepad_staking_contract_address(&self) -> Addr;

    #[allow(clippy::too_many_arguments)]
    fn eclipsepad_staking_prepare_contract(
        &mut self,
        equinox_voter: Option<Addr>,
        beclip_minter: Option<Addr>,
        staking_token: Option<&str>,
        beclip_address: Option<Addr>,
        beclip_whitelist: Option<Vec<Addr>>,
        lock_schedule: Option<Vec<(u64, u64)>>,
        seconds_per_essence: Option<u128>,
        dao_treasury_address: Option<Addr>,
        penalty_multiplier: Option<&str>,
        pagintaion_config: Option<eclipse_base::staking::types::PaginationConfig>,
        eclip_per_second: Option<u64>,
        eclip_per_second_multiplier: Option<&str>,
    );

    fn eclipsepad_staking_try_stake(
        &mut self,
        sender: impl ToString,
        amount: u128,
        denom: &str,
    ) -> StdResult<AppResponse>;

    fn eclipsepad_staking_try_lock(
        &mut self,
        sender: impl ToString,
        amount: u128,
        lock_tier: u64,
    ) -> StdResult<AppResponse>;

    fn eclipsepad_staking_try_update_config(
        &mut self,
        sender: impl ToString,
        admin: Option<&str>,
        equinox_voter: Option<Addr>,
        beclip_minter: Option<Addr>,
        beclip_address: Option<Addr>,
        beclip_whitelist: Option<Vec<Addr>>,
        lock_schedule: Option<Vec<(u64, u64)>>,
        dao_treasury_address: Option<Addr>,
        penalty_multiplier: Option<&str>,
        eclip_per_second_multiplier: Option<&str>,
    ) -> StdResult<AppResponse>;

    fn eclipsepad_staking_query_essence(&self, user: &str) -> StdResult<QueryEssenceResponse>;

    fn eclipsepad_staking_query_total_essence(&self) -> StdResult<QueryEssenceResponse>;
}

impl EclipsepadStakingExtension for ControllerHelper {
    fn eclipsepad_staking_code_id(&self) -> u64 {
        self.extension_list
            .iter()
            .find(|x| x.name == NAME)
            .unwrap()
            .code_id
    }

    fn eclipsepad_staking_contract_address(&self) -> Addr {
        self.extension_list
            .iter()
            .find(|x| x.name == NAME)
            .unwrap()
            .contract_address
            .to_owned()
    }

    fn eclipsepad_staking_prepare_contract(
        &mut self,
        equinox_voter: Option<Addr>,
        beclip_minter: Option<Addr>,
        staking_token: Option<&str>,
        beclip_address: Option<Addr>,
        beclip_whitelist: Option<Vec<Addr>>,
        lock_schedule: Option<Vec<(u64, u64)>>,
        seconds_per_essence: Option<u128>,
        dao_treasury_address: Option<Addr>,
        penalty_multiplier: Option<&str>,
        pagintaion_config: Option<eclipse_base::staking::types::PaginationConfig>,
        eclip_per_second: Option<u64>,
        eclip_per_second_multiplier: Option<&str>,
    ) {
        let code_id = self.app.store_code(Box::new(
            ContractWrapper::new_with_empty(
                eclipsepad_staking::contract::execute,
                eclipsepad_staking::contract::instantiate,
                eclipsepad_staking::contract::query,
            )
            .with_migrate_empty(eclipsepad_staking::contract::migrate),
        ));

        let contract_address = self
            .app
            .instantiate_contract(
                code_id,
                self.owner.clone(),
                &eclipse_base::staking::msg::InstantiateMsg {
                    equinox_voter: equinox_voter.as_ref().map(|x| x.to_string()),
                    beclip_minter: beclip_minter.as_ref().map(|x| x.to_string()),
                    staking_token: staking_token.as_ref().map(|x| x.to_string()),
                    beclip_address: beclip_address.as_ref().map(|x| x.to_string()),
                    beclip_whitelist: beclip_whitelist
                        .as_ref()
                        .map(|x| x.into_iter().map(|y| y.to_string()).collect()),
                    lock_schedule: lock_schedule.to_owned(),
                    seconds_per_essence: seconds_per_essence
                        .as_ref()
                        .map(|x| Uint128::new(x.to_owned())),
                    dao_treasury_address: dao_treasury_address.as_ref().map(|x| x.to_string()),
                    penalty_multiplier: penalty_multiplier
                        .as_ref()
                        .map(|x| str_to_dec(&x.to_string())),
                    pagintaion_config: pagintaion_config.to_owned(),
                    eclip_per_second: eclip_per_second.to_owned(),
                    eclip_per_second_multiplier: eclip_per_second_multiplier
                        .as_ref()
                        .map(|x| str_to_dec(&x.to_string())),
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

    fn eclipsepad_staking_try_stake(
        &mut self,
        sender: impl ToString,
        amount: u128,
        denom: &str,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.eclipsepad_staking_contract_address(),
                &ExecuteMsg::Stake {},
                &coins(amount, denom),
            )
            .map_err(parse_err)
    }

    fn eclipsepad_staking_try_lock(
        &mut self,
        sender: impl ToString,
        amount: u128,
        lock_tier: u64,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.eclipsepad_staking_contract_address(),
                &ExecuteMsg::Lock {
                    amount: Uint128::new(amount),
                    lock_tier,
                },
                &[],
            )
            .map_err(parse_err)
    }

    fn eclipsepad_staking_try_update_config(
        &mut self,
        sender: impl ToString,
        admin: Option<&str>,
        equinox_voter: Option<Addr>,
        beclip_minter: Option<Addr>,
        beclip_address: Option<Addr>,
        beclip_whitelist: Option<Vec<Addr>>,
        lock_schedule: Option<Vec<(u64, u64)>>,
        dao_treasury_address: Option<Addr>,
        penalty_multiplier: Option<&str>,
        eclip_per_second_multiplier: Option<&str>,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.eclipsepad_staking_contract_address(),
                &eclipse_base::staking::msg::ExecuteMsg::UpdateConfig {
                    admin: admin.as_ref().map(|x| x.to_string()),
                    equinox_voter: equinox_voter.as_ref().map(|x| x.to_string()),
                    beclip_minter: beclip_minter.as_ref().map(|x| x.to_string()),
                    beclip_address: beclip_address.as_ref().map(|x| x.to_string()),
                    beclip_whitelist: beclip_whitelist
                        .as_ref()
                        .map(|x| x.into_iter().map(|y| y.to_string()).collect()),
                    lock_schedule: lock_schedule.to_owned(),
                    dao_treasury_address: dao_treasury_address.as_ref().map(|x| x.to_string()),
                    penalty_multiplier: penalty_multiplier
                        .as_ref()
                        .map(|x| str_to_dec(&x.to_string())),
                    eclip_per_second_multiplier: eclip_per_second_multiplier
                        .as_ref()
                        .map(|x| str_to_dec(&x.to_string())),
                },
                &[],
            )
            .map_err(parse_err)
    }

    fn eclipsepad_staking_query_essence(&self, user: &str) -> StdResult<QueryEssenceResponse> {
        self.app.wrap().query_wasm_smart(
            self.eclipsepad_staking_contract_address(),
            &QueryMsg::QueryEssence {
                user: user.to_string(),
            },
        )
    }

    fn eclipsepad_staking_query_total_essence(&self) -> StdResult<QueryEssenceResponse> {
        self.app.wrap().query_wasm_smart(
            self.eclipsepad_staking_contract_address(),
            &QueryMsg::QueryTotalEssence {},
        )
    }
}
