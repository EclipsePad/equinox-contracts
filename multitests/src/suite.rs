use anyhow::Result as AnyResult;
use astroport::{
    asset::{Asset, AssetInfo, PairInfo},
    factory::{PairConfig, PairType, QueryMsg as FactoryQueryMsg},
    incentives::{ExecuteMsg as IncentivesExecuteMsg, QueryMsg as IncentivesQueryMsg},
    pair::ExecuteMsg as PairExecuteMsg,
    staking,
    token::{
        Cw20Coin, InstantiateMsg as AstroInstantiateMsg, MinterResponse as AstroportMinterResponse,
    },
    vesting::{self, Cw20HookMsg as VestingCw20HookMsg, VestingAccount},
};
// use astroport_governance::voting_escrow::{
//     Cw20HookMsg as AstroportVotingEscrowCw20HookMsg, ExecuteMsg as AstroportVotingEscrowExecuteMsg,
//     QueryMsg as AstroportVotingEscrowQueryMsg,
// };
use astroport_voting_escrow;
use cosmwasm_std::{coin, coins, to_json_binary, Addr, Binary, Coin, Decimal, StdResult, Uint128};
use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};
use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};
use eclipse_base::assets::{Currency, TokenUnverified};
use equinox_msg::{
    flexible_staking::{
        Config as FlexibleStakingConfig, Cw20HookMsg as FlexibleStakingCw20HookMsg,
        ExecuteMsg as FlexibleStakingExecuteMsg, InstantiateMsg as FlexibleStakingInstantiateMsg,
        QueryMsg as FlexibleStakingQueryMsg, UpdateConfigMsg as FlexibleStakingUpdateConfig,
    },
    lockdrop::{
        Config as LockdropConfig, Cw20HookMsg as LockdropCw20HookMsg,
        ExecuteMsg as LockdropExecuteMsg, InstantiateMsg as LockdropInstantiateMsg, LockConfig,
        LockupInfoResponse, LpLockupStateResponse, QueryMsg as LockdropQueryMsg,
        SingleLockupStateResponse, StakeType, UpdateConfigMsg as LockdropUpdateConfigMsg,
        UserLpLockupInfoResponse, UserSingleLockupInfoResponse,
    },
    lp_staking::{
        Config as LpStakingConfig, Cw20HookMsg as LpStakingCw20HookMsg,
        ExecuteMsg as LpStakingExecuteMsg, InstantiateMsg as LpStakingInstantiateMsg,
        QueryMsg as LpStakingQueryMsg, RewardConfig, TotalStaking,
        UpdateConfigMsg as LpStakingUpdateConfigMsg,
        UserRewardResponse as LpStakingUserRewardResponse, UserStaking,
    },
    reward_distributor::{
        FlexibleReward, InstantiateMsg as RewardDistributorInstantiateMsg, LockingRewardConfig,
        QueryMsg as RewardDistributorQueryMsg, TimelockReward, TotalStakingData,
    },
    timelock_staking::{
        Config as TimelockStakingConfig, Cw20HookMsg as TimelockCw20HookMsg,
        ExecuteMsg as TimelockStakingExecuteMsg, InstantiateMsg as TimelockStakingInstantiateMsg,
        QueryMsg as TimelockStakingQueryMsg, TimeLockConfig,
        UpdateConfigMsg as TimelockStakingUpdateConfig, UserStaking as TimelockUserStaking,
    },
    token_converter::{
        Config as ConverterConfig, Cw20HookMsg as ConverterCw20HookMsg,
        ExecuteMsg as ConverterExecuteMsg, InstantiateMsg as ConverterInstantiateMsg,
        QueryMsg as ConverterQueryMsg, RewardConfig as ConverterRewardConfig,
        RewardResponse as ConverterRewardResponse, UpdateConfig as ConverterUpdateConfig,
    },
    voter::{
        Config as VoterConfig, ExecuteMsg as VoterExecuteMsg,
        InstantiateMsg as VoterInstantiateMsg, QueryMsg as VoterQueryMsg,
        UpdateConfig as VoterUpdateConfig,
    },
};

pub const ASTRO: &str = "astro";
// pub const XASTRO: &str = "xastro";
pub const ECLIP_ASTRO: &str = "eclipastro";

// for tf tracker
const MODULE_ADDRESS: &str = "tokenfactory_module";

fn store_minter(app: &mut App) -> u64 {
    app.store_code(Box::new(ContractWrapper::new(
        minter_mocks::contract::execute,
        minter_mocks::contract::instantiate,
        minter_mocks::contract::query,
    )))
}

fn store_tokenfactory_tracker(app: &mut App) -> u64 {
    app.store_code(Box::new(
        ContractWrapper::new(
            astroport_tokenfactory_tracker::contract::instantiate, // fake
            astroport_tokenfactory_tracker::contract::instantiate,
            astroport_tokenfactory_tracker::query::query,
        )
        .with_sudo(astroport_tokenfactory_tracker::contract::sudo),
    ))
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

fn store_cw20_token(app: &mut App) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    ));

    app.store_code(contract)
}

fn store_astroport_pair(app: &mut App) -> u64 {
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

fn store_astroport_factory(app: &mut App) -> u64 {
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

fn store_astroport_generator(app: &mut App) -> u64 {
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

fn store_astroport_vesting(app: &mut App) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        astroport_vesting::contract::execute,
        astroport_vesting::contract::instantiate,
        astroport_vesting::contract::query,
    ));

    app.store_code(contract)
}

fn store_astroport_voting_escrow(app: &mut App) -> u64 {
    app.store_code(Box::new(ContractWrapper::new_with_empty(
        astroport_voting_escrow::contract::execute,
        astroport_voting_escrow::contract::instantiate,
        astroport_voting_escrow::contract::query,
    )))
}

// fn store_astroport_emissions_controller(app: &mut App) -> u64 {
//     app.store_code(Box::new(ContractWrapper::new_with_empty(
//         astroport_emissions_controller::execute::execute,
//         astroport_emissions_controller::instantiate::instantiate,
//         astroport_emissions_controller::query::query,
//     )))
// }

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

fn store_eclipsepad_staking(app: &mut App) -> u64 {
    app.store_code(Box::new(ContractWrapper::new_with_empty(
        eclipsepad_staking::contract::execute,
        eclipsepad_staking::contract::instantiate,
        eclipsepad_staking::contract::query,
    )))
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

fn store_lp_staking(app: &mut App) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        lp_staking::contract::execute,
        lp_staking::contract::instantiate,
        lp_staking::contract::query,
    ));

    app.store_code(contract)
}

fn store_lockdrop(app: &mut App) -> u64 {
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
    pub initial_balances: Vec<Cw20Coin>,
    pub timelock_config: Vec<TimeLockConfig>,
    pub eclip_daily_reward: Option<Uint128>,
    pub lp_staking_eclip_daily_reward: Option<Uint128>,
    pub locking_reward_config: Option<Vec<LockingRewardConfig>>,
    pub lockdrop_init_timestamp: u64,
    pub lockdrop_deposit_window: Option<u64>,
    pub lockdrop_withdraw_window: Option<u64>,
    pub lock_configs: Option<Vec<LockConfig>>,
}

