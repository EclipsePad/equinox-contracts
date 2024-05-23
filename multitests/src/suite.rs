use anyhow::Result as AnyResult;
use astroport::{
    asset::{Asset, AssetInfo, PairInfo},
    factory::{PairConfig, PairType, QueryMsg as FactoryQueryMsg},
    incentives::{self, ExecuteMsg as IncentivesExecuteMsg, QueryMsg as IncentivesQueryMsg},
    pair::ExecuteMsg as PairExecuteMsg,
    staking::{
        Config as AstroStakingConfig, ExecuteMsg as AstroStakingExecuteMsg,
        InstantiateMsg as AstroStakingInstantiateMsg, QueryMsg as AstroStakingQueryMsg,
    },
    vesting::{self, ExecuteMsg as VestingExecuteMsg, VestingAccount},
};
use cosmwasm_std::{
    coin, coins, testing::MockApi, to_json_binary, Addr, Decimal, DepsMut, Empty, Env, GovMsg,
    IbcMsg, IbcQuery, MemoryStorage, MessageInfo, Response, StdResult, Uint128,
};
use cw20::{BalanceResponse, Cw20Coin, Cw20ExecuteMsg, Cw20QueryMsg, MinterResponse};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
use cw_multi_test::{
    App, AppResponse, BankKeeper, BasicAppBuilder, ContractWrapper, DistributionKeeper, Executor,
    FailingModule, StakeKeeper, WasmKeeper,
};
use equinox_msg::{
    lockdrop::{
        Config as LockdropConfig, Cw20HookMsg as LockdropCw20HookMsg,
        ExecuteMsg as LockdropExecuteMsg, InstantiateMsg as LockdropInstantiateMsg,
        LpLockupInfoResponse, LpLockupStateResponse, QueryMsg as LockdropQueryMsg,
        SingleLockupInfoResponse, SingleLockupStateResponse, StakeType,
        UpdateConfigMsg as LockdropUpdateConfigMsg, UserLpLockupInfoResponse,
        UserSingleLockupInfoResponse,
    },
    lp_staking::{
        Config as LpStakingConfig, Cw20HookMsg as LpStakingCw20HookMsg,
        ExecuteMsg as LpStakingExecuteMsg, InstantiateMsg as LpStakingInstantiateMsg,
        QueryMsg as LpStakingQueryMsg, RewardAmount as LpStakingRewardAmount,
        RewardWeight as LpStakingRewardWeight, UpdateConfigMsg as LpStakingUpdateConfigMsg,
        UserStaking as LpStakingUserStaking,
    },
    single_sided_staking::{
        Config as SingleStakingConfig, Cw20HookMsg as SingleStakingCw20HookMsg,
        ExecuteMsg as SingleSidedStakingExecuteMsg,
        InstantiateMsg as SingleSidedStakingInstantiateMsg, QueryMsg as SingleStakingQueryMsg,
        UpdateConfigMsg as SingleStakingUpdateConfigMsg,
        UserRewardByDuration as SingleStakingUserRewardByDuration,
        UserStaking as SingleSidedUserStaking,
    },
    token_converter::{
        Config as ConverterConfig, ExecuteMsg as ConverterExecuteMsg,
        InstantiateMsg as ConverterInstantiateMsg, QueryMsg as ConverterQueryMsg,
        RewardResponse as ConverterRewardResponse, StakeInfo as ConverterStakeInfo,
        UpdateConfig as ConverterUpdateConfig,
    },
    voter::InstantiateMsg as VoterInstantiateMsg,
};

use crate::common::stargate::StargateKeeper;

fn store_astro_staking(app: &mut CustomizedApp) -> u64 {
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

fn store_tracking_code(app: &mut CustomizedApp) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(
            |_: DepsMut, _: Env, _: MessageInfo, _: Empty| -> StdResult<Response> {
                unimplemented!()
            },
            astroport_tokenfactory_tracker::contract::instantiate,
            astroport_tokenfactory_tracker::query::query,
        )
        .with_sudo_empty(astroport_tokenfactory_tracker::contract::sudo),
    );

    app.store_code(contract)
}

fn store_astroport_token(app: &mut CustomizedApp) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    ));

    app.store_code(contract)
}

fn store_astroport_pair(app: &mut CustomizedApp) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(
            astroport_pair::contract::execute,
            astroport_pair::contract::instantiate,
            astroport_pair::contract::query,
        )
        .with_reply_empty(astroport_pair::contract::reply),
    );

    app.store_code(contract)
}

fn store_astroport_factory(app: &mut CustomizedApp) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(
            astroport_factory::contract::execute,
            astroport_factory::contract::instantiate,
            astroport_factory::contract::query,
        )
        .with_reply_empty(astroport_factory::contract::reply),
    );

    app.store_code(contract)
}

fn store_astroport_generator(app: &mut CustomizedApp) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(
            astroport_incentives::execute::execute,
            astroport_incentives::instantiate::instantiate,
            astroport_incentives::query::query,
        )
        .with_reply_empty(astroport_incentives::reply::reply),
    );

    app.store_code(contract)
}

fn store_astroport_vesting(app: &mut CustomizedApp) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        astroport_vesting::contract::execute,
        astroport_vesting::contract::instantiate,
        astroport_vesting::contract::query,
    ));

    app.store_code(contract)
}

fn store_eclipastro(app: &mut CustomizedApp) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        eclipastro_token::contract::execute,
        eclipastro_token::contract::instantiate,
        eclipastro_token::contract::query,
    ));

    app.store_code(contract)
}

