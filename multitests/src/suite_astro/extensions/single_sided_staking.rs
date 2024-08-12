use cosmwasm_std::{coins, Addr, StdResult};
use cw_multi_test::{AppResponse, ContractWrapper, Executor};

use eclipse_base::error::parse_err;
use equinox_msg::single_sided_staking::{
    ExecuteMsg, QueryMsg, RewardConfig, TimeLockConfig, UserRewardByDuration, UserStaking,
};

use crate::suite_astro::helper::{Acc, ControllerHelper, Extension};

const NAME: &str = "single_sided_staking";

pub trait SingleSidedStakingExtension {
    fn single_sided_staking_code_id(&self) -> u64;
    fn single_sided_staking_contract_address(&self) -> Addr;

    fn single_sided_staking_prepare_contract(
        &mut self,
        eclip_astro: &str,
        timelock_config: &Option<Vec<TimeLockConfig>>,
        reward_config: &RewardConfig,
        token_converter: &Addr,
        treasury: &Addr,
        voter: &Addr,
    );

    fn single_sided_staking_try_stake(
        &mut self,
        sender: impl ToString,
        amount: u128,
        denom: &str,
        duration: u64,
        recipient: &Option<String>,
    ) -> StdResult<AppResponse>;

    fn single_sided_staking_query_staking(
        &self,
        user: impl ToString,
    ) -> StdResult<Vec<UserStaking>>;

    fn single_sided_staking_query_reward(
        &self,
        user: impl ToString,
    ) -> StdResult<Vec<UserRewardByDuration>>;
}

impl SingleSidedStakingExtension for ControllerHelper {
    fn single_sided_staking_code_id(&self) -> u64 {
        self.extension_list
            .iter()
            .find(|x| x.name == NAME)
            .unwrap()
            .code_id
    }

    fn single_sided_staking_contract_address(&self) -> Addr {
        self.extension_list
            .iter()
            .find(|x| x.name == NAME)
            .unwrap()
            .contract_address
            .to_owned()
    }

    fn single_sided_staking_prepare_contract(
        &mut self,
        eclip_astro: &str,
        timelock_config: &Option<Vec<TimeLockConfig>>,
        reward_config: &RewardConfig,
        token_converter: &Addr,
        treasury: &Addr,
        voter: &Addr,
    ) {
        let code_id = self.app.store_code(Box::new(
            ContractWrapper::new_with_empty(
                single_sided_staking::contract::execute,
                single_sided_staking::contract::instantiate,
                single_sided_staking::contract::query,
            )
            .with_migrate_empty(single_sided_staking::contract::migrate),
        ));

        let contract_address = self
            .app
            .instantiate_contract(
                code_id,
                self.acc(Acc::Owner),
                &equinox_msg::single_sided_staking::InstantiateMsg {
                    owner: self.acc(Acc::Owner),
                    token: eclip_astro.to_string(),
                    timelock_config: timelock_config.to_owned(),
                    rewards: reward_config.to_owned(),
                    token_converter: token_converter.to_owned(),
                    treasury: treasury.to_owned(),
                    voter: voter.to_string(),
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

    fn single_sided_staking_try_stake(
        &mut self,
        sender: impl ToString,
        amount: u128,
        denom: &str,
        duration: u64,
        recipient: &Option<String>,
    ) -> StdResult<AppResponse> {
        self.app
            .execute_contract(
                Addr::unchecked(sender.to_string()),
                self.single_sided_staking_contract_address(),
                &ExecuteMsg::Stake {
                    duration,
                    recipient: recipient.to_owned(),
                },
                &coins(amount, denom.to_string()),
            )
            .map_err(parse_err)
    }

    fn single_sided_staking_query_staking(
        &self,
        user: impl ToString,
    ) -> StdResult<Vec<UserStaking>> {
        self.app.wrap().query_wasm_smart(
            self.single_sided_staking_contract_address(),
            &QueryMsg::Staking {
                user: user.to_string(),
            },
        )
    }

    fn single_sided_staking_query_reward(
        &self,
        user: impl ToString,
    ) -> StdResult<Vec<UserRewardByDuration>> {
        self.app.wrap().query_wasm_smart(
            self.single_sided_staking_contract_address(),
            &QueryMsg::Reward {
                user: user.to_string(),
            },
        )
    }
}
