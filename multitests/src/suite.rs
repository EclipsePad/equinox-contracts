use anyhow::Result as AnyResult;
use astroport::{
    staking::{
        ConfigResponse as AstroStakingConfigResponse, Cw20HookMsg as AstroStakingCw20HookMsg,
        InstantiateMsg as AstroStakingInstantiateMsg, QueryMsg as AstroStakingQueryMsg,
    },
    token::{
        Cw20Coin, InstantiateMsg as AstroInstantiateMsg, MinterResponse as AstroportMinterResponse,
    },
};
use cosmwasm_std::{to_json_binary, Addr, StdResult, Uint128};
use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};
use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};
use equinox_msg::{
    flexible_staking::{
        ExecuteMsg as FlexibleStakingExecuteMsg, InstantiateMsg as FlexibleStakingInstantiateMsg,
        UpdateConfigMsg as FlexibleStakingUpdateConfig,
    },
    reward_distributor::{InstantiateMsg as RewardDistributorInstantiateMsg, LockingRewardConfig},
    timelock_staking::{
        ExecuteMsg as TimelockStakingExecuteMsg, InstantiateMsg as TimelockStakingInstantiateMsg,
        TimeLockConfig, UpdateConfigMsg as TimelockStakingUpdateConfig,
    },
    token_converter::{
        Config as ConverterConfig, Cw20HookMsg as ConverterCw20HookMsg,
        ExecuteMsg as ConverterExecuteMsg, InstantiateMsg as ConverterInstantiateMsg,
        QueryMsg as ConverterQueryMsg, RewardConfig as ConverterRewardConfig,
        RewardResponse as ConverterRewardResponse, UpdateConfig as ConverterUpdateConfig,
    },
    voter::InstantiateMsg as VoterInstantiateMsg,
};

fn store_astro(app: &mut App) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        astroport_token::contract::execute,
        astroport_token::contract::instantiate,
        astroport_token::contract::query,
    ));

    app.store_code(contract)
}

fn store_xastro(app: &mut App) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        astroport_xastro_token::contract::execute,
        astroport_xastro_token::contract::instantiate,
        astroport_xastro_token::contract::query,
    ));

    app.store_code(contract)
}

fn store_astro_staking(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(
            astroport_staking::contract::execute,
            astroport_staking::contract::instantiate,
            astroport_staking::contract::query,
        )
        .with_reply_empty(astroport_staking::contract::reply),
    );

    app.store_code(contract)
}

fn store_eclipastro(app: &mut App) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        eclipastro_token::contract::execute,
        eclipastro_token::contract::instantiate,
        eclipastro_token::contract::query,
    ));

    app.store_code(contract)
}

fn store_converter(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(
            token_converter::contract::execute,
            token_converter::contract::instantiate,
            token_converter::contract::query,
        )
        .with_reply_empty(token_converter::contract::reply),
    );

    app.store_code(contract)
}

fn store_flexible_staking(app: &mut App) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        flexible_staking::contract::execute,
        flexible_staking::contract::instantiate,
        flexible_staking::contract::query,
    ));

    app.store_code(contract)
}

fn store_timelock_staking(app: &mut App) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        timelock_staking::contract::execute,
        timelock_staking::contract::instantiate,
        timelock_staking::contract::query,
    ));

    app.store_code(contract)
}

fn store_reward_distributor(app: &mut App) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        reward_distributor::contract::execute,
        reward_distributor::contract::instantiate,
        reward_distributor::contract::query,
    ));

    app.store_code(contract)
}

fn store_voter(app: &mut App) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(
            voter::contract::execute,
            voter::contract::instantiate,
            voter::contract::query,
        )
        .with_reply_empty(voter::contract::reply),
    );

    app.store_code(contract)
}

#[derive(Debug)]

pub struct SuiteBuilder {
    pub admin: Option<String>,
    pub initial_balances: Vec<Cw20Coin>,
    pub timelock_config: Vec<TimeLockConfig>,
    pub eclip_daily_reward: Uint128,
    pub locking_reward_config: Vec<LockingRewardConfig>,
}

impl SuiteBuilder {
    pub fn new() -> Self {
        Self {
            admin: None,
            initial_balances: vec![],
            timelock_config: vec![],
            eclip_daily_reward: Uint128::zero(),
            locking_reward_config: vec![],
        }
    }