fn store_converter(app: &mut CustomizedApp) -> u64 {
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

fn store_voter(app: &mut CustomizedApp) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        voter::contract::execute,
        voter::contract::instantiate,
        voter::contract::query,
    ));

    app.store_code(contract)
}

fn store_lp_staking(app: &mut CustomizedApp) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        lp_staking::contract::execute,
        lp_staking::contract::instantiate,
        lp_staking::contract::query,
    ));

    app.store_code(contract)
}

fn store_single_staking(app: &mut CustomizedApp) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        single_sided_staking::contract::execute,
        single_sided_staking::contract::instantiate,
        single_sided_staking::contract::query,
    ));

    app.store_code(contract)
}

fn store_lockdrop(app: &mut CustomizedApp) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        lockdrop::contract::execute,
        lockdrop::contract::instantiate,
        lockdrop::contract::query,
    ));

    app.store_code(contract)
}

#[derive(Debug)]
pub struct SuiteBuilder {
    pub admin: Option<String>,
}

pub const ASTRO_DENOM: &str = "factory/admin/astro";
pub const ADMIN: &str = "admin";
pub const TREASURY: &str = "treasury";
pub const VXASTRO: &str = "vxastro";
pub const STABILITY_POOL_REWARD_HOLDER: &str = "stability_pool_reward_holder";
pub const CE_REWARD_HOLDER: &str = "ce_reward_holder";

pub type CustomizedApp = App<
    BankKeeper,
    MockApi,
    MemoryStorage,
    FailingModule<Empty, Empty, Empty>,
    WasmKeeper<Empty, Empty>,
    StakeKeeper,
    DistributionKeeper,
    FailingModule<IbcMsg, IbcQuery, Empty>,
    FailingModule<GovMsg, Empty, Empty>,
    StargateKeeper,
>;

impl SuiteBuilder {
    pub fn new() -> Self {
        Self { admin: None }
    }