impl SuiteBuilder {
    pub fn new() -> Self {
        Self {
            admin: None,
            initial_balances: vec![],
            timelock_config: vec![],
            eclip_daily_reward: None,
            lp_staking_eclip_daily_reward: None,
            locking_reward_config: None,
            lockdrop_init_timestamp: 0u64,
            lockdrop_deposit_window: None,
            lockdrop_withdraw_window: None,
            lock_configs: None,
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

    pub fn with_timelock_config(mut self, config: Vec<(u64, u64)>) -> Self {
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

    pub fn with_eclip_daily_reward(mut self, daily_reward: u128) -> Self {
        self.eclip_daily_reward = Some(Uint128::from(daily_reward));
        self
    }

    pub fn with_lp_staking_eclip_daily_reward(mut self, daily_reward: u128) -> Self {
        self.lp_staking_eclip_daily_reward = Some(Uint128::from(daily_reward));
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
        self.locking_reward_config = Some(locking_reward_config);
        self
    }

    pub fn with_lockdrop_init_timestamp(mut self, time: u64) -> Self {
        self.lockdrop_init_timestamp = time;
        self
    }

    pub fn with_lockdrop_deposit_window(mut self, time: u64) -> Self {
        self.lockdrop_deposit_window = Some(time);
        self
    }

    pub fn with_lockdrop_withdraw_window(mut self, time: u64) -> Self {
        self.lockdrop_withdraw_window = Some(time);
        self
    }

    pub fn with_lock_configs(mut self, config: Vec<(u64, u64, u64)>) -> Self {
        let lock_configs = config
            .into_iter()
            .map(
                |(duration, multiplier, early_unlock_penalty_bps)| LockConfig {
                    duration,
                    multiplier,
                    early_unlock_penalty_bps,
                },
            )
            .collect::<Vec<LockConfig>>();
        self.lock_configs = Some(lock_configs);
        self
    }

    #[track_caller]
    pub fn build(self) -> Suite {
        let mut app: App = App::default();

        // set time
        app.update_block(|x| {
            x.time = x
                .time
                .plus_seconds(astroport_governance::utils::EPOCH_START);
        });

        let admin = Addr::unchecked("admin");
        let eclipse_treasury = Addr::unchecked("eclipse_treasury");
        let eclipse_stability_pool = Addr::unchecked("eclipse_stability_pool");
        let ce_reward_distributor = Addr::unchecked("ce_reward_distributor");
        let vxastro_contract = Addr::unchecked("vxastro");

        let minter_id = store_minter(&mut app);
        let minter_contract = app
            .instantiate_contract(
                minter_id,
                admin.clone(),
                &eclipse_base::minter::msg::InstantiateMsg { cw20_code_id: None },
                &[],
                "minter",
                Some(admin.clone().to_string()),
            )
            .unwrap();

        let tokenfactory_tracker_id = store_tokenfactory_tracker(&mut app);
        // let tokenfactory_tracker = app
        //     .instantiate_contract(
        //         tokenfactory_tracker_id,
        //         admin.clone(),
        //         &astroport::tokenfactory_tracker::InstantiateMsg {
        //             tokenfactory_module_address: MODULE_ADDRESS.to_string(),
        //             track_over_seconds: true,
        //             tracked_denom: ASTRO.to_string(),
        //         },
        //         &[],
        //         "tokenfactory_tracker",
        //         Some(admin.clone().to_string()),
        //     )
        //     .unwrap();

        let astro_staking_id = store_astro_staking(&mut app);
        let astro_staking_contract = app
            .instantiate_contract(
                astro_staking_id,
                admin.clone(),
                &staking::InstantiateMsg {
                    deposit_token_denom: ASTRO.to_string(),
                    token_factory_addr: MODULE_ADDRESS.to_string(),
                    tracking_code_id: tokenfactory_tracker_id,
                    tracking_admin: admin.to_string(),
                },
                &[],
                "ASTRO staking",
                Some(admin.clone().to_string()),
            )
            .unwrap();

        let cw20_token_code_id = store_cw20_token(&mut app);
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
            tracker_config: None,
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
                        denom: ASTRO.to_string(),
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
                &astroport::incentives::InstantiateMsg {
                    owner: admin.to_string(),
                    factory: astroport_factory.to_string(),
                    astro_token: AssetInfo::NativeToken {
                        denom: ASTRO.to_string(),
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

        // let astro_staking_config: AstroStakingConfigResponse = app
        //     .wrap()
        //     .query_wasm_smart(
        //         astro_staking_contract.clone(),
        //         &AstroStakingQueryMsg::Config {},
        //     )
        //     .unwrap_or(AstroStakingConfigResponse {
        //         deposit_token_addr: astro_staking_contract.clone(),
        //         share_token_addr: Addr::unchecked(""),
        //     });
        // let xastro_contract = astro_staking_config.share_token_addr;

        let astroport_voting_escrow_id = store_astroport_voting_escrow(&mut app);
        let astroport_voting_escrow_address = app
            .instantiate_contract(
                astroport_voting_escrow_id,
                admin.clone(),
                &astroport_governance::voting_escrow::InstantiateMsg {
                    owner: admin.to_string(),
                    guardian_addr: Some("guardian".to_string()),
                    deposit_token_addr: String::default(),
                    marketing: None,
                    logo_urls_whitelist: vec![],
                },
                &[],
                "Astroport voting escrow",
                None,
            )
            .unwrap();

        // let astroport_generator_controller_id = store_astroport_generator_controller(&mut app);
        // let astroport_generator_controller_address = app
        //     .instantiate_contract(
        //         astroport_generator_controller_id,
        //         admin.clone(),
        //         &astroport_governance::generator_controller::InstantiateMsg {
        //             owner: admin.to_string(),
        //             escrow_addr: astroport_voting_escrow_address.to_string(),
        //             generator_addr: astroport_generator.to_string(),
        //             factory_addr: astroport_factory.to_string(),
        //             pools_limit: 10,
        //             whitelisted_pools: vec![],
        //         },
        //         &[],
        //         "Astroport generator controller",
        //         None,
        //     )
        //     .unwrap();

        // let eclipastro_id = store_eclipastro(&mut app);
        let converter_id = store_converter(&mut app);
        // let converter_contract = app
        //     .instantiate_contract(
        //         converter_id,
        //         admin.clone(),
        //         &ConverterInstantiateMsg {
        //             owner: admin.clone().into_string(),
        //             token_in: astro_contract.clone().into_string(),
        //             xtoken: xastro_contract.clone().into_string(),
        //             treasury: eclipse_treasury.clone().into_string(),
        //             token_code_id: eclipastro_id,
        //             marketing: None,
        //         },
        //         &[],
        //         "converter",
        //         Some(admin.clone().to_string()),
        //     )
        //     .unwrap();
        // let converter_config: ConverterConfig = app
        //     .wrap()
        //     .query_wasm_smart(converter_contract.clone(), &ConverterQueryMsg::Config {})
        //     .unwrap_or(ConverterConfig {
        //         token_in: Addr::unchecked(""),
        //         token_out: Addr::unchecked(""),
        //         xtoken: Addr::unchecked(""),
        //         treasury: Addr::unchecked(""),
        //         vxtoken_holder: Addr::unchecked(""),
        //         stability_pool: Addr::unchecked(""),
        //         staking_reward_distributor: Addr::unchecked(""),
        //         ce_reward_distributor: Addr::unchecked(""),
        //     });
        // let eclipastro_contract = converter_config.token_out;

        let flexible_staking_id = store_flexible_staking(&mut app);
        let flexible_staking_contract = app
            .instantiate_contract(
                flexible_staking_id,
                admin.clone(),
                &FlexibleStakingInstantiateMsg {
                    owner: admin.clone().into_string(),
                    token: ECLIP_ASTRO.to_string(),
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
                    token: ECLIP_ASTRO.to_string(),
                    timelock_config: Some(self.timelock_config.clone()),
                    dao_treasury_address: Addr::unchecked("dao_treasury_address").to_string(),
                },
                &[],
                "timelock staking",
                Some(admin.clone().to_string()),
            )
            .unwrap();

        let reward_distributor_id = store_reward_distributor(&mut app);
        let eclip = "factory/creator/eclip".to_string();
        // let reward_distributor_contract = app
        //     .instantiate_contract(
        //         reward_distributor_id,
        //         admin.clone(),
        //         &RewardDistributorInstantiateMsg {
        //             owner: admin.clone().into_string(),
        //             eclipastro: ECLIP_ASTRO.to_string(),
        //             eclip: eclip.clone(),
        //             flexible_staking: flexible_staking_contract.clone().into_string(),
        //             timelock_staking: timelock_staking_contract.clone().into_string(),
        //             token_converter: converter_contract.clone().into_string(),
        //             eclip_daily_reward: self.eclip_daily_reward.clone(),
        //             locking_reward_config: self.locking_reward_config.clone(),
        //         },
        //         &[],
        //         "reward distributor",
        //         Some(admin.clone().to_string()),
        //     )
        //     .unwrap();

        let eclipsepad_staking_id = store_eclipsepad_staking(&mut app);
        let eclipsepad_staking_contract = app
            .instantiate_contract(
                eclipsepad_staking_id,
                admin.clone(),
                &eclipse_base::staking::msg::InstantiateMsg {
                    equinox_voter: None,
                    beclip_minter: None,
                    staking_token: None,
                    beclip_address: None,
                    beclip_whitelist: None,
                    lock_schedule: None,
                    seconds_per_essence: None,
                    dao_treasury_address: None,
                    penalty_multiplier: None,
                    pagintaion_config: None,
                    eclip_per_second: None,
                    eclip_per_second_multiplier: None,
                },
                &[],
                "eclipsepad staking",
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
                    astro: ASTRO.to_string(),
                    xastro: String::default(),
                    vxastro: vxastro_contract.to_string(),
                    staking_contract: astro_staking_contract.clone().into_string(),
                    converter_contract: String::default(),
                    astroport_voting_escrow_contract: astroport_voting_escrow_address.to_string(),
                    astroport_generator_controller: String::default(),
                    eclipsepad_staking_contract: eclipsepad_staking_contract.to_string(),
                },
                &[],
                "voter",
                Some(admin.clone().to_string()),
            )
            .unwrap();

        // let asset_infos = vec![
        //     AssetInfo::Token {
        //         contract_addr: eclipastro_contract.clone(),
        //     },
        //     AssetInfo::Token {
        //         contract_addr: xastro_contract.clone(),
        //     },
        // ];

        // let msg = astroport::factory::ExecuteMsg::CreatePair {
        //     pair_type: PairType::Xyk {},
        //     asset_infos,
        //     init_params: None,
        // };

        // app.execute_contract(admin.clone(), astroport_factory.clone(), &msg, &[])
        //     .unwrap();

        // let info: PairInfo = app
        //     .wrap()
        //     .query_wasm_smart(
        //         astroport_factory.clone(),
        //         &FactoryQueryMsg::Pair {
        //             asset_infos: vec![
        //                 AssetInfo::Token {
        //                     contract_addr: eclipastro_contract.clone(),
        //                 },
        //                 AssetInfo::Token {
        //                     contract_addr: xastro_contract.clone(),
        //                 },
        //             ],
        //         },
        //     )
        //     .unwrap();
        // let eclipastro_xastro_lp_contract = info.contract_addr;
        // let eclipastro_xastro_lp_token_contract = info.liquidity_token;

        let lp_staking_code_id = store_lp_staking(&mut app);
        // let lp_staking = app
        //     .instantiate_contract(
        //         lp_staking_code_id,
        //         admin.clone(),
        //         &LpStakingInstantiateMsg {
        //             owner: admin.clone().to_string(),
        //             lp_token: eclipastro_xastro_lp_token_contract.to_string(),
        //             lp_contract: eclipastro_xastro_lp_contract.to_string(),
        //             eclip: eclip.clone(),
        //             astro: astro_contract.to_string(),
        //             xastro: xastro_contract.to_string(),
        //             astro_staking: astro_staking_contract.to_string(),
        //             converter: converter_contract.to_string(),
        //             eclip_daily_reward: self.lp_staking_eclip_daily_reward,
        //             astroport_generator: astroport_generator.to_string(),
        //             treasury: eclipse_treasury.to_string(),
        //             stability_pool: eclipse_stability_pool.to_string(),
        //             ce_reward_distributor: Some(ce_reward_distributor.to_string()),
        //         },
        //         &[],
        //         "Eclipsefi lp staking",
        //         None,
        //     )
        //     .unwrap();

        let lockdrop_code_id = store_lockdrop(&mut app);
        let init_timestamp = match self.lockdrop_init_timestamp {
            0u64 => app.block_info().time.seconds() + 86400,
            _ => self.lockdrop_init_timestamp,
        };
        // let lockdrop = app
        //     .instantiate_contract(
        //         lockdrop_code_id,
        //         admin.clone(),
        //         &LockdropInstantiateMsg {
        //             init_timestamp: init_timestamp,
        //             deposit_window: self.lockdrop_deposit_window,
        //             withdrawal_window: self.lockdrop_withdraw_window,
        //             lock_configs: self.lock_configs,
        //             astro_token: astro_contract.to_string(),
        //             xastro_token: xastro_contract.to_string(),
        //             eclipastro_token: eclipastro_contract.to_string(),
        //             astro_staking: astro_staking_contract.to_string(),
        //             converter: converter_contract.to_string(),
        //             liquidity_pool: eclipastro_xastro_lp_contract.to_string(),
        //             owner: None,
        //             eclip: eclip.clone(),
        //             dao_treasury_address: Addr::unchecked("dao_treasury_address").to_string(),
        //         },
        //         &[],
        //         "Eclipsefi lockdrop",
        //         None,
        //     )
        //     .unwrap();

        Suite {
            app,
            admin,
            astro_denom: ASTRO.to_string(),
            astro_staking_contract,
            astroport_voting_escrow: astroport_voting_escrow_address,
            astroport_generator_controller: Addr::unchecked("default"),
            xastro_denom: String::default(),
            vxastro_contract,
            astroport_factory,
            astroport_vesting,
            astroport_generator,
            eclipastro_denom: ECLIP_ASTRO.to_string(),
            converter_contract: Addr::unchecked("default"),
            flexible_staking_contract,
            timelock_staking_contract,
            reward_distributor_contract: Addr::unchecked("default"),
            eclipsepad_staking: eclipsepad_staking_contract,
            voter_contract,
            eclipse_stability_pool,
            ce_reward_distributor,
            eclipse_treasury,
            eclip,
            eclipastro_xastro_lp_contract: Addr::unchecked("default"),
            eclipastro_xastro_lp_token_contract: Addr::unchecked("default"),
            lp_staking: Addr::unchecked("default"),
            lockdrop: Addr::unchecked("default"),

            minter_contract,
        }
    }
}

pub struct Suite {
    app: App,
    admin: Addr,
    astro_denom: String,
    astro_staking_contract: Addr,
    astroport_voting_escrow: Addr,
    astroport_generator_controller: Addr,
    xastro_denom: String,
    vxastro_contract: Addr,
    astroport_factory: Addr,
    astroport_vesting: Addr,
    astroport_generator: Addr,
    eclipastro_denom: String,
    converter_contract: Addr,
    flexible_staking_contract: Addr,
    timelock_staking_contract: Addr,
    reward_distributor_contract: Addr,
    eclipsepad_staking: Addr,
    voter_contract: Addr,
    eclipse_stability_pool: Addr,
    ce_reward_distributor: Addr,
    eclipse_treasury: Addr,
    eclip: String,
    eclipastro_xastro_lp_contract: Addr,
    eclipastro_xastro_lp_token_contract: Addr,
    lp_staking: Addr,
    lockdrop: Addr,

    minter_contract: Addr,
}

impl Suite {
    pub fn admin(&self) -> String {
        self.admin.to_string()
    }
    pub fn astro_denom(&self) -> String {
        self.astro_denom.to_string()
    }
    pub fn astro_staking_contract(&self) -> String {
        self.astro_staking_contract.to_string()
    }
    pub fn astroport_voting_escrow_contract(&self) -> String {
        self.astroport_voting_escrow.to_string()
    }
    pub fn astroport_generator_controller_contract(&self) -> String {
        self.astroport_generator_controller.to_string()
    }
    pub fn xastro_denom(&self) -> String {
        self.xastro_denom.to_string()
    }
    pub fn vxastro_contract(&self) -> String {
        self.vxastro_contract.to_string()
    }
    pub fn astroport_factory_contract(&self) -> String {
        self.astroport_factory.to_string()
    }
    pub fn astroport_vesting_contract(&self) -> String {
        self.astroport_vesting.to_string()
    }
    pub fn astroport_generator_contract(&self) -> String {
        self.astroport_generator.to_string()
    }
    pub fn eclipastro_denom(&self) -> String {
        self.eclipastro_denom.to_string()
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
    pub fn eclipsepad_staking_contract(&self) -> String {
        self.eclipsepad_staking.to_string()
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
    pub fn eclip(&self) -> String {
        self.eclip.clone()
    }
    pub fn eclipastro_xastro_lp_contract(&self) -> String {
        self.eclipastro_xastro_lp_contract.to_string()
    }
    pub fn eclipastro_xastro_lp_token_contract(&self) -> String {
        self.eclipastro_xastro_lp_token_contract.to_string()
    }
    pub fn lp_staking(&self) -> String {
        self.lp_staking.to_string()
    }
    pub fn lockdrop(&self) -> String {
        self.lockdrop.to_string()
    }
    pub fn minter_contract(&self) -> Addr {
        self.minter_contract.clone()
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
                        timelock_contract: Some(
                            self.timelock_staking_contract.clone().into_string(),
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

        self.app
            .execute_contract(
                self.admin.clone(),
                self.timelock_staking_contract.clone(),
                &TimelockStakingExecuteMsg::AllowUsers {
                    users: vec![self.lockdrop.to_string()],
                },
                &[],
            )
            .unwrap();
        self.app
            .execute_contract(
                self.admin.clone(),
                self.flexible_staking_contract.clone(),
                &FlexibleStakingExecuteMsg::AllowUsers {
                    users: vec![self.lockdrop.to_string()],
                },
                &[],
            )
            .unwrap();
    }

    // // xASTRO staking contract
    // pub fn stake_astro(&mut self, sender: &str, amount: u128) -> AnyResult<AppResponse> {
    //     self.app.execute_contract(
    //         Addr::unchecked(sender),
    //         self.astro_contract.clone(),
    //         &Cw20ExecuteMsg::Send {
    //             contract: self.astro_staking_contract(),
    //             amount: Uint128::from(amount),
    //             msg: to_json_binary(&AstroStakingCw20HookMsg::Enter {})?,
    //         },
    //         &[],
    //     )
    // }

    // pub fn send_astro(
    //     &mut self,
    //     sender: &str,
    //     recipient: &str,
    //     amount: u128,
    // ) -> AnyResult<AppResponse> {
    //     self.app.execute_contract(
    //         Addr::unchecked(sender),
    //         self.astro_contract.clone(),
    //         &Cw20ExecuteMsg::Transfer {
    //             recipient: recipient.to_string(),
    //             amount: Uint128::from(amount),
    //         },
    //         &[],
    //     )
    // }

    // // query astro token amount
    // pub fn query_astro_balance(&self, address: &str) -> StdResult<u128> {
    //     let balance: BalanceResponse = self.app.wrap().query_wasm_smart(
    //         self.astro_contract.clone(),
    //         &Cw20QueryMsg::Balance {
    //             address: address.to_owned(),
    //         },
    //     )?;
    //     Ok(balance.balance.u128())
    // }

    // // query xastro token amount
    // pub fn query_xastro_balance(&self, address: &str) -> StdResult<u128> {
    //     let balance: BalanceResponse = self.app.wrap().query_wasm_smart(
    //         self.xastro_contract.clone(),
    //         &Cw20QueryMsg::Balance {
    //             address: address.to_owned(),
    //         },
    //     )?;
    //     Ok(balance.balance.u128())
    // }

    // query astro staking total deposit
    pub fn query_astro_staking_total_deposit(&self) -> StdResult<u128> {
        self.app.wrap().query_wasm_smart(
            self.astro_staking_contract.clone(),
            &astroport::staking::QueryMsg::TotalDeposit {},
        )
    }
    // query astro staking total shares
    pub fn query_astro_staking_total_shares(&self) -> StdResult<u128> {
        self.app.wrap().query_wasm_smart(
            self.astro_staking_contract.clone(),
            &astroport::staking::QueryMsg::TotalShares {},
        )
    }

    // astroport factory contract
    pub fn astroport_factory_update_config(
        &mut self,
        sender: &Addr,
        token_code_id: Option<u64>,
        fee_address: Option<String>,
        generator_address: Option<String>,
        whitelist_code_id: Option<u64>,
        coin_registry_address: Option<String>,
    ) -> AnyResult<AppResponse> {
        let msg = astroport::factory::ExecuteMsg::UpdateConfig {
            token_code_id,
            fee_address,
            generator_address,
            whitelist_code_id,
            coin_registry_address,
        };

        self.app
            .execute_contract(sender.clone(), self.astroport_factory.clone(), &msg, &[])
    }

    pub fn create_pair(
        &mut self,
        sender: &str,
        pair_type: PairType,
        tokens: [&Addr; 2],
        init_params: Option<Binary>,
    ) -> AnyResult<AppResponse> {
        let asset_infos = vec![
            AssetInfo::Token {
                contract_addr: tokens[0].clone(),
            },
            AssetInfo::Token {
                contract_addr: tokens[1].clone(),
            },
        ];

        let msg = astroport::factory::ExecuteMsg::CreatePair {
            pair_type,
            asset_infos,
            init_params,
        };

        self.app.execute_contract(
            Addr::unchecked(sender),
            self.astroport_factory.clone(),
            &msg,
            &[],
        )
    }

    pub fn create_pair_native(
        &mut self,
        sender: &str,
        pair_type: PairType,
        tokens: &[&str; 2],
    ) -> AnyResult<AppResponse> {
        let asset_infos = vec![
            AssetInfo::NativeToken {
                denom: tokens[0].to_string(),
            },
            AssetInfo::NativeToken {
                denom: tokens[1].to_string(),
            },
        ];

        let msg = astroport::factory::ExecuteMsg::CreatePair {
            pair_type,
            asset_infos,
            init_params: None,
        };

        self.app.execute_contract(
            Addr::unchecked(sender),
            self.astroport_factory.clone(),
            &msg,
            &[],
        )
    }

    pub fn astroport_generator_controller_update_whitelist(
        &mut self,
        sender: &str,
        lp_token_list: &[String],
    ) -> AnyResult<AppResponse> {
        let msg = astroport_governance::generator_controller::ExecuteMsg::UpdateWhitelist {
            add: Some(lp_token_list.to_owned()),
            remove: None,
        };

        self.app.execute_contract(
            Addr::unchecked(sender),
            self.astroport_generator_controller.clone(),
            &msg,
            &[],
        )
    }

    pub fn provide_liquidity(
        &mut self,
        sender: &str,
        pair: Addr,
        assets: Vec<Asset>,
        receiver: Option<String>,
        slippage_tolerance: Option<Decimal>,
    ) -> AnyResult<AppResponse> {
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
            }
        }
        let msg = PairExecuteMsg::ProvideLiquidity {
            assets,
            slippage_tolerance,
            auto_stake: None,
            receiver,
            min_lp_to_receive: None,
        };

        self.app
            .execute_contract(Addr::unchecked(sender), pair.clone(), &msg, &[])
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

    // pub fn register_vesting_accounts(
    //     &mut self,
    //     sender: &str,
    //     vesting_accounts: Vec<VestingAccount>,
    //     amount: u128,
    // ) -> AnyResult<AppResponse> {
    //     let msg = Cw20ExecuteMsg::Send {
    //         contract: self.astroport_vesting.to_string(),
    //         amount: Uint128::from(amount),
    //         msg: to_json_binary(&VestingCw20HookMsg::RegisterVestingAccounts { vesting_accounts })?,
    //     };

    //     self.app.execute_contract(
    //         Addr::unchecked(sender),
    //         self.astro_contract.clone(),
    //         &msg,
    //         &[],
    //     )
    // }

    pub fn query_pair_info(&self, asset_infos: Vec<AssetInfo>) -> StdResult<PairInfo> {
        let info: PairInfo = self.app.wrap().query_wasm_smart(
            self.astroport_factory.clone(),
            &FactoryQueryMsg::Pair { asset_infos },
        )?;
        Ok(info)
    }

    pub fn query_pair_list(
        &self,
        start_after: Option<Vec<AssetInfo>>,
        limit: Option<u32>,
    ) -> StdResult<astroport::factory::PairsResponse> {
        self.app.wrap().query_wasm_smart(
            self.astroport_factory.clone(),
            &FactoryQueryMsg::Pairs { start_after, limit },
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

    // // convert ASTRO to eclipASTRO in Convert contract
    // pub fn convert_astro(&mut self, sender: &str, amount: u128) -> AnyResult<AppResponse> {
    //     self.app.execute_contract(
    //         Addr::unchecked(sender),
    //         self.astro_contract.clone(),
    //         &Cw20ExecuteMsg::Send {
    //             contract: self.converter_contract(),
    //             amount: Uint128::from(amount),
    //             msg: to_json_binary(&ConverterCw20HookMsg::Convert {})?,
    //         },
    //         &[],
    //     )
    // }
    // pub fn convert_xastro(&mut self, sender: &str, amount: u128) -> AnyResult<AppResponse> {
    //     self.app.execute_contract(
    //         Addr::unchecked(sender),
    //         self.xastro_contract.clone(),
    //         &Cw20ExecuteMsg::Send {
    //             contract: self.converter_contract(),
    //             amount: Uint128::from(amount),
    //             msg: to_json_binary(&ConverterCw20HookMsg::Convert {})?,
    //         },
    //         &[],
    //     )
    // }

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

    // // query eclipastro token amount
    // pub fn query_eclipastro_balance(&self, address: &str) -> StdResult<u128> {
    //     let balance: BalanceResponse = self.app.wrap().query_wasm_smart(
    //         self.eclipastro_contract.clone(),
    //         &Cw20QueryMsg::Balance {
    //             address: address.to_owned(),
    //         },
    //     )?;
    //     Ok(balance.balance.u128())
    // }

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
            &ConverterQueryMsg::WithdrawableBalance {},
        )?;
        Ok(reward.u128())
    }

    // eclipsepad staking contract
    pub fn eclipsepad_staking_try_stake(
        &mut self,
        sender: &str,
        amount: u128,
        denom: &str,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            Addr::unchecked(&self.eclipsepad_staking_contract()),
            &eclipse_base::staking::msg::ExecuteMsg::Stake {},
            &coins(amount, denom.to_string()),
        )
    }

    pub fn eclipsepad_staking_try_lock(
        &mut self,
        sender: &str,
        amount: u128,
        lock_tier: u64,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            Addr::unchecked(&self.eclipsepad_staking_contract()),
            &eclipse_base::staking::msg::ExecuteMsg::Lock {
                amount: Uint128::new(amount),
                lock_tier,
            },
            &[],
        )
    }

    pub fn eclipsepad_staking_query_essence(
        &self,
        user: &str,
    ) -> StdResult<eclipse_base::staking::msg::QueryEssenceResponse> {
        self.app.wrap().query_wasm_smart(
            Addr::unchecked(&self.eclipsepad_staking_contract()),
            &eclipse_base::staking::msg::QueryMsg::QueryEssence {
                user: user.to_string(),
            },
        )
    }

    pub fn eclipsepad_staking_query_total_essence(
        &self,
    ) -> StdResult<eclipse_base::staking::msg::QueryEssenceResponse> {
        self.app.wrap().query_wasm_smart(
            Addr::unchecked(&self.eclipsepad_staking_contract()),
            &eclipse_base::staking::msg::QueryMsg::QueryTotalEssence {},
        )
    }

    // voter contract
    pub fn voter_swap_to_eclip_astro(
        &mut self,
        sender: &str,
        amount: u128,
        denom: &str,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.voter_contract.clone(),
            &VoterExecuteMsg::SwapToEclipAstro {},
            &coins(amount, denom),
        )
    }

    pub fn update_voter_config(
        &mut self,
        sender: &str,
        config: VoterUpdateConfig,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.voter_contract.clone(),
            &VoterExecuteMsg::UpdateConfig { config },
            &[],
        )
    }

    pub fn update_voter_owner(&mut self, sender: &str, new_owner: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.voter_contract.clone(),
            &VoterExecuteMsg::UpdateOwner {
                owner: Addr::unchecked(new_owner).into_string(),
            },
            &[],
        )
    }

    pub fn voter_vote(
        &mut self,
        sender: &str,
        voting_list: &[equinox_msg::voter::VotingListItem],
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.voter_contract.clone(),
            &VoterExecuteMsg::Vote {
                voting_list: voting_list.to_owned(),
            },
            &[],
        )
    }

