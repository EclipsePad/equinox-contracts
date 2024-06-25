use cosmwasm_std::StdResult;
use cw_multi_test::{AppResponse, Executor};

use goplend_base::{
    adapters::scheduler::common::types::Log,
    error::parse_err,
    oracle::{
        msg::{ExecuteMsg, QueryMsg},
        types::{Config, PriceItem, RawPriceItem},
    },
};

use crate::helpers::suite::{
    core::Project,
    types::{ProjectAccount, ProjectNft},
};

use super::helper::ControllerHelper;

pub trait VoterExtension {
    fn oracle_try_remove_prices(
        &mut self,
        sender: ProjectAccount,
        collections: &[impl ToString],
    ) -> StdResult<AppResponse>;

    fn oracle_query_config(&self) -> StdResult<Config>;
}

impl VoterExtension for ControllerHelper {
    fn oracle_try_remove_prices(
        &mut self,
        sender: ProjectAccount,
        collections: &[impl ToString],
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                sender.into(),
                self.get_oracle_address(),
                &ExecuteMsg::RemovePrices {
                    collections: collections.iter().map(|x| x.to_string()).collect(),
                },
                &[],
            )
            .map_err(parse_err)
    }

    fn oracle_query_config(&self) -> StdResult<Config> {
        self.app
            .wrap()
            .query_wasm_smart(self.get_oracle_address(), &QueryMsg::QueryConfig {})
    }
}