    #[track_caller]
    pub fn build(self) -> Suite {
        let admin = Addr::unchecked(ADMIN);

        let mut app = BasicAppBuilder::new()
            .with_stargate(StargateKeeper::default())
            .build(|router, _, storage| {
                router
                    .bank
                    .init_balance(storage, &admin, coins(u128::MAX, ASTRO_DENOM))
                    .unwrap()
            });

        let tracking_code_id = store_tracking_code(&mut app);

        let astro_staking_id = store_astro_staking(&mut app);

        let astro_staking_contract = app
            .instantiate_contract(
                astro_staking_id,
                admin.clone(),
                &AstroStakingInstantiateMsg {
                    deposit_token_denom: ASTRO_DENOM.to_string(),
                    tracking_admin: admin.clone().into_string(),
                    tracking_code_id: tracking_code_id,
                    token_factory_addr: admin.clone().into_string(),
                },
                &[coin(1u128, ASTRO_DENOM)],
                "ASTRO staking",
                Some(ADMIN.to_string()),
            )
            .unwrap();

        let config: AstroStakingConfig = app
            .wrap()
            .query_wasm_smart(
                astro_staking_contract.clone(),
                &AstroStakingQueryMsg::Config {},
            )
            .unwrap();
        let xastro = config.xastro_denom;

        let cw20_token_code_id = store_astroport_token(&mut app);
        let pair_code_id = store_astroport_pair(&mut app);
        let factory_code_id = store_astroport_factory(&mut app);
        let msg = astroport::factory::InstantiateMsg {
            pair_configs: vec![PairConfig {
                code_id: pair_code_id,
                pair_type: PairType::Xyk {},
                total_fee_bps: 30,
                maker_fee_bps: 3333,
                is_disabled: false,
                is_generator_disabled: false,
                permissioned: false,
            }],
            token_code_id: cw20_token_code_id,
            fee_address: None,
            generator_address: None,
            owner: admin.to_string(),
            whitelist_code_id: 0,
            coin_registry_address: "coin_registry".to_string(),
        };
        let astroport_factory = app
            .instantiate_contract(
                factory_code_id,
                admin.clone(),
                &msg,
                &[],
                String::from("Astroport Factory"),
                None,
            )
            .unwrap();
        let vesting_code_id = store_astroport_vesting(&mut app);
        let astroport_vesting = app
            .instantiate_contract(
                vesting_code_id,
                admin.clone(),
                &vesting::InstantiateMsg {
                    owner: admin.to_string(),
                    vesting_token: AssetInfo::NativeToken {
                        denom: ASTRO_DENOM.to_string(),
                    },
                },
                &[],
                "Astroport Vesting",
                None,
            )
            .unwrap();
        let generator_code_id = store_astroport_generator(&mut app);
        let astroport_generator = app
            .instantiate_contract(
                generator_code_id,
                admin.clone(),
                &incentives::InstantiateMsg {
                    owner: admin.to_string(),
                    factory: astroport_factory.to_string(),
                    astro_token: AssetInfo::NativeToken {
                        denom: ASTRO_DENOM.to_string(),
                    },
                    vesting_contract: astroport_vesting.to_string(),
                    incentivization_fee_info: None,
                    guardian: None,
                },
                &[],
                "Astroport Generator",
                None,
            )
            .unwrap();
        let eclipastro_id = store_eclipastro(&mut app);
        let converter_id = store_converter(&mut app);
        let converter_contract = app
            .instantiate_contract(
                converter_id,
                admin.clone(),
                &ConverterInstantiateMsg {
                    owner: admin.clone().into_string(),
                    astro: ASTRO_DENOM.to_string(),
                    xastro: xastro.clone(),
                    treasury: Addr::unchecked(TREASURY.to_string()).to_string(),
                    staking_contract: astro_staking_contract.clone(),
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
            .unwrap();
        let eclipastro = converter_config.eclipastro;
        let beclip_id = store_astroport_token(&mut app);
        let beclip = app
            .instantiate_contract(
                beclip_id,
                admin.clone(),
                &Cw20InstantiateMsg {
                    name: "bECLIP token".to_string(),
                    symbol: "bECLIP".to_string(),
                    marketing: None,
                    decimals: 6,
                    initial_balances: vec![Cw20Coin {
                        address: admin.to_string(),
                        amount: Uint128::from(1_000_000_000_000u128),
                    }],
                    mint: Some(MinterResponse {
                        minter: admin.to_string(),
                        cap: None,
                    }),
                },
                &[],
                "converter",
                Some(admin.clone().to_string()),
            )
            .unwrap();

        let single_staking_id = store_single_staking(&mut app);
        let single_staking_contract = app
            .instantiate_contract(
                single_staking_id,
                admin.clone(),
                &SingleSidedStakingInstantiateMsg {
                    owner: admin.clone(),
                    beclip: AssetInfo::Token {
                        contract_addr: beclip.clone(),
                    },
                    token: eclipastro.clone(),
                    timelock_config: None,
                    token_converter: converter_contract.clone(),
                    beclip_daily_reward: None,
                    treasury: Addr::unchecked(TREASURY.to_string()),
                },
                &[],
                "Single Sided Staking",
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
                    astro: ASTRO_DENOM.to_string(),
                    xastro: xastro.clone(),
                    vxastro: Addr::unchecked(VXASTRO.to_string()).to_string(),
                    staking_contract: astro_staking_contract.clone().into_string(),
                    converter_contract: converter_contract.clone().into_string(),
                },
                &[],
                "voter",
                Some(admin.clone().to_string()),
            )
            .unwrap();

        let asset_infos = vec![
            AssetInfo::Token {
                contract_addr: eclipastro.clone(),
            },
            AssetInfo::NativeToken {
                denom: xastro.clone(),
            },
        ];

        let msg = astroport::factory::ExecuteMsg::CreatePair {
            pair_type: PairType::Xyk {},
            asset_infos,
            init_params: None,
        };

        app.execute_contract(admin.clone(), astroport_factory.clone(), &msg, &[])
            .unwrap();

        let info: PairInfo = app
            .wrap()
            .query_wasm_smart(
                astroport_factory.clone(),
                &FactoryQueryMsg::Pair {
                    asset_infos: vec![
                        AssetInfo::Token {
                            contract_addr: eclipastro.clone(),
                        },
                        AssetInfo::NativeToken {
                            denom: xastro.clone(),
                        },
                    ],
                },
            )
            .unwrap();
        let eclipastro_xastro_lp_contract = info.contract_addr;
        let eclipastro_xastro_lp_token_contract = info.liquidity_token;

        let lp_staking_code_id = store_lp_staking(&mut app);
        let lp_staking_contract = app
            .instantiate_contract(
                lp_staking_code_id,
                admin.clone(),
                &LpStakingInstantiateMsg {
                    lp_token: eclipastro_xastro_lp_token_contract.clone(),
                    lp_contract: eclipastro_xastro_lp_contract.clone(),
                    beclip: AssetInfo::Token {
                        contract_addr: beclip.clone(),
                    },
                    astro: ASTRO_DENOM.to_string(),
                    xastro: xastro.clone(),
                    astro_staking: astro_staking_contract.clone(),
                    converter: converter_contract.clone(),
                    beclip_daily_reward: None,
                    astroport_generator: astroport_generator.clone(),
                    treasury: Addr::unchecked(TREASURY.to_string()),
                    stability_pool: Some(Addr::unchecked(STABILITY_POOL_REWARD_HOLDER.to_string())),
                    ce_reward_distributor: Some(Addr::unchecked(CE_REWARD_HOLDER.to_string())),
                },
                &[],
                "Eclipsefi lp staking",
                None,
            )
            .unwrap();

        let lockdrop_code_id = store_lockdrop(&mut app);
        let init_timestamp = app.block_info().time.seconds() + 86400;
        let lockdrop_contract = app
            .instantiate_contract(
                lockdrop_code_id,
                admin.clone(),
                &LockdropInstantiateMsg {
                    init_timestamp: init_timestamp,
                    deposit_window: None,
                    withdrawal_window: None,
                    lock_configs: None,
                    astro_token: ASTRO_DENOM.to_string(),
                    xastro_token: xastro.clone(),
                    eclipastro_token: eclipastro.clone(),
                    astro_staking: astro_staking_contract.clone(),
                    converter: converter_contract.clone(),
                    liquidity_pool: eclipastro_xastro_lp_contract.clone(),
                    owner: None,
                    beclip: AssetInfo::Token {
                        contract_addr: beclip.clone(),
                    },
                    single_sided_staking: single_staking_contract.clone(),
                    lp_staking: lp_staking_contract.clone(),
                    dao_treasury_address: Addr::unchecked(TREASURY.to_string()),
                },
                &[],
                "Eclipsefi lockdrop",
                None,
            )
            .unwrap();

        let eclipse_stability_pool = Addr::unchecked(STABILITY_POOL_REWARD_HOLDER);
        let ce_reward_distributor = Addr::unchecked(CE_REWARD_HOLDER);
        let treasury = Addr::unchecked(TREASURY);

        Suite {
            app,
            admin,
            astro: ASTRO_DENOM.to_string(),
            xastro,
            astro_staking_contract,
            eclipastro,
            converter_contract,
            beclip,
            single_staking_contract,
            lp_staking_contract,
            lockdrop_contract,
            voter_contract,
            eclipse_stability_pool,
            ce_reward_distributor,
            eclipastro_xastro_lp_contract,
            eclipastro_xastro_lp_token_contract,
            astroport_generator,
            astroport_vesting,
            treasury,
        }
    }
}

pub struct Suite {
    app: CustomizedApp,
    admin: Addr,
    astro: String,
    astro_staking_contract: Addr,
    xastro: String,
    eclipastro: Addr,
    converter_contract: Addr,
    beclip: Addr,
    single_staking_contract: Addr,
    lp_staking_contract: Addr,
    lockdrop_contract: Addr,
    voter_contract: Addr,
    eclipse_stability_pool: Addr,
    ce_reward_distributor: Addr,
    eclipastro_xastro_lp_contract: Addr,
    eclipastro_xastro_lp_token_contract: Addr,
    astroport_generator: Addr,
    astroport_vesting: Addr,
    treasury: Addr,
}

impl Suite {
    pub fn admin(&self) -> String {
        self.admin.to_string()
    }
    pub fn astro(&self) -> String {
        self.astro.clone()
    }
    pub fn astro_staking_contract(&self) -> String {
        self.astro_staking_contract.to_string()
    }
    pub fn xastro(&self) -> String {
        self.xastro.clone()
    }
    pub fn eclipastro(&self) -> String {
        self.eclipastro.to_string()
    }
    pub fn converter_contract(&self) -> String {
        self.converter_contract.to_string()
    }
    pub fn beclip(&self) -> String {
        self.beclip.to_string()
    }
    pub fn single_staking_contract(&self) -> String {
        self.single_staking_contract.to_string()
    }
    pub fn lp_staking_contract(&self) -> String {
        self.lp_staking_contract.to_string()
    }
    pub fn lockdrop_contract(&self) -> String {
        self.lockdrop_contract.to_string()
    }
    pub fn voter_contract(&self) -> String {
        self.voter_contract.to_string()
    }
    pub fn eclipastro_xastro_lp_contract(&self) -> String {
        self.eclipastro_xastro_lp_contract.to_string()
    }
    pub fn eclipastro_xastro_lp_token_contract(&self) -> String {
        self.eclipastro_xastro_lp_token_contract.to_string()
    }
    pub fn astroport_generator(&self) -> String {
        self.astroport_generator.to_string()
    }
    pub fn astroport_vesting(&self) -> String {
        self.astroport_vesting.to_string()
    }
    pub fn ce_reward_distributor(&self) -> String {
        self.ce_reward_distributor.to_string()
    }
    pub fn eclipse_stability_pool(&self) -> String {
        self.eclipse_stability_pool.to_string()
    }
    pub fn treasury(&self) -> String {
        self.treasury.to_string()
    }