    pub fn query_voter_config(&self) -> StdResult<VoterConfig> {
        let config: VoterConfig = self
            .app
            .wrap()
            .query_wasm_smart(self.voter_contract.clone(), &VoterQueryMsg::Config {})?;
        Ok(config)
    }

    pub fn query_voter_convert_ratio(&self) -> StdResult<(Uint128, Uint128)> {
        let config: (Uint128, Uint128) = self
            .app
            .wrap()
            .query_wasm_smart(self.voter_contract.clone(), &VoterQueryMsg::ConvertRatio {})?;
        Ok(config)
    }

    pub fn query_voter_owner(&self) -> StdResult<String> {
        let owner: Addr = self
            .app
            .wrap()
            .query_wasm_smart(self.voter_contract.clone(), &VoterQueryMsg::Owner {})?;
        Ok(owner.into_string())
    }

    pub fn voter_query_voting_power(&self, address: &str) -> StdResult<Uint128> {
        self.app.wrap().query_wasm_smart(
            self.voter_contract.clone(),
            &VoterQueryMsg::VotingPower {
                address: address.to_string(),
            },
        )
    }

    pub fn voter_query_voter_info(
        &self,
        address: &str,
    ) -> StdResult<astroport_governance::generator_controller::UserInfoResponse> {
        self.app.wrap().query_wasm_smart(
            self.voter_contract.clone(),
            &VoterQueryMsg::VoterInfo {
                address: address.to_string(),
            },
        )
    }