    pub fn with_initial_balances(mut self, balances: Vec<(&str, u128)>) -> Self {
        let initial_balances = balances
            .into_iter()
            .map(|(address, amount)| Cw20Coin {
                address: address.to_owned(),
                amount: amount.into(),
            })
            .collect::<Vec<Cw20Coin>>();
        self.initial_balances = initial_balances;
        self
    }

    pub fn with_timelock_config(mut self, config: Vec<(u64, u16)>) -> Self {
        let timelock_config = config
            .into_iter()
            .map(|(duration, early_unlock_penalty_bps)| TimeLockConfig {
                duration,
                early_unlock_penalty_bps,
            })
            .collect::<Vec<TimeLockConfig>>();
        self.timelock_config = timelock_config;
        self
    }

    pub fn with_eclip_daily_reward(mut self, daily_reward: Uint128) -> Self {
        self.eclip_daily_reward = daily_reward;
        self
    }

    pub fn with_locking_reward_config(mut self, config: Vec<(u64, u64)>) -> Self {
        let locking_reward_config = config
            .into_iter()
            .map(|(duration, multiplier)| LockingRewardConfig {
                duration,
                multiplier,
            })
            .collect::<Vec<LockingRewardConfig>>();
        self.locking_reward_config = locking_reward_config;
        self
    }