    // update block's time to simulate passage of time
    pub fn update_time(&mut self, time_update: u64) {
        let mut block = self.app.block_info();
        block.time = block.time.plus_seconds(time_update);
        self.app.set_block(block);
    }

    // get block's time
    pub fn get_time(&mut self) -> u64 {
        let block = self.app.block_info();
        block.time.seconds()
    }

    pub fn update_config(&mut self) {
        self.app
            .execute_contract(
                self.admin.clone(),
                self.converter_contract.clone(),
                &ConverterExecuteMsg::UpdateConfig {
                    config: ConverterUpdateConfig {
                        vxastro_holder: Some(self.voter_contract.clone()),
                        treasury: None,
                        stability_pool: Some(self.eclipse_stability_pool.clone()),
                        single_staking_contract: Some(self.single_staking_contract.clone()),
                        ce_reward_distributor: Some(self.ce_reward_distributor.clone()),
                    },
                },
                &[],
            )
            .unwrap();
        self.app
            .execute_contract(
                self.admin.clone(),
                self.single_staking_contract.clone(),
                &SingleSidedStakingExecuteMsg::AllowUsers {
                    users: vec![self.lockdrop_contract.to_string()],
                },
                &[],
            )
            .unwrap();
    }

    pub fn mint_native(
        &mut self,
        recipient: String,
        denom: String,
        amount: u128,
    ) -> AnyResult<AppResponse> {
        self.app.sudo(cw_multi_test::SudoMsg::Bank(
            cw_multi_test::BankSudo::Mint {
                to_address: recipient,
                amount: vec![coin(amount, denom)],
            },
        ))
    }

    pub fn query_balance_native(&mut self, recipient: String, denom: String) -> StdResult<u128> {
        let balance = self
            .app
            .wrap()
            .query_balance(recipient, denom)
            .unwrap()
            .amount;
        Ok(balance.u128())
    }