    // flexible_stake contract
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

    pub fn balance_native(&mut self, recipient: String, denom: String) -> StdResult<u128> {
        let balance = self
            .app
            .wrap()
            .query_balance(recipient, denom)
            .unwrap()
            .amount;
        Ok(balance.u128())
    }

    pub fn update_flexible_stake_config(
        &mut self,
        sender: &str,
        config: FlexibleStakingUpdateConfig,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.flexible_staking_contract.clone(),
            &FlexibleStakingExecuteMsg::UpdateConfig { config },
            &[],
        )
    }

    pub fn update_flexible_stake_owner(
        &mut self,
        sender: &str,
        new_owner: &str,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.flexible_staking_contract.clone(),
            &FlexibleStakingExecuteMsg::UpdateOwner {
                owner: Addr::unchecked(new_owner.to_string()).into_string(),
            },
            &[],
        )
    }

    pub fn update_flexible_allow_users(
        &mut self,
        sender: &str,
        users: Vec<String>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.flexible_staking_contract.clone(),
            &FlexibleStakingExecuteMsg::AllowUsers { users },
            &[],
        )
    }

    pub fn update_flexible_block_users(
        &mut self,
        sender: &str,
        users: Vec<String>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.flexible_staking_contract.clone(),
            &FlexibleStakingExecuteMsg::BlockUsers { users },
            &[],
        )
    }

    // pub fn flexible_stake(&mut self, sender: &str, amount: u128) -> AnyResult<AppResponse> {
    //     self.app.execute_contract(
    //         Addr::unchecked(sender),
    //         self.eclipastro_contract.clone(),
    //         &Cw20ExecuteMsg::Send {
    //             contract: self.flexible_staking_contract(),
    //             amount: Uint128::from(amount),
    //             msg: to_json_binary(&FlexibleStakingCw20HookMsg::Stake {})?,
    //         },
    //         &[],
    //     )
    // }

    pub fn flexible_relock(
        &mut self,
        sender: &str,
        duration: u64,
        amount: Option<Uint128>,
        recipient: Option<String>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.flexible_staking_contract.clone(),
            &FlexibleStakingCw20HookMsg::Relock {
                duration,
                amount,
                recipient,
            },
            &[],
        )
    }

    // pub fn flexible_relock_with_deposit(
    //     &mut self,
    //     sender: &str,
    //     stake_amount: u128,
    //     duration: u64,
    //     amount: Option<Uint128>,
    //     recipient: Option<String>,
    // ) -> AnyResult<AppResponse> {
    //     self.app.execute_contract(
    //         Addr::unchecked(sender),
    //         self.eclipastro_contract.clone(),
    //         &Cw20ExecuteMsg::Send {
    //             contract: self.flexible_staking_contract(),
    //             amount: Uint128::from(stake_amount),
    //             msg: to_json_binary(&FlexibleStakingCw20HookMsg::Relock {
    //                 duration,
    //                 amount,
    //                 recipient,
    //             })?,
    //         },
    //         &[],
    //     )
    // }

    pub fn flexible_claim(&mut self, sender: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.flexible_staking_contract.clone(),
            &FlexibleStakingExecuteMsg::Claim {},
            &[],
        )
    }

    pub fn flexible_unstake(
        &mut self,
        sender: &str,
        amount: u128,
        recipient: Option<String>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.flexible_staking_contract.clone(),
            &FlexibleStakingExecuteMsg::Unstake {
                amount: Uint128::from(amount),
                recipient,
            },
            &[],
        )
    }

    pub fn query_flexible_stake_config(&self) -> StdResult<FlexibleStakingConfig> {
        let config: FlexibleStakingConfig = self.app.wrap().query_wasm_smart(
            self.flexible_staking_contract.clone(),
            &FlexibleStakingQueryMsg::Config {},
        )?;
        Ok(config)
    }

    pub fn query_flexible_stake_owner(&self) -> StdResult<String> {
        let owner: Addr = self.app.wrap().query_wasm_smart(
            self.flexible_staking_contract.clone(),
            &FlexibleStakingQueryMsg::Owner {},
        )?;
        Ok(owner.into_string())
    }
    pub fn query_flexible_is_allowed(&self, user: String) -> StdResult<bool> {
        Ok(self.app.wrap().query_wasm_smart(
            self.flexible_staking_contract.clone(),
            &FlexibleStakingQueryMsg::IsAllowed { user },
        )?)
    }

    pub fn query_flexible_staking(&self, user: &str) -> StdResult<u128> {
        let amount: Uint128 = self.app.wrap().query_wasm_smart(
            self.flexible_staking_contract.clone(),
            &FlexibleStakingQueryMsg::Staking {
                user: user.to_string(),
            },
        )?;
        Ok(amount.u128())
    }

    pub fn query_total_flexible_staking(&self) -> StdResult<u128> {
        let amount: Uint128 = self.app.wrap().query_wasm_smart(
            self.flexible_staking_contract.clone(),
            &FlexibleStakingQueryMsg::TotalStaking {},
        )?;
        Ok(amount.u128())
    }

    pub fn query_flexible_stake_reward(&self, user: &str) -> StdResult<FlexibleReward> {
        let rewards = self.app.wrap().query_wasm_smart(
            self.flexible_staking_contract.clone(),
            &FlexibleStakingQueryMsg::Reward {
                user: user.to_string(),
            },
        )?;
        Ok(rewards)
    }

    // timelock_staking contract
    pub fn update_timelock_stake_config(
        &mut self,
        sender: &str,
        config: TimelockStakingUpdateConfig,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.timelock_staking_contract.clone(),
            &TimelockStakingExecuteMsg::UpdateConfig { config },
            &[],
        )
    }

    pub fn update_timelock_allow_users(
        &mut self,
        sender: &str,
        users: Vec<String>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.timelock_staking_contract.clone(),
            &TimelockStakingExecuteMsg::AllowUsers { users },
            &[],
        )
    }

    pub fn update_timelock_block_users(
        &mut self,
        sender: &str,
        users: Vec<String>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.timelock_staking_contract.clone(),
            &TimelockStakingExecuteMsg::BlockUsers { users },
            &[],
        )
    }

    pub fn update_timelock_stake_owner(
        &mut self,
        sender: &str,
        new_owner: &str,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.timelock_staking_contract.clone(),
            &TimelockStakingExecuteMsg::UpdateOwner {
                owner: Addr::unchecked(new_owner.to_string()).into_string(),
            },
            &[],
        )
    }

    // pub fn timelock_stake(
    //     &mut self,
    //     sender: &str,
    //     amount: u128,
    //     duration: u64,
    //     recipient: Option<String>,
    // ) -> AnyResult<AppResponse> {
    //     self.app.execute_contract(
    //         Addr::unchecked(sender),
    //         self.eclipastro_contract.clone(),
    //         &Cw20ExecuteMsg::Send {
    //             contract: self.timelock_staking_contract(),
    //             amount: Uint128::from(amount),
    //             msg: to_json_binary(&TimelockCw20HookMsg::Lock {
    //                 duration,
    //                 recipient,
    //             })?,
    //         },
    //         &[],
    //     )
    // }

    pub fn timelock_unstake(
        &mut self,
        sender: &str,
        duration: u64,
        locked_at: u64,
        amount: Option<Uint128>,
        recipient: Option<String>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.timelock_staking_contract.clone(),
            &TimelockStakingExecuteMsg::Unlock {
                duration,
                locked_at,
                amount,
                recipient,
            },
            &[],
        )
    }

    pub fn timelock_claim(
        &mut self,
        sender: &str,
        duration: u64,
        locked_at: u64,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.timelock_staking_contract.clone(),
            &TimelockStakingExecuteMsg::Claim {
                duration,
                locked_at,
            },
            &[],
        )
    }

    pub fn timelock_claim_all(&mut self, sender: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.timelock_staking_contract.clone(),
            &TimelockStakingExecuteMsg::ClaimAll {},
            &[],
        )
    }

    pub fn timelock_restake(
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
            self.timelock_staking_contract.clone(),
            &TimelockStakingExecuteMsg::Relock {
                from_duration,
                to_duration,
                relocks: vec![(locked_at, amount)],
                recipient,
            },
            &[],
        )
    }

    // pub fn timelock_restake_with_deposit(
    //     &mut self,
    //     sender: &str,
    //     from_duration: u64,
    //     locked_at: u64,
    //     to_duration: u64,
    //     stake_amount: u128,
    //     recipient: Option<String>,
    //     amount: Option<Uint128>,
    // ) -> AnyResult<AppResponse> {
    //     self.app.execute_contract(
    //         Addr::unchecked(sender),
    //         self.eclipastro_contract.clone(),
    //         &Cw20ExecuteMsg::Send {
    //             contract: self.timelock_staking_contract(),
    //             amount: Uint128::from(stake_amount),
    //             msg: to_json_binary(&TimelockStakingExecuteMsg::Relock {
    //                 from_duration,
    //                 to_duration,
    //                 relocks: vec![(locked_at, amount)],
    //                 recipient,
    //             })?,
    //         },
    //         &[],
    //     )
    // }

    pub fn query_timelock_stake_config(&self) -> StdResult<TimelockStakingConfig> {
        let config: TimelockStakingConfig = self.app.wrap().query_wasm_smart(
            self.timelock_staking_contract.clone(),
            &TimelockStakingQueryMsg::Config {},
        )?;
        Ok(config)
    }
    pub fn query_timelock_is_allowed(&self, user: String) -> StdResult<bool> {
        Ok(self.app.wrap().query_wasm_smart(
            self.timelock_staking_contract.clone(),
            &TimelockStakingQueryMsg::IsAllowed { user },
        )?)
    }

    pub fn query_timelock_stake_owner(&self) -> StdResult<String> {
        let owner: Addr = self.app.wrap().query_wasm_smart(
            self.timelock_staking_contract.clone(),
            &TimelockStakingQueryMsg::Owner {},
        )?;
        Ok(owner.into_string())
    }

    pub fn query_timelock_staking(&self, user: &str) -> StdResult<Vec<TimelockUserStaking>> {
        let user_staking: Vec<TimelockUserStaking> = self.app.wrap().query_wasm_smart(
            self.timelock_staking_contract.clone(),
            &TimelockStakingQueryMsg::Staking {
                user: user.to_string(),
            },
        )?;
        Ok(user_staking)
    }

    pub fn query_timelock_total_staking(&self) -> StdResult<u128> {
        let total_staking: Uint128 = self.app.wrap().query_wasm_smart(
            self.timelock_staking_contract.clone(),
            &TimelockStakingQueryMsg::TotalStaking {},
        )?;
        Ok(total_staking.u128())
    }

    pub fn query_timelock_staking_reward(&self, user: &str) -> StdResult<Vec<TimelockReward>> {
        let reward: Vec<TimelockReward> = self.app.wrap().query_wasm_smart(
            self.timelock_staking_contract.clone(),
            &TimelockStakingQueryMsg::Reward {
                user: user.to_string(),
            },
        )?;
        Ok(reward)
    }

    pub fn query_reward_distributor_total_staking(&self) -> StdResult<TotalStakingData> {
        let total_staking = self.app.wrap().query_wasm_smart(
            self.reward_distributor_contract.clone(),
            &RewardDistributorQueryMsg::TotalStaking {},
        )?;
        Ok(total_staking)
    }

    pub fn query_reward_distributor_pending_rewards(&self) -> StdResult<Vec<(u64, Uint128)>> {
        let pending_rewards = self.app.wrap().query_wasm_smart(
            self.reward_distributor_contract.clone(),
            &RewardDistributorQueryMsg::PendingRewards {},
        )?;
        Ok(pending_rewards)
    }

    pub fn lp_staking_update_config(
        &mut self,
        sender: &str,
        new_config: LpStakingUpdateConfigMsg,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lp_staking.clone(),
            &LpStakingExecuteMsg::UpdateConfig { config: new_config },
            &[],
        )
    }

    pub fn lp_staking_update_reward_config(
        &mut self,
        sender: &str,
        new_config: RewardConfig,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lp_staking.clone(),
            &LpStakingExecuteMsg::UpdateRewardConfig { config: new_config },
            &[],
        )
    }

    pub fn stake_lp_token(&mut self, sender: &str, amount: u128) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.eclipastro_xastro_lp_token_contract.clone(),
            &Cw20ExecuteMsg::Send {
                contract: self.lp_staking.to_string(),
                amount: Uint128::from(amount),
                msg: to_json_binary(&LpStakingCw20HookMsg::Stake {})?,
            },
            &[],
        )
    }

    pub fn lp_staking_claim_rewards(&mut self, sender: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lp_staking.clone(),
            &LpStakingExecuteMsg::Claim {},
            &[],
        )
    }

    pub fn unstake_lp_token(
        &mut self,
        sender: &str,
        amount: u128,
        recipient: Option<String>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lp_staking.clone(),
            &LpStakingExecuteMsg::Unstake {
                amount: Uint128::from(amount),
                recipient,
            },
            &[],
        )
    }

    pub fn query_lp_staking_config(&self) -> StdResult<LpStakingConfig> {
        let config: LpStakingConfig = self
            .app
            .wrap()
            .query_wasm_smart(self.lp_staking.clone(), &LpStakingQueryMsg::Config {})?;
        Ok(config)
    }

    pub fn query_user_lp_token_staking(&self, user: &str) -> StdResult<UserStaking> {
        let res: UserStaking = self.app.wrap().query_wasm_smart(
            self.lp_staking.clone(),
            &LpStakingQueryMsg::Staking {
                user: user.to_string(),
            },
        )?;
        Ok(res)
    }

    pub fn query_user_lp_staking_reward(
        &self,
        user: &str,
    ) -> StdResult<Vec<LpStakingUserRewardResponse>> {
        let res: Vec<LpStakingUserRewardResponse> = self.app.wrap().query_wasm_smart(
            self.lp_staking.clone(),
            &LpStakingQueryMsg::Reward {
                user: user.to_string(),
            },
        )?;
        Ok(res)
    }

    pub fn query_total_lp_token_staking(&self) -> StdResult<TotalStaking> {
        let res: TotalStaking = self
            .app
            .wrap()
            .query_wasm_smart(self.lp_staking.clone(), &LpStakingQueryMsg::TotalStaking {})?;
        Ok(res)
    }

    // lockdrop
    pub fn update_lockdrop_config(
        &mut self,
        sender: &str,
        new_config: LockdropUpdateConfigMsg,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop.clone(),
            &LockdropExecuteMsg::UpdateConfig { new_config },
            &[],
        )
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
            Addr::unchecked(token),
            &Cw20ExecuteMsg::Send {
                contract: self.lockdrop.to_string(),
                amount: Uint128::from(amount),
                msg: to_json_binary(&LockdropCw20HookMsg::IncreaseLockup {
                    stake_type: StakeType::SingleStaking,
                    duration,
                })?,
            },
            &[],
        )
    }

    pub fn single_staking_extend_duration_without_deposit(
        &mut self,
        sender: &str,
        from_duration: u64,
        to_duration: u64,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop.clone(),
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
            Addr::unchecked(token),
            &Cw20ExecuteMsg::Send {
                contract: self.lockdrop.to_string(),
                amount: Uint128::from(amount),
                msg: to_json_binary(&LockdropCw20HookMsg::ExtendDuration {
                    stake_type: StakeType::SingleStaking,
                    from: from_duration,
                    to: to_duration,
                })?,
            },
            &[],
        )
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
            Addr::unchecked(token),
            &Cw20ExecuteMsg::Send {
                contract: self.lockdrop.to_string(),
                amount: Uint128::from(amount),
                msg: to_json_binary(&LockdropCw20HookMsg::IncreaseLockup {
                    stake_type: StakeType::LpStaking,
                    duration,
                })?,
            },
            &[],
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
            self.lockdrop.clone(),
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
            Addr::unchecked(token),
            &Cw20ExecuteMsg::Send {
                contract: self.lockdrop.to_string(),
                amount: Uint128::from(amount),
                msg: to_json_binary(&LockdropCw20HookMsg::ExtendDuration {
                    stake_type: StakeType::LpStaking,
                    from: from_duration,
                    to: to_duration,
                })?,
            },
            &[],
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
            self.lockdrop.clone(),
            &LockdropExecuteMsg::SingleLockupWithdraw { amount, duration },
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
            self.lockdrop.clone(),
            &LockdropExecuteMsg::LpLockupWithdraw { amount, duration },
            &[],
        )
    }

    pub fn increase_eclip_incentives_lockdrop(
        &mut self,
        sender: &str,
        amount: u128,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop.clone(),
            &LockdropExecuteMsg::IncreaseEclipIncentives {},
            &[Coin {
                denom: self.eclip(),
                amount: Uint128::from(amount),
            }],
        )
    }

    pub fn lockdrop_stake_to_vaults(&mut self, sender: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop.clone(),
            &LockdropExecuteMsg::StakeToVaults {},
            &[],
        )
    }

    pub fn single_lockdrop_claim_rewards_and_optionally_unlock(
        &mut self,
        sender: &str,
        duration: u64,
        amount: Option<Uint128>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop.clone(),
            &LockdropExecuteMsg::ClaimRewardsAndOptionallyUnlock {
                stake_type: StakeType::SingleStaking,
                duration,
                amount,
            },
            &[],
        )
    }

    pub fn lp_lockdrop_claim_rewards_and_optionally_unlock(
        &mut self,
        sender: &str,
        duration: u64,
        amount: Option<Uint128>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop.clone(),
            &LockdropExecuteMsg::ClaimRewardsAndOptionallyUnlock {
                stake_type: StakeType::LpStaking,
                duration,
                amount,
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
            self.lockdrop.clone(),
            &LockdropExecuteMsg::RelockSingleStaking { from, to },
            &[],
        )
    }

    // pub fn single_lockup_relock_with_deposit(
    //     &mut self,
    //     sender: &str,
    //     from: u64,
    //     to: u64,
    //     amount: u128,
    // ) -> AnyResult<AppResponse> {
    //     self.app.execute_contract(
    //         Addr::unchecked(sender),
    //         self.eclipastro_contract.clone(),
    //         &Cw20ExecuteMsg::Send {
    //             contract: self.lockdrop.to_string(),
    //             amount: Uint128::from(amount),
    //             msg: to_json_binary(&LockdropCw20HookMsg::Relock { from, to })?,
    //         },
    //         &[],
    //     )
    // }

    pub fn query_lockdrop_config(&self) -> StdResult<LockdropConfig> {
        let res: LockdropConfig = self
            .app
            .wrap()
            .query_wasm_smart(self.lockdrop.clone(), &LockdropQueryMsg::Config {})?;
        Ok(res)
    }

    pub fn query_single_lockup_info(&self) -> StdResult<Vec<LockupInfoResponse>> {
        let res: Vec<LockupInfoResponse> = self.app.wrap().query_wasm_smart(
            self.lockdrop.clone(),
            &LockdropQueryMsg::SingleLockupInfo {},
        )?;
        Ok(res)
    }

    pub fn query_lp_lockup_info(&self) -> StdResult<Vec<LockupInfoResponse>> {
        let res: Vec<LockupInfoResponse> = self
            .app
            .wrap()
            .query_wasm_smart(self.lockdrop.clone(), &LockdropQueryMsg::LpLockupInfo {})?;
        Ok(res)
    }

    pub fn query_single_lockup_state(&self) -> StdResult<SingleLockupStateResponse> {
        let res: SingleLockupStateResponse = self.app.wrap().query_wasm_smart(
            self.lockdrop.clone(),
            &LockdropQueryMsg::SingleLockupState {},
        )?;
        Ok(res)
    }

    pub fn query_lp_lockup_state(&self) -> StdResult<LpLockupStateResponse> {
        let res: LpLockupStateResponse = self
            .app
            .wrap()
            .query_wasm_smart(self.lockdrop.clone(), &LockdropQueryMsg::LpLockupState {})?;
        Ok(res)
    }

    pub fn query_user_single_lockup_info(
        &self,
        user: &str,
    ) -> StdResult<Vec<UserSingleLockupInfoResponse>> {
        let res: Vec<UserSingleLockupInfoResponse> = self.app.wrap().query_wasm_smart(
            self.lockdrop.clone(),
            &LockdropQueryMsg::UserSingleLockupInfo {
                user: Addr::unchecked(user),
            },
        )?;
        Ok(res)
    }

    pub fn query_user_lp_lockup_info(
        &self,
        user: &str,
    ) -> StdResult<Vec<UserLpLockupInfoResponse>> {
        let res: Vec<UserLpLockupInfoResponse> = self.app.wrap().query_wasm_smart(
            self.lockdrop.clone(),
            &LockdropQueryMsg::UserLpLockupInfo {
                user: Addr::unchecked(user),
            },
        )?;
        Ok(res)
    }

    pub fn calculate_penalty(
        &self,
        amount: u128,
        duration: u64,
        locked_at: u64,
    ) -> StdResult<u128> {
        let penalty: Uint128 = self.app.wrap().query_wasm_smart(
            self.timelock_staking_contract.clone(),
            &TimelockStakingQueryMsg::CalculatePenalty {
                amount: Uint128::from(amount),
                duration,
                locked_at,
            },
        )?;
        Ok(penalty.u128())
    }

    pub fn minter_try_mint(
        &mut self,
        sender: &str,
        denom: &str,
        amount: u128,
        mint_to_address: impl ToString,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.minter_contract.clone(),
            &eclipse_base::minter::msg::ExecuteMsg::Mint {
                token: TokenUnverified::new_native(denom),
                amount: Uint128::new(amount),
                recipient: mint_to_address.to_string(),
            },
            &[],
        )
    }

    pub fn minter_try_burn(
        &mut self,
        sender: &str,
        denom: &str,
        amount: u128,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.minter_contract.clone(),
            &eclipse_base::minter::msg::ExecuteMsg::Burn {},
            &[coin(amount, denom.to_string())],
        )
    }

    pub fn minter_try_register_denom(
        &mut self,
        sender: &str,
        denom: &str,
        creator: &str,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.minter_contract.clone(),
            &eclipse_base::minter::msg::ExecuteMsg::RegisterCurrency {
                currency: Currency::new(&TokenUnverified::new_native(denom), 6),
                creator: creator.to_string(),
            },
            &[],
        )
    }
}