    #[track_caller]
    pub fn build(self) -> Suite {
        let mut app: App = App::default();

        let admin = Addr::unchecked("admin");
        let eclipse_treasury = Addr::unchecked("eclipse_treasury");
        let eclipse_stability_pool = Addr::unchecked("eclipse_stability_pool");
        let ce_reward_distributor = Addr::unchecked("ce_reward_distributor");

        let astro_id = store_astro(&mut app);
        let astro_contract = app
            .instantiate_contract(
                astro_id,
                admin.clone(),
                &AstroInstantiateMsg {
                    name: "astro token".to_owned(),
                    symbol: "ASTRO".to_owned(),
                    decimals: 6,
                    initial_balances: self.initial_balances,
                    mint: Some(AstroportMinterResponse {
                        minter: "minter".to_owned(),
                        cap: None,
                    }),
                    marketing: None,
                },
                &[],
                "ASTRO token",
                Some(admin.clone().to_string()),
            )
            .unwrap();

        let astro_staking_id = store_astro_staking(&mut app);
        let xastro_id = store_xastro(&mut app);
        let astro_staking_contract = app
            .instantiate_contract(
                astro_staking_id,
                admin.clone(),
                &AstroStakingInstantiateMsg {
                    owner: admin.clone().into_string(),
                    token_code_id: xastro_id,
                    deposit_token_addr: astro_contract.clone().into_string(),
                    marketing: None,
                },
                &[],
                "ASTRO staking",
                Some(admin.clone().to_string()),
            )
            .unwrap();

        let astro_staking_config: AstroStakingConfigResponse = app
            .wrap()
            .query_wasm_smart(
                astro_staking_contract.clone(),
                &AstroStakingQueryMsg::Config {},
            )
            .unwrap_or(AstroStakingConfigResponse {
                deposit_token_addr: astro_staking_contract.clone(),
                share_token_addr: Addr::unchecked(""),
            });
        let xastro_contract = astro_staking_config.share_token_addr;

        let eclipastro_id = store_eclipastro(&mut app);
        let converter_id = store_converter(&mut app);
        let converter_contract = app
            .instantiate_contract(
                converter_id,
                admin.clone(),
                &ConverterInstantiateMsg {
                    owner: admin.clone().into_string(),
                    token_in: astro_contract.clone().into_string(),
                    xtoken: xastro_contract.clone().into_string(),
                    treasury: eclipse_treasury.clone().into_string(),
                    token_code_id: eclipastro_id,
                    marketing: None,
                },
                &[],
                "converter",
                Some(admin.clone().to_string()),
            )
            .unwrap();
        let converter_config: ConverterConfig = app
            .wrap()
            .query_wasm_smart(converter_contract.clone(), &ConverterQueryMsg::Config {})
            .unwrap_or(ConverterConfig {
                token_in: Addr::unchecked(""),
                token_out: Addr::unchecked(""),
                xtoken: Addr::unchecked(""),
                treasury: Addr::unchecked(""),
                vxtoken_holder: Addr::unchecked(""),
                stability_pool: Addr::unchecked(""),
                staking_reward_distributor: Addr::unchecked(""),
                ce_reward_distributor: Addr::unchecked(""),
            });
        let eclipastro_contract = converter_config.token_out;

        let flexible_staking_id = store_flexible_staking(&mut app);
        let flexible_staking_contract = app
            .instantiate_contract(
                flexible_staking_id,
                admin.clone(),
                &FlexibleStakingInstantiateMsg {
                    owner: admin.clone().into_string(),
                    token: eclipastro_contract.clone().into_string(),
                },
                &[],
                "flexible staking",
                Some(admin.clone().to_string()),
            )
            .unwrap();

        let timelock_staking_id = store_timelock_staking(&mut app);
        let timelock_staking_contract = app
            .instantiate_contract(
                timelock_staking_id,
                admin.clone(),
                &TimelockStakingInstantiateMsg {
                    owner: admin.clone().into_string(),
                    token: eclipastro_contract.clone().into_string(),
                    timelock_config: self.timelock_config.clone(),
                },
                &[],
                "timelock staking",
                Some(admin.clone().to_string()),
            )
            .unwrap();

        let reward_distributor_id = store_reward_distributor(&mut app);
        let reward_distributor_contract = app
            .instantiate_contract(
                reward_distributor_id,
                admin.clone(),
                &RewardDistributorInstantiateMsg {
                    owner: admin.clone().into_string(),
                    eclipastro: eclipastro_contract.clone().into_string(),
                    eclip: "eclip".to_owned(),
                    flexible_staking: flexible_staking_contract.clone().into_string(),
                    timelock_staking: timelock_staking_contract.clone().into_string(),
                    token_converter: converter_contract.clone().into_string(),
                    eclip_daily_reward: self.eclip_daily_reward.clone(),
                    locking_reward_config: self.locking_reward_config.clone(),
                },
                &[],
                "reward distributor",
                Some(admin.clone().to_string()),
            )
            .unwrap();

        let voter_id = store_voter(&mut app);
        let voter_contract = app
            .instantiate_contract(
                voter_id,
                admin.clone(),
                &VoterInstantiateMsg {
                    owner: admin.clone().into_string(),
                    base_token: astro_contract.clone().into_string(),
                    xtoken: xastro_contract.clone().into_string(),
                    vxtoken: Addr::unchecked("vxastro").into_string(),
                    staking_contract: astro_staking_contract.clone().into_string(),
                    converter_contact: converter_contract.clone().into_string(),
                },
                &[],
                "voter",
                Some(admin.clone().to_string()),
            )
            .unwrap();

        Suite {
            app,
            admin,
            astro_contract,
            astro_staking_contract,
            xastro_contract,
            eclipastro_contract,
            converter_contract,
            flexible_staking_contract,
            timelock_staking_contract,
            reward_distributor_contract,
            voter_contract,
            eclipse_stability_pool,
            ce_reward_distributor,
            eclipse_treasury,
        }
    }
}

pub struct Suite {
    app: App,
    admin: Addr,
    astro_contract: Addr,
    astro_staking_contract: Addr,
    xastro_contract: Addr,
    eclipastro_contract: Addr,
    converter_contract: Addr,
    flexible_staking_contract: Addr,
    timelock_staking_contract: Addr,
    reward_distributor_contract: Addr,
    voter_contract: Addr,
    eclipse_stability_pool: Addr,
    ce_reward_distributor: Addr,
    eclipse_treasury: Addr,
}