    pub fn stake_astro(&mut self, sender: &str, amount: u128) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.astro_staking_contract.clone(),
            &AstroStakingExecuteMsg::Enter { receiver: None },
            &[coin(amount, self.astro.clone())],
        )
    }

    pub fn query_astro_staking_data(&mut self) -> StdResult<(Uint128, Uint128)> {
        let total_deposit = self
            .app
            .wrap()
            .query_wasm_smart(
                self.astro_staking_contract.clone(),
                &AstroStakingQueryMsg::TotalDeposit {},
            )
            .unwrap();
        let total_shares = self
            .app
            .wrap()
            .query_wasm_smart(
                self.astro_staking_contract.clone(),
                &AstroStakingQueryMsg::TotalShares {},
            )
            .unwrap();
        Ok((total_deposit, total_shares))
    }

    // convert ASTRO to eclipASTRO in Convert contract
    pub fn convert_astro(&mut self, sender: &str, amount: u128) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.converter_contract.clone(),
            &ConverterExecuteMsg::Convert { recipient: None },
            &[coin(amount, self.astro())],
        )
    }

    pub fn convert_xastro(&mut self, sender: &str, amount: u128) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.converter_contract.clone(),
            &ConverterExecuteMsg::Convert { recipient: None },
            &[coin(amount, self.xastro())],
        )
    }

    pub fn query_eclipastro_balance(&self, address: &str) -> StdResult<u128> {
        let balance: BalanceResponse = self.app.wrap().query_wasm_smart(
            self.eclipastro.clone(),
            &Cw20QueryMsg::Balance {
                address: address.to_owned(),
            },
        )?;
        Ok(balance.balance.u128())
    }

    pub fn query_converter_withdrawable_balance(&self) -> StdResult<u128> {
        let balance: Uint128 = self.app.wrap().query_wasm_smart(
            self.converter_contract.clone(),
            &ConverterQueryMsg::WithdrawableBalance {},
        )?;
        Ok(balance.u128())
    }

    pub fn query_converter_rewards(&self) -> StdResult<ConverterRewardResponse> {
        let rewards: ConverterRewardResponse = self.app.wrap().query_wasm_smart(
            self.converter_contract.clone(),
            &ConverterQueryMsg::Rewards {},
        )?;
        Ok(rewards)
    }

    pub fn query_converter_stake_info(&self) -> StdResult<ConverterStakeInfo> {
        let info: ConverterStakeInfo = self.app.wrap().query_wasm_smart(
            self.converter_contract.clone(),
            &ConverterQueryMsg::StakeInfo {},
        )?;
        Ok(info)
    }
    pub fn query_converter_config(&self) -> StdResult<ConverterConfig> {
        let info: ConverterConfig = self.app.wrap().query_wasm_smart(
            self.converter_contract.clone(),
            &ConverterQueryMsg::Config {},
        )?;
        Ok(info)
    }

    pub fn provide_liquidity(
        &mut self,
        sender: &str,
        pair: Addr,
        assets: Vec<Asset>,
        receiver: Option<String>,
        slippage_tolerance: Option<Decimal>,
    ) -> AnyResult<AppResponse> {
        let mut coins = vec![];
        for asset in &assets {
            if !asset.is_native_token() {
                let increase_allowance_msg = Cw20ExecuteMsg::IncreaseAllowance {
                    spender: pair.to_string(),
                    amount: asset.amount,
                    expires: None,
                };
                let _ = self.app.execute_contract(
                    Addr::unchecked(sender),
                    Addr::unchecked(asset.info.to_string()),
                    &increase_allowance_msg,
                    &[],
                );
            } else {
                coins.push(coin(asset.amount.u128(), asset.info.to_string()));
            }
        }
        let msg = PairExecuteMsg::ProvideLiquidity {
            assets,
            slippage_tolerance,
            auto_stake: None,
            receiver,
        };

        self.app
            .execute_contract(Addr::unchecked(sender), pair.clone(), &msg, &coins)
    }
    pub fn setup_pools(
        &mut self,
        sender: &str,
        pools: Vec<(String, Uint128)>,
    ) -> AnyResult<AppResponse> {
        let msg = IncentivesExecuteMsg::SetupPools { pools };

        self.app.execute_contract(
            Addr::unchecked(sender),
            self.astroport_generator.clone(),
            &msg,
            &[],
        )
    }
    pub fn generator_set_tokens_per_second(
        &mut self,
        sender: &str,
        amount: u128,
    ) -> AnyResult<AppResponse> {
        let msg = IncentivesExecuteMsg::SetTokensPerSecond {
            amount: Uint128::from(amount),
        };

        self.app.execute_contract(
            Addr::unchecked(sender),
            self.astroport_generator.clone(),
            &msg,
            &[],
        )
    }
    pub fn register_vesting_accounts(
        &mut self,
        sender: &str,
        vesting_accounts: Vec<VestingAccount>,
        amount: u128,
    ) -> AnyResult<AppResponse> {
        let msg = VestingExecuteMsg::RegisterVestingAccounts { vesting_accounts };

        self.app.execute_contract(
            Addr::unchecked(sender),
            self.astroport_vesting.clone(),
            &msg,
            &[coin(amount, self.astro())],
        )
    }

    pub fn query_lp_staking_config(&self) -> StdResult<LpStakingConfig> {
        let config: LpStakingConfig = self.app.wrap().query_wasm_smart(
            self.lp_staking_contract.clone(),
            &LpStakingQueryMsg::Config {},
        )?;
        Ok(config)
    }
    pub fn lp_staking_update_config(
        &mut self,
        sender: &str,
        new_config: LpStakingUpdateConfigMsg,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lp_staking_contract.clone(),
            &LpStakingExecuteMsg::UpdateConfig { config: new_config },
            &[],
        )
    }
    pub fn query_lp_token_balance(&self, address: &str) -> StdResult<Uint128> {
        let res: BalanceResponse = self.app.wrap().query_wasm_smart(
            self.eclipastro_xastro_lp_token_contract.clone(),
            &Cw20QueryMsg::Balance {
                address: address.to_string(),
            },
        )?;
        Ok(res.balance)
    }
    pub fn query_incentive_pending_rewards(&self, address: &str) -> StdResult<Vec<Asset>> {
        let res: Vec<Asset> = self.app.wrap().query_wasm_smart(
            self.astroport_generator.clone(),
            &IncentivesQueryMsg::PendingRewards {
                lp_token: self.eclipastro_xastro_lp_token_contract(),
                user: address.to_string(),
            },
        )?;
        Ok(res)
    }

    pub fn query_incentive_deposit(&self, lp_token: &str, address: &str) -> StdResult<Uint128> {
        let res: Uint128 = self.app.wrap().query_wasm_smart(
            self.astroport_generator.clone(),
            &IncentivesQueryMsg::Deposit {
                lp_token: lp_token.to_string(),
                user: address.to_string(),
            },
        )?;
        Ok(res)
    }
    pub fn stake_lp_token(&mut self, sender: &str, amount: u128) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.eclipastro_xastro_lp_token_contract.clone(),
            &Cw20ExecuteMsg::Send {
                contract: self.lp_staking_contract.to_string(),
                amount: Uint128::from(amount),
                msg: to_json_binary(&LpStakingCw20HookMsg::Stake {})?,
            },
            &[],
        )
    }
    pub fn query_user_lp_token_staking(&self, user: &str) -> StdResult<LpStakingUserStaking> {
        let res: LpStakingUserStaking = self.app.wrap().query_wasm_smart(
            self.lp_staking_contract.clone(),
            &LpStakingQueryMsg::Staking {
                user: user.to_string(),
            },
        )?;
        Ok(res)
    }
    pub fn query_user_lp_token_rewards(&self, user: &str) -> StdResult<Vec<LpStakingRewardAmount>> {
        let res: Vec<LpStakingRewardAmount> = self.app.wrap().query_wasm_smart(
            self.lp_staking_contract.clone(),
            &LpStakingQueryMsg::Reward {
                user: user.to_string(),
            },
        )?;
        Ok(res)
    }

    pub fn query_total_lp_token_staking(&self) -> StdResult<Uint128> {
        let res: Uint128 = self.app.wrap().query_wasm_smart(
            self.lp_staking_contract.clone(),
            &LpStakingQueryMsg::TotalStaking {},
        )?;
        Ok(res)
    }
    pub fn query_reward_weights(&self) -> StdResult<Vec<LpStakingRewardWeight>> {
        let res: Vec<LpStakingRewardWeight> = self.app.wrap().query_wasm_smart(
            self.lp_staking_contract.clone(),
            &LpStakingQueryMsg::RewardWeights {},
        )?;
        Ok(res)
    }
    pub fn query_user_lp_staking_reward(
        &self,
        user: &str,
    ) -> StdResult<Vec<LpStakingRewardAmount>> {
        let res: Vec<LpStakingRewardAmount> = self.app.wrap().query_wasm_smart(
            self.lp_staking_contract.clone(),
            &LpStakingQueryMsg::Reward {
                user: user.to_string(),
            },
        )?;
        Ok(res)
    }
    pub fn lp_staking_claim_rewards(&mut self, sender: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lp_staking_contract.clone(),
            &LpStakingExecuteMsg::Claim {},
            &[],
        )
    }
    pub fn mint_beclip(&mut self, recipient: &str, amount: u128) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            self.admin.clone(),
            self.beclip.clone(),
            &Cw20ExecuteMsg::Mint {
                recipient: recipient.to_string(),
                amount: Uint128::from(amount),
            },
            &[],
        )
    }
    pub fn query_beclip_balance(&self, address: &str) -> StdResult<u128> {
        let balance: BalanceResponse = self.app.wrap().query_wasm_smart(
            self.beclip.clone(),
            &Cw20QueryMsg::Balance {
                address: address.to_owned(),
            },
        )?;
        Ok(balance.balance.u128())
    }

    // timelock_staking contract
    pub fn update_single_sided_stake_config(
        &mut self,
        sender: &str,
        config: SingleStakingUpdateConfigMsg,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.single_staking_contract.clone(),
            &SingleSidedStakingExecuteMsg::UpdateConfig { config },
            &[],
        )
    }
    pub fn query_single_sided_stake_config(&self) -> StdResult<SingleStakingConfig> {
        let config: SingleStakingConfig = self.app.wrap().query_wasm_smart(
            self.single_staking_contract.clone(),
            &SingleStakingQueryMsg::Config {},
        )?;
        Ok(config)
    }
    pub fn query_single_sided_staking(&self, user: &str) -> StdResult<Vec<SingleSidedUserStaking>> {
        let user_staking: Vec<SingleSidedUserStaking> = self.app.wrap().query_wasm_smart(
            self.single_staking_contract.clone(),
            &SingleStakingQueryMsg::Staking {
                user: user.to_string(),
            },
        )?;
        Ok(user_staking)
    }
    pub fn query_single_sided_total_staking(&self) -> StdResult<u128> {
        let total_staking: Uint128 = self.app.wrap().query_wasm_smart(
            self.single_staking_contract.clone(),
            &SingleStakingQueryMsg::TotalStaking {},
        )?;
        Ok(total_staking.u128())
    }
    pub fn single_sided_stake(
        &mut self,
        sender: &str,
        amount: u128,
        duration: u64,
        recipient: Option<String>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.eclipastro.clone(),
            &Cw20ExecuteMsg::Send {
                contract: self.single_staking_contract(),
                amount: Uint128::from(amount),
                msg: to_json_binary(&SingleStakingCw20HookMsg::Stake {
                    lock_duration: duration,
                    recipient,
                })?,
            },
            &[],
        )
    }
    pub fn single_sided_unstake(
        &mut self,
        sender: &str,
        duration: u64,
        locked_at: u64,
        amount: Option<Uint128>,
        recipient: Option<String>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.single_staking_contract.clone(),
            &SingleSidedStakingExecuteMsg::Unstake {
                duration,
                locked_at: Some(locked_at),
                amount,
                recipient,
            },
            &[],
        )
    }
    pub fn single_sided_restake(
        &mut self,
        sender: &str,
        from_duration: u64,
        locked_at: u64,
        to_duration: u64,
        recipient: Option<String>,
        amount: Option<Uint128>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.single_staking_contract.clone(),
            &SingleSidedStakingExecuteMsg::Restake {
                from_duration,
                locked_at: Some(locked_at),
                amount,
                to_duration,
                recipient,
            },
            &[],
        )
    }
    pub fn query_single_sided_staking_reward(
        &self,
        user: &str,
    ) -> StdResult<Vec<SingleStakingUserRewardByDuration>> {
        let reward: Vec<SingleStakingUserRewardByDuration> = self.app.wrap().query_wasm_smart(
            self.single_staking_contract.clone(),
            &SingleStakingQueryMsg::Reward {
                user: user.to_string(),
            },
        )?;
        Ok(reward)
    }
    pub fn query_single_sided_staking_eclipastro_rewards(&self) -> StdResult<Vec<(u64, Uint128)>> {
        let reward: Vec<(u64, Uint128)> = self.app.wrap().query_wasm_smart(
            self.single_staking_contract.clone(),
            &SingleStakingQueryMsg::EclipastroRewards {},
        )?;
        Ok(reward)
    }
    pub fn single_stake_claim(
        &mut self,
        sender: &str,
        duration: u64,
        locked_at: u64,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.single_staking_contract.clone(),
            &SingleSidedStakingExecuteMsg::Claim {
                duration,
                locked_at: Some(locked_at),
            },
            &[],
        )
    }
    pub fn single_stake_claim_all(&mut self, sender: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.single_staking_contract.clone(),
            &SingleSidedStakingExecuteMsg::ClaimAll {
                with_flexible: true,
            },
            &[],
        )
    }
    pub fn calculate_penalty(
        &self,
        amount: u128,
        duration: u64,
        locked_at: u64,
    ) -> StdResult<u128> {
        let penalty: Uint128 = self.app.wrap().query_wasm_smart(
            self.single_staking_contract.clone(),
            &SingleStakingQueryMsg::CalculatePenalty {
                amount: Uint128::from(amount),
                duration,
                locked_at,
            },
        )?;
        Ok(penalty.u128())
    }
    // lockdrop
    pub fn update_lockdrop_config(
        &mut self,
        sender: &str,
        new_config: LockdropUpdateConfigMsg,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::UpdateConfig { new_config },
            &[],
        )
    }
    pub fn query_lockdrop_config(&self) -> StdResult<LockdropConfig> {
        let res: LockdropConfig = self
            .app
            .wrap()
            .query_wasm_smart(self.lockdrop_contract.clone(), &LockdropQueryMsg::Config {})?;
        Ok(res)
    }
    pub fn single_staking_increase_lockdrop(
        &mut self,
        sender: &str,
        token: String,
        amount: u128,
        duration: u64,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::IncreaseLockup {
                stake_type: StakeType::SingleStaking,
                duration,
            },
            &[coin(amount, token)],
        )
    }
    pub fn query_single_lockup_info(&self) -> StdResult<SingleLockupInfoResponse> {
        let res: SingleLockupInfoResponse = self.app.wrap().query_wasm_smart(
            self.lockdrop_contract.clone(),
            &LockdropQueryMsg::SingleLockupInfo {},
        )?;
        Ok(res)
    }
    pub fn query_user_single_lockup_info(
        &self,
        user: &str,
    ) -> StdResult<Vec<UserSingleLockupInfoResponse>> {
        let res: Vec<UserSingleLockupInfoResponse> = self.app.wrap().query_wasm_smart(
            self.lockdrop_contract.clone(),
            &LockdropQueryMsg::UserSingleLockupInfo {
                user: user.to_string(),
            },
        )?;
        Ok(res)
    }
    pub fn lp_staking_increase_lockdrop(
        &mut self,
        sender: &str,
        token: String,
        amount: u128,
        duration: u64,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::IncreaseLockup {
                stake_type: StakeType::LpStaking,
                duration,
            },
            &[coin(amount, token)],
        )
    }
    pub fn query_lp_lockup_info(&self) -> StdResult<LpLockupInfoResponse> {
        let res: LpLockupInfoResponse = self.app.wrap().query_wasm_smart(
            self.lockdrop_contract.clone(),
            &LockdropQueryMsg::LpLockupInfo {},
        )?;
        Ok(res)
    }
    pub fn query_user_lp_lockup_info(
        &self,
        user: &str,
    ) -> StdResult<Vec<UserLpLockupInfoResponse>> {
        let res: Vec<UserLpLockupInfoResponse> = self.app.wrap().query_wasm_smart(
            self.lockdrop_contract.clone(),
            &LockdropQueryMsg::UserLpLockupInfo {
                user: user.to_string(),
            },
        )?;
        Ok(res)
    }
    pub fn single_staking_extend_duration_without_deposit(
        &mut self,
        sender: &str,
        from_duration: u64,
        to_duration: u64,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::ExtendLock {
                stake_type: StakeType::SingleStaking,
                from: from_duration,
                to: to_duration,
            },
            &[],
        )
    }

    pub fn single_staking_extend_duration_with_deposit(
        &mut self,
        sender: &str,
        token: String,
        amount: u128,
        from_duration: u64,
        to_duration: u64,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::ExtendLock {
                stake_type: StakeType::SingleStaking,
                from: from_duration,
                to: to_duration,
            },
            &[coin(amount, token)],
        )
    }
    pub fn lp_lockup_extend_duration_without_deposit(
        &mut self,
        sender: &str,
        from_duration: u64,
        to_duration: u64,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::ExtendLock {
                stake_type: StakeType::LpStaking,
                from: from_duration,
                to: to_duration,
            },
            &[],
        )
    }

    pub fn lp_lockup_extend_duration_with_deposit(
        &mut self,
        sender: &str,
        token: String,
        amount: u128,
        from_duration: u64,
        to_duration: u64,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::ExtendLock {
                stake_type: StakeType::LpStaking,
                from: from_duration,
                to: to_duration,
            },
            &[coin(amount, token)],
        )
    }
    pub fn single_staking_lockdrop_withdraw(
        &mut self,
        sender: &str,
        amount: Option<Uint128>,
        duration: u64,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::Unlock {
                stake_type: StakeType::SingleStaking,
                duration,
                amount,
            },
            &[],
        )
    }
    pub fn lp_staking_lockdrop_withdraw(
        &mut self,
        sender: &str,
        amount: Option<Uint128>,
        duration: u64,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::Unlock {
                stake_type: StakeType::LpStaking,
                amount,
                duration,
            },
            &[],
        )
    }
    pub fn fund_beclip(&mut self, sender: &str, amount: u128) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.beclip.clone(),
            &Cw20ExecuteMsg::Send {
                contract: self.lockdrop_contract().to_string(),
                amount: Uint128::from(amount),
                msg: to_json_binary(&LockdropCw20HookMsg::IncreasebEclipIncentives {})?,
            },
            &[],
        )
    }
    pub fn lockdrop_stake_to_vaults(&mut self, sender: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::StakeToVaults {},
            &[],
        )
    }
    pub fn query_single_lockup_state(&self) -> StdResult<SingleLockupStateResponse> {
        let res: SingleLockupStateResponse = self.app.wrap().query_wasm_smart(
            self.lockdrop_contract.clone(),
            &LockdropQueryMsg::SingleLockupState {},
        )?;
        Ok(res)
    }

    pub fn query_lp_lockup_state(&self) -> StdResult<LpLockupStateResponse> {
        let res: LpLockupStateResponse = self.app.wrap().query_wasm_smart(
            self.lockdrop_contract.clone(),
            &LockdropQueryMsg::LpLockupState {},
        )?;
        Ok(res)
    }
    pub fn query_total_beclip_incentives(&self) -> StdResult<BalanceResponse> {
        let res: BalanceResponse = self.app.wrap().query_wasm_smart(
            self.lockdrop_contract.clone(),
            &LockdropQueryMsg::TotalbEclipIncentives {},
        )?;
        Ok(res)
    }
    pub fn single_lockdrop_claim_rewards(
        &mut self,
        sender: &str,
        duration: u64,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::ClaimRewards {
                stake_type: StakeType::SingleStaking,
                duration,
                assets: None,
            },
            &[],
        )
    }
    pub fn single_lockdrop_claim_all_rewards(&mut self, sender: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::ClaimAllRewards {
                stake_type: StakeType::SingleStaking,
                with_flexible: true,
                assets: None,
            },
            &[],
        )
    }
    pub fn lp_lockdrop_claim_rewards(
        &mut self,
        sender: &str,
        duration: u64,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::ClaimRewards {
                stake_type: StakeType::LpStaking,
                duration,
                assets: None,
            },
            &[],
        )
    }
    pub fn lp_lockdrop_claim_all_rewards(&mut self, sender: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::ClaimAllRewards {
                stake_type: StakeType::LpStaking,
                with_flexible: true,
                assets: None,
            },
            &[],
        )
    }

    pub fn single_lockup_relock(
        &mut self,
        sender: &str,
        from: u64,
        to: u64,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::ExtendLock {
                stake_type: StakeType::SingleStaking,
                from,
                to,
            },
            &[],
        )
    }

    pub fn single_lockup_relock_with_deposit(
        &mut self,
        sender: &str,
        from: u64,
        to: u64,
        asset: String,
        amount: u128,
    ) -> AnyResult<AppResponse> {
        if asset == self.eclipastro.to_string() {
            self.app.execute_contract(
                Addr::unchecked(sender),
                self.eclipastro.clone(),
                &Cw20ExecuteMsg::Send {
                    contract: self.lockdrop_contract.to_string(),
                    amount: Uint128::from(amount),
                    msg: to_json_binary(&LockdropCw20HookMsg::ExtendLockup {
                        stake_type: StakeType::SingleStaking,
                        from,
                        to,
                    })?,
                },
                &[],
            )
        } else {
            self.app.execute_contract(
                Addr::unchecked(sender),
                self.lockdrop_contract.clone(),
                &LockdropCw20HookMsg::ExtendLockup {
                    stake_type: StakeType::SingleStaking,
                    from,
                    to,
                },
                &[coin(amount, asset)],
            )
        }
    }

    pub fn single_lockup_unlock(
        &mut self,
        sender: &str,
        duration: u64,
        amount: Option<Uint128>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::Unlock {
                stake_type: StakeType::SingleStaking,
                duration,
                amount,
            },
            &[],
        )
    }
    pub fn lp_lockup_unlock(
        &mut self,
        sender: &str,
        duration: u64,
        amount: Option<Uint128>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::Unlock {
                stake_type: StakeType::LpStaking,
                duration,
                amount,
            },
            &[],
        )
    }
}