impl Suite {
    pub fn admin(&self) -> String {
        self.admin.to_string()
    }
    pub fn astro_contract(&self) -> String {
        self.astro_contract.to_string()
    }
    pub fn astro_staking_contract(&self) -> String {
        self.astro_staking_contract.to_string()
    }
    pub fn xastro_contract(&self) -> String {
        self.xastro_contract.to_string()
    }
    pub fn eclipastro_contract(&self) -> String {
        self.eclipastro_contract.to_string()
    }
    pub fn converter_contract(&self) -> String {
        self.converter_contract.to_string()
    }
    pub fn flexible_staking_contract(&self) -> String {
        self.flexible_staking_contract.to_string()
    }
    pub fn timelock_staking_contract(&self) -> String {
        self.timelock_staking_contract.to_string()
    }
    pub fn reward_distributor_contract(&self) -> String {
        self.reward_distributor_contract.to_string()
    }
    pub fn voter_contract(&self) -> String {
        self.voter_contract.to_string()
    }
    pub fn eclipse_treasury(&self) -> String {
        self.eclipse_treasury.to_string()
    }
    pub fn eclipse_stability_pool(&self) -> String {
        self.eclipse_stability_pool.to_string()
    }
    pub fn eclipse_ce_reward_distributor(&self) -> String {
        self.ce_reward_distributor.to_string()
    }

    // update block's time to simulate passage of time
    pub fn update_time(&mut self, time_update: u64) {
        let mut block = self.app.block_info();
        block.time = block.time.plus_seconds(time_update);
        self.app.set_block(block);
    }

    pub fn update_config(&mut self) {
        self.app
            .execute_contract(
                self.admin.clone(),
                self.converter_contract.clone(),
                &ConverterExecuteMsg::UpdateConfig {
                    config: ConverterUpdateConfig {
                        token_in: None,
                        token_out: None,
                        xtoken: None,
                        vxtoken_holder: Some(self.voter_contract.clone().into_string()),
                        treasury: None,
                        stability_pool: Some(self.eclipse_stability_pool.clone().into_string()),
                        staking_reward_distributor: Some(
                            self.reward_distributor_contract.clone().into_string(),
                        ),
                        ce_reward_distributor: Some(
                            self.ce_reward_distributor.clone().into_string(),
                        ),
                    },
                },
                &[],
            )
            .unwrap();
        self.app
            .execute_contract(
                self.admin.clone(),
                self.flexible_staking_contract.clone(),
                &FlexibleStakingExecuteMsg::UpdateConfig {
                    config: FlexibleStakingUpdateConfig {
                        token: None,
                        reward_contract: Some(
                            self.reward_distributor_contract.clone().into_string(),
                        ),
                    },
                },
                &[],
            )
            .unwrap();
        self.app
            .execute_contract(
                self.admin.clone(),
                self.timelock_staking_contract.clone(),
                &TimelockStakingExecuteMsg::UpdateConfig {
                    config: TimelockStakingUpdateConfig {
                        token: None,
                        reward_contract: Some(
                            self.reward_distributor_contract.clone().into_string(),
                        ),
                        timelock_config: None,
                    },
                },
                &[],
            )
            .unwrap();
    }

    // xASTRO staking contract
    pub fn stake_astro(&mut self, sender: &str, amount: u128) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.astro_contract.clone(),
            &Cw20ExecuteMsg::Send {
                contract: self.astro_staking_contract(),
                amount: Uint128::from(amount),
                msg: to_json_binary(&AstroStakingCw20HookMsg::Enter {})?,
            },
            &[],
        )
    }

    pub fn send_astro(
        &mut self,
        sender: &str,
        recipient: &str,
        amount: u128,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.astro_contract.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: recipient.to_string(),
                amount: Uint128::from(amount),
            },
            &[],
        )
    }

    // query astro token amount
    pub fn query_astro_balance(&self, address: &str) -> StdResult<u128> {
        let balance: BalanceResponse = self.app.wrap().query_wasm_smart(
            self.astro_contract.clone(),
            &Cw20QueryMsg::Balance {
                address: address.to_owned(),
            },
        )?;
        Ok(balance.balance.u128())
    }

    // query xastro token amount
    pub fn query_xastro_balance(&self, address: &str) -> StdResult<u128> {
        let balance: BalanceResponse = self.app.wrap().query_wasm_smart(
            self.xastro_contract.clone(),
            &Cw20QueryMsg::Balance {
                address: address.to_owned(),
            },
        )?;
        Ok(balance.balance.u128())
    }

    // query astro staking total deposit
    pub fn query_astro_staking_total_deposit(&self) -> StdResult<u128> {
        self.app.wrap().query_wasm_smart(
            self.astro_staking_contract.clone(),
            &AstroStakingQueryMsg::TotalDeposit {},
        )
    }
    // query astro staking total shares
    pub fn query_astro_staking_total_shares(&self) -> StdResult<u128> {
        self.app.wrap().query_wasm_smart(
            self.astro_staking_contract.clone(),
            &AstroStakingQueryMsg::TotalShares {},
        )
    }

    // convert ASTRO to eclipASTRO in Convert contract
    pub fn convert_astro(&mut self, sender: &str, amount: u128) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.astro_contract.clone(),
            &Cw20ExecuteMsg::Send {
                contract: self.converter_contract(),
                amount: Uint128::from(amount),
                msg: to_json_binary(&ConverterCw20HookMsg::Convert {})?,
            },
            &[],
        )
    }

    pub fn update_converter_config(&mut self, config: ConverterUpdateConfig) {
        self.app
            .execute_contract(
                self.admin.clone(),
                self.converter_contract.clone(),
                &ConverterExecuteMsg::UpdateConfig { config },
                &[],
            )
            .unwrap();
    }

    pub fn update_reward_config(
        &mut self,
        config: ConverterRewardConfig,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            self.admin.clone(),
            self.converter_contract.clone(),
            &ConverterExecuteMsg::UpdateRewardConfig { config },
            &[],
        )
    }

    pub fn update_owner(&mut self, owner: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            self.admin.clone(),
            self.converter_contract.clone(),
            &ConverterExecuteMsg::UpdateOwner {
                owner: Addr::unchecked(owner).into_string(),
            },
            &[],
        )
    }

    pub fn claim_treasury_reward(&mut self, amount: u128) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            self.admin.clone(),
            self.converter_contract.clone(),
            &ConverterExecuteMsg::ClaimTreasuryReward {
                amount: Uint128::from(amount),
            },
            &[],
        )
    }

    pub fn claim(&mut self) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            self.reward_distributor_contract.clone(),
            self.converter_contract.clone(),
            &ConverterExecuteMsg::Claim {},
            &[],
        )
    }

    pub fn withdraw_xtoken(
        &mut self,
        sender: &str,
        amount: u128,
        recipient: &str,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.converter_contract.clone(),
            &ConverterExecuteMsg::WithdrawAvailableBalance {
                amount: Uint128::from(amount),
                recipient: recipient.to_string(),
            },
            &[],
        )
    }

    // query eclipastro token amount
    pub fn query_eclipastro_balance(&self, address: &str) -> StdResult<u128> {
        let balance: BalanceResponse = self.app.wrap().query_wasm_smart(
            self.eclipastro_contract.clone(),
            &Cw20QueryMsg::Balance {
                address: address.to_owned(),
            },
        )?;
        Ok(balance.balance.u128())
    }

    // query config of converter
    pub fn query_converter_config(&self) -> StdResult<ConverterConfig> {
        let config: ConverterConfig = self.app.wrap().query_wasm_smart(
            self.converter_contract.clone(),
            &ConverterQueryMsg::Config {},
        )?;
        Ok(config)
    }

    pub fn query_reward_config(&self) -> StdResult<ConverterRewardConfig> {
        let config: ConverterRewardConfig = self.app.wrap().query_wasm_smart(
            self.converter_contract.clone(),
            &ConverterQueryMsg::RewardConfig {},
        )?;
        Ok(config)
    }

    pub fn query_converter_owner(&self) -> StdResult<Addr> {
        let owner: Addr = self.app.wrap().query_wasm_smart(
            self.converter_contract.clone(),
            &ConverterQueryMsg::Owner {},
        )?;
        Ok(owner)
    }

    pub fn query_converter_reward(&self) -> StdResult<ConverterRewardResponse> {
        let reward: ConverterRewardResponse = self.app.wrap().query_wasm_smart(
            self.converter_contract.clone(),
            &ConverterQueryMsg::Rewards {},
        )?;
        Ok(reward)
    }

    pub fn query_withdrawable_balance(&self) -> StdResult<u128> {
        let reward: Uint128 = self.app.wrap().query_wasm_smart(
            self.converter_contract.clone(),
            &ConverterQueryMsg::WithdrawableBalance {  },
        )?;
        Ok(reward.u128())
    }
}
