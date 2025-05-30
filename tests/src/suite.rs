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
    coin, coins,
    testing::{MockApi, MockStorage},
    Addr, Api, BlockInfo, CanonicalAddr, Decimal, DepsMut, Empty, Env, GovMsg, IbcMsg, IbcQuery,
    MessageInfo, RecoverPubkeyError, Response, StdError, StdResult, Storage, Timestamp, Uint128,
    VerificationError,
};
use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};
use cw_multi_test::{
    AddressGenerator, App, AppBuilder, AppResponse, BankKeeper, ContractWrapper,
    DistributionKeeper, Executor, FailingModule, StakeKeeper, WasmKeeper,
};
use eclipse_base::{
    minter::msg::ExecuteMsg as EclipsepadMinterExecuteMsg,
    staking::{msg::ExecuteMsg as EclipStakingExecuteMsg, types::PaginationConfig},
    voter::{
        msg::AstroStakingRewardResponse,
        state::{EPOCH_LENGTH, GENESIS_EPOCH_START_DATE, VOTE_DELAY},
    },
};
use equinox_msg::{
    lockdrop::{
        BlacklistRewards, Config as LockdropConfig, Cw20HookMsg as LockdropCw20HookMsg,
        ExecuteMsg as LockdropExecuteMsg, IncentiveAmounts, IncentiveRewards,
        InstantiateMsg as LockdropInstantiateMsg, LockConfig, LpLockupInfoResponse,
        LpLockupStateResponse, QueryMsg as LockdropQueryMsg, SingleLockupInfoResponse,
        SingleLockupStateResponse, StakeType, UpdateConfigMsg as LockdropUpdateConfigMsg,
        UserLpLockupInfoResponse, UserSingleLockupInfoResponse,
    },
    lp_staking::{
        Config as LpStakingConfig, ExecuteMsg as LpStakingExecuteMsg,
        InstantiateMsg as LpStakingInstantiateMsg, QueryMsg as LpStakingQueryMsg,
        RewardAmount as LpStakingRewardAmount, RewardDistribution,
        RewardWeight as LpStakingRewardWeight, UpdateConfigMsg as LpStakingUpdateConfigMsg,
        UserStaking as LpStakingUserStaking,
    },
    single_sided_staking::{
        Config as SingleStakingConfig, ExecuteMsg as SingleSidedStakingExecuteMsg,
        InstantiateMsg as SingleSidedStakingInstantiateMsg, QueryMsg as SingleStakingQueryMsg,
        TimeLockConfig, UnbondedItem, UpdateConfigMsg as SingleStakingUpdateConfigMsg, UserReward,
        UserStaking as SingleSidedUserStaking,
    },
};

use crate::common::stargate::MockStargate;

fn store_astro_staking(app: &mut TestApp) -> u64 {
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

fn store_tracking_code(app: &mut TestApp) -> u64 {
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

fn store_astroport_token(app: &mut TestApp) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    ));

    app.store_code(contract)
}

fn store_astroport_pair(app: &mut TestApp) -> u64 {
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

fn store_astroport_factory(app: &mut TestApp) -> u64 {
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

fn store_astroport_incentives(app: &mut TestApp) -> u64 {
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

fn store_astroport_vesting(app: &mut TestApp) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        astroport_vesting::contract::execute,
        astroport_vesting::contract::instantiate,
        astroport_vesting::contract::query,
    ));

    app.store_code(contract)
}

// it's actually cw20-base as we don't need to test cw20-gift features here
fn store_cw20_gift(app: &mut TestApp) -> u64 {
    app.store_code(Box::new(ContractWrapper::new_with_empty(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    )))
}

fn store_minter(app: &mut TestApp) -> u64 {
    app.store_code(Box::new(
        ContractWrapper::new_with_empty(
            minter_mocks::contract::execute,
            minter_mocks::contract::instantiate,
            minter_mocks::contract::query,
        )
        .with_reply(minter_mocks::contract::reply),
    ))
}

fn instantiate_minter(
    app: &mut TestApp,
    code_id: u64,
    admin: &Addr,
    whitelist: &Option<Vec<Addr>>,
    cw20_code_id: &Option<u64>,
    permissionless_token_creation: &Option<bool>,
    permissionless_token_registration: &Option<bool>,
    max_tokens_per_owner: &Option<u16>,
) -> Addr {
    app.instantiate_contract(
        code_id,
        admin.to_owned(),
        &eclipse_base::minter::msg::InstantiateMsg {
            whitelist: whitelist
                .as_ref()
                .map(|x| x.iter().map(|y| y.to_string()).collect::<Vec<String>>()),
            cw20_code_id: cw20_code_id.to_owned(),
            permissionless_token_creation: permissionless_token_creation.to_owned(),
            permissionless_token_registration: permissionless_token_registration.to_owned(),
            max_tokens_per_owner: max_tokens_per_owner.to_owned(),
        },
        &[],
        "minter",
        Some(admin.to_string()),
    )
    .unwrap()
}

fn store_voter(app: &mut TestApp) -> u64 {
    app.store_code(Box::new(
        ContractWrapper::new_with_empty(
            voter_mocks::contract::execute,
            voter_mocks::contract::instantiate,
            voter_mocks::contract::query,
        )
        .with_reply_empty(voter_mocks::contract::reply)
        .with_sudo_empty(voter_mocks::contract::sudo),
    ))
}

fn instantiate_voter(
    app: &mut TestApp,
    code_id: u64,
    admin: &Addr,
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
) -> Addr {
    app.instantiate_contract(
        code_id,
        admin.to_owned(),
        &eclipse_base::voter::msg::InstantiateMsg {
            worker_list: worker_list.map(|x| x.into_iter().map(|y| y.to_string()).collect()),

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
        "voter",
        Some(admin.to_string()),
    )
    .unwrap()
}

fn store_eclipsepad_staking(app: &mut TestApp) -> u64 {
    app.store_code(Box::new(ContractWrapper::new_with_empty(
        eclipsepad_staking::contract::execute,
        eclipsepad_staking::contract::instantiate,
        eclipsepad_staking::contract::query,
    )))
}

fn instantiate_eclipsepad_staking(
    app: &mut TestApp,
    code_id: u64,
    admin: &Addr,
    equinox_voter: Option<String>,
    staking_token: Option<String>,
    beclip_minter: Option<String>,
    beclip_address: Option<String>,
    beclip_whitelist: Option<Vec<String>>,
    lock_schedule: Option<Vec<(u64, u64)>>,
    seconds_per_essence: Option<Uint128>,
    dao_treasury_address: Option<String>,
    penalty_multiplier: Option<Decimal>,
    pagintaion_config: Option<PaginationConfig>,
    eclip_per_second: Option<u64>,
    eclip_per_second_multiplier: Option<Decimal>,
) -> Addr {
    app.instantiate_contract(
        code_id,
        admin.to_owned(),
        &eclipse_base::staking::msg::InstantiateMsg {
            equinox_voter,
            staking_token,
            beclip_minter,
            beclip_address,
            beclip_whitelist,
            lock_schedule,
            seconds_per_essence,
            dao_treasury_address,
            penalty_multiplier,
            pagintaion_config,
            eclip_per_second,
            eclip_per_second_multiplier,
        },
        &[],
        "eclipsepad_staking",
        Some(admin.to_string()),
    )
    .unwrap()
}

fn store_lp_staking(app: &mut TestApp) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(
            lp_staking::contract::execute,
            lp_staking::contract::instantiate,
            lp_staking::contract::query,
        )
        .with_reply(lp_staking::contract::reply),
    );

    app.store_code(contract)
}

fn store_single_staking(app: &mut TestApp) -> u64 {
    let contract = Box::new(
        ContractWrapper::new_with_empty(
            single_sided_staking::contract::execute,
            single_sided_staking::contract::instantiate,
            single_sided_staking::contract::query,
        )
        .with_reply(single_sided_staking::contract::reply),
    );

    app.store_code(contract)
}

fn store_lockdrop(app: &mut TestApp) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        lockdrop::contract::execute,
        lockdrop::contract::instantiate,
        lockdrop::contract::query,
    ));

    app.store_code(contract)
}

pub struct TestApi {
    mock_api: MockApi,
}

impl TestApi {
    pub fn new() -> Self {
        Self {
            mock_api: MockApi::default(),
        }
    }
}

impl Api for TestApi {
    fn addr_validate(&self, input: &str) -> StdResult<Addr> {
        if input.starts_with(TestAddr::ADDR_PREFIX) {
            self.mock_api.addr_validate(input)
        } else {
            Err(StdError::generic_err(format!(
                "TestApi: address {input} does not start with {}",
                TestAddr::ADDR_PREFIX
            )))
        }
    }

    fn addr_canonicalize(&self, human: &str) -> StdResult<CanonicalAddr> {
        self.mock_api.addr_canonicalize(human)
    }

    fn addr_humanize(&self, canonical: &CanonicalAddr) -> StdResult<Addr> {
        self.mock_api.addr_humanize(canonical)
    }

    fn secp256k1_verify(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        self.mock_api
            .secp256k1_verify(message_hash, signature, public_key)
    }

    fn secp256k1_recover_pubkey(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        recovery_param: u8,
    ) -> Result<Vec<u8>, RecoverPubkeyError> {
        self.mock_api
            .secp256k1_recover_pubkey(message_hash, signature, recovery_param)
    }

    fn ed25519_verify(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        self.mock_api.ed25519_verify(message, signature, public_key)
    }

    fn ed25519_batch_verify(
        &self,
        messages: &[&[u8]],
        signatures: &[&[u8]],
        public_keys: &[&[u8]],
    ) -> Result<bool, VerificationError> {
        self.mock_api
            .ed25519_batch_verify(messages, signatures, public_keys)
    }

    fn debug(&self, message: &str) {
        self.mock_api.debug(message)
    }
}

pub struct TestAddr;

impl TestAddr {
    pub const ADDR_PREFIX: &'static str = "wasm1";
    pub const COUNT_KEY: &'static [u8] = b"address_count";

    pub fn new(seed: &str) -> Addr {
        Addr::unchecked(format!("{}_{seed}", Self::ADDR_PREFIX))
    }
}

impl AddressGenerator for TestAddr {
    fn contract_address(
        &self,
        _api: &dyn Api,
        storage: &mut dyn Storage,
        _code_id: u64,
        _instance_id: u64,
    ) -> AnyResult<Addr> {
        let count = if let Some(next) = storage.get(Self::COUNT_KEY) {
            u64::from_be_bytes(next.as_slice().try_into().unwrap()) + 1
        } else {
            1u64
        };
        storage.set(Self::COUNT_KEY, &count.to_be_bytes());

        Ok(Addr::unchecked(format!(
            "{}_contract{count}",
            Self::ADDR_PREFIX
        )))
    }
}

pub type TestApp<ExecC = Empty, QueryC = Empty> = App<
    BankKeeper,
    TestApi,
    MockStorage,
    FailingModule<ExecC, QueryC, Empty>,
    WasmKeeper<ExecC, QueryC>,
    StakeKeeper,
    DistributionKeeper,
    FailingModule<IbcMsg, IbcQuery, Empty>,
    FailingModule<GovMsg, Empty, Empty>,
    MockStargate,
>;

#[derive(Debug)]
pub struct SuiteBuilder {
    pub admin: Option<String>,
}

pub const INITIAL_LIQUIDITY: u128 = 1_000_000_000_000_000_000;

pub const ASTRO_DENOM: &str = "factory/wasm1_admin/astro";
pub const ADMIN: &str = "wasm1_admin";
pub const TREASURY: &str = "wasm1_treasury";
pub const VXASTRO: &str = "wasm1_vxastro";
pub const FUNDING_DAO: &str = "wasm1_funding_dao";
pub const CE_REWARD_HOLDER: &str = "wasm1_ce_reward_holder";
pub const COIN_REGISTRY: &str = "wasm1_coin_registry";
pub const CHAIN_ID: &str = "cw-multitest-1";

pub const ALICE: &str = "wasm1_alice";
pub const BOB: &str = "wasm1_bob";
pub const CAROL: &str = "wasm1_carol";
pub const ATTACKER: &str = "wasm1_attacker";
// const VICTIM: &str = "victim";

impl SuiteBuilder {
    pub fn new() -> Self {
        Self { admin: None }
    }

    #[track_caller]
    pub fn build(self) -> Suite {
        let admin = Addr::unchecked(ADMIN);

        let mut app = AppBuilder::new()
            .with_stargate(MockStargate::default())
            .with_wasm(WasmKeeper::new().with_address_generator(TestAddr))
            .with_api(TestApi::new())
            .with_block(BlockInfo {
                height: 1,
                time: Timestamp::from_seconds(1696810000),
                chain_id: CHAIN_ID.to_string(),
            })
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

        let _ = app.execute_contract(
            admin.clone(),
            astro_staking_contract.clone(),
            &AstroStakingExecuteMsg::Enter { receiver: None },
            &[coin(10000u128, ASTRO_DENOM.to_string())],
        );

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
            coin_registry_address: COIN_REGISTRY.to_string(),
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
                        denom: ASTRO_DENOM.to_string(),
                    },
                },
                &[],
                "Astroport Vesting",
                None,
            )
            .unwrap();
        let incentives_code_id = store_astroport_incentives(&mut app);
        let astroport_incentives = app
            .instantiate_contract(
                incentives_code_id,
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
                "Astroport incentives",
                None,
            )
            .unwrap();
        let beclip_code_id = store_cw20_gift(&mut app);
        // don't move the contract as eclipAstro denom is hardcoded
        let minter_id = store_minter(&mut app);
        let minter_contract = instantiate_minter(
            &mut app,
            minter_id,
            &admin,
            &None,
            &Some(beclip_code_id),
            &None,
            &None,
            &None,
        );
        // create eclip
        app.execute_contract(
            admin.clone(),
            minter_contract.clone(),
            &EclipsepadMinterExecuteMsg::CreateNative {
                owner: None,
                whitelist: None,
                permissionless_burning: None,
                subdenom: "eclip".to_string(),
                decimals: None,
            },
            &coins(1u128, ASTRO_DENOM),
        )
        .unwrap();
        let eclip = format!("factory/{minter_contract}/eclip");
        // mint eclip
        app.execute_contract(
            admin.clone(),
            minter_contract.clone(),
            &EclipsepadMinterExecuteMsg::Mint {
                denom_or_address: eclip.clone(),
                amount: Uint128::from(2_000_000_000_000u128),
                recipient: None,
            },
            &[],
        )
        .unwrap();
        // create eclipastro
        app.execute_contract(
            admin.clone(),
            minter_contract.clone(),
            &EclipsepadMinterExecuteMsg::CreateNative {
                owner: None,
                whitelist: None,
                permissionless_burning: None,
                subdenom: "eclipASTRO".to_string(),
                decimals: None,
            },
            &coins(1u128, ASTRO_DENOM),
        )
        .unwrap();
        let eclipastro = format!("factory/{minter_contract}/eclipASTRO");
        // create beclip
        let result = app
            .execute_contract(
                admin.clone(),
                minter_contract.clone(),
                &EclipsepadMinterExecuteMsg::CreateCw20 {
                    owner: Some(admin.to_string()),
                    whitelist: None,
                    permissionless_burning: None,
                    cw20_code_id: Some(beclip_code_id),
                    name: "bECLIP".to_string(),
                    symbol: "bECLIP".to_string(),
                    decimals: None,
                    marketing: None,
                },
                &[],
            )
            .unwrap();
        let mut beclip = Addr::unchecked("");
        for event in result.events.iter() {
            for attribute in event.attributes.iter() {
                if attribute.key == "cw20_address".to_string() {
                    beclip = Addr::unchecked(attribute.value.clone());
                }
            }
        }
        let eclipsepad_staking_id = store_eclipsepad_staking(&mut app);
        let eclipsepad_staking_contract = instantiate_eclipsepad_staking(
            &mut app,
            eclipsepad_staking_id,
            &admin,
            None,
            Some(eclip.clone()),
            Some(minter_contract.to_string()),
            Some(beclip.to_string()),
            None,
            Some(vec![
                (2592000, 20547945),
                (7776000, 184931507),
                (15552000, 739726027),
                (23328000, 1664383562),
                (31536000, 3000000000),
            ]),
            None,
            Some(TREASURY.to_string()),
            None,
            None,
            Some(24500),
            None,
        );

        let voter_id = store_voter(&mut app);
        let voter_contract = instantiate_voter(
            &mut app,
            voter_id,
            &admin,
            Some(vec![&admin.to_string()]),
            &admin,
            None,
            &minter_contract,
            &eclipsepad_staking_contract,
            None,
            None,
            &astro_staking_contract,
            &admin,
            &admin,
            &admin,
            &admin,
            None,
            &eclip,
            ASTRO_DENOM,
            &xastro,
            &eclipastro,
            GENESIS_EPOCH_START_DATE,
            EPOCH_LENGTH,
            VOTE_DELAY,
        );
        // add voter contract to eclipastro mint whitelist
        app.execute_contract(
            admin.clone(),
            minter_contract.clone(),
            &EclipsepadMinterExecuteMsg::UpdateCurrencyInfo {
                denom_or_address: eclipastro.clone(),
                owner: None,
                whitelist: Some(vec![voter_contract.to_string()]),
                permissionless_burning: None,
            },
            &[],
        )
        .unwrap();
        // add eclip staking contract to beclip whitelist
        app.execute_contract(
            admin.clone(),
            minter_contract.clone(),
            &EclipsepadMinterExecuteMsg::UpdateCurrencyInfo {
                denom_or_address: beclip.to_string(),
                owner: None,
                whitelist: Some(vec![
                    admin.to_string(),
                    eclipsepad_staking_contract.to_string(),
                ]),
                permissionless_burning: None,
            },
            &vec![],
        )
        .unwrap();

        let single_staking_id = store_single_staking(&mut app);
        let single_staking_contract = app
            .instantiate_contract(
                single_staking_id,
                admin.clone(),
                &SingleSidedStakingInstantiateMsg {
                    owner: admin.to_string(),
                    eclip: eclip.clone(),
                    eclip_staking: eclipsepad_staking_contract.to_string(),
                    beclip: beclip.to_string(),
                    token: eclipastro.clone(),
                    timelock_config: Some(vec![
                        TimeLockConfig {
                            duration: 0,
                            reward_multiplier: 10000,
                        },
                        TimeLockConfig {
                            duration: 86400 * 30,
                            reward_multiplier: 20000,
                        },
                        TimeLockConfig {
                            duration: 86400 * 30 * 3,
                            reward_multiplier: 60000,
                        },
                        TimeLockConfig {
                            duration: 86400 * 30 * 6,
                            reward_multiplier: 120000,
                        },
                        TimeLockConfig {
                            duration: 86400 * 30 * 9,
                            reward_multiplier: 180000,
                        },
                        TimeLockConfig {
                            duration: 86400 * 365,
                            reward_multiplier: 240000,
                        },
                    ]),
                    voter: voter_contract.to_string(),
                    lockdrop: None,
                    treasury: TREASURY.to_string(),
                    blacklist: Some(vec![CAROL.to_string()]),
                    init_early_unlock_penalty: None,
                },
                &[],
                "Single Sided Staking",
                Some(admin.clone().to_string()),
            )
            .unwrap();

        app.execute_contract(
            admin.clone(),
            voter_contract.clone(),
            &eclipse_base::voter::msg::ExecuteMsg::UpdateAddressConfig {
                admin: None,
                worker_list: None,
                eclipse_dao: None,
                eclipsepad_foundry: None,
                eclipsepad_minter: None,
                eclipsepad_staking: None,
                eclipsepad_tribute_market: None,
                eclipse_single_sided_vault: Some(single_staking_contract.to_string()),
                astroport_staking: None,
                astroport_assembly: None,
                astroport_voting_escrow: None,
                astroport_emission_controller: None,
                astroport_router: None,
                astroport_tribute_market: None,
            },
            &[],
        )
        .unwrap();

        // replenish minter balance
        app.sudo(cw_multi_test::SudoMsg::Bank(
            cw_multi_test::BankSudo::Mint {
                to_address: minter_contract.to_string(),
                amount: coins(INITIAL_LIQUIDITY, eclipastro.clone()),
            },
        ))
        .unwrap();

        let asset_infos = vec![
            AssetInfo::NativeToken {
                denom: eclipastro.clone(),
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
                        AssetInfo::NativeToken {
                            denom: eclipastro.clone(),
                        },
                        AssetInfo::NativeToken {
                            denom: xastro.clone(),
                        },
                    ],
                },
            )
            .unwrap();
        let eclipastro_xastro_lp_contract = info.contract_addr;
        let eclipastro_xastro_lp_token = info.liquidity_token;

        let lp_staking_code_id = store_lp_staking(&mut app);
        let lp_staking_contract = app
            .instantiate_contract(
                lp_staking_code_id,
                admin.clone(),
                &LpStakingInstantiateMsg {
                    owner: None,
                    lp_token: AssetInfo::NativeToken {
                        denom: eclipastro_xastro_lp_token.clone(),
                    },
                    lp_contract: eclipastro_xastro_lp_contract.to_string(),
                    lockdrop: None,
                    eclip: eclip.clone(),
                    eclip_staking: eclipsepad_staking_contract.to_string(),
                    beclip: beclip.to_string(),
                    astro: ASTRO_DENOM.to_string(),
                    xastro: xastro.clone(),
                    astro_staking: astro_staking_contract.to_string(),
                    astroport_incentives: astroport_incentives.to_string(),
                    treasury: TREASURY.to_string(),
                    funding_dao: FUNDING_DAO.to_string(),
                    blacklist: Some(vec![CAROL.to_string()]),
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
                    init_timestamp,
                    deposit_window: None,
                    withdrawal_window: None,
                    lock_configs: Some(vec![
                        LockConfig {
                            duration: 0,
                            multiplier: 10000,
                        },
                        LockConfig {
                            duration: 86400 * 30,
                            multiplier: 20000,
                        },
                        LockConfig {
                            duration: 86400 * 30 * 3,
                            multiplier: 60000,
                        },
                        LockConfig {
                            duration: 86400 * 30 * 6,
                            multiplier: 120000,
                        },
                        LockConfig {
                            duration: 86400 * 30 * 9,
                            multiplier: 180000,
                        },
                        LockConfig {
                            duration: 86400 * 365,
                            multiplier: 240000,
                        },
                    ]),
                    astro_token: ASTRO_DENOM.to_string(),
                    xastro_token: xastro.clone(),
                    astro_staking: astro_staking_contract.to_string(),
                    owner: None,
                    beclip: beclip.to_string(),
                    eclip: eclip.clone(),
                    eclip_staking: eclipsepad_staking_contract.to_string(),
                    blacklist: Some(vec![CAROL.to_string()]),
                    init_early_unlock_penalty: None,
                },
                &[],
                "Eclipsefi lockdrop",
                None,
            )
            .unwrap();

        // add voter contract to eclip staking contract and whitelist
        app.execute_contract(
            admin.clone(),
            eclipsepad_staking_contract.clone(),
            &EclipStakingExecuteMsg::UpdateConfig {
                admin: None,
                equinox_voter: Some(voter_contract.to_string()),
                beclip_minter: None,
                beclip_address: None,
                beclip_whitelist: Some(vec![
                    lockdrop_contract.to_string(),
                    single_staking_contract.to_string(),
                    lp_staking_contract.to_string(),
                ]),
                lock_schedule: None,
                dao_treasury_address: None,
                penalty_multiplier: None,
                eclip_per_second_multiplier: None,
            },
            &[],
        )
        .unwrap();

        let funding_dao = Addr::unchecked(FUNDING_DAO);
        let treasury = Addr::unchecked(TREASURY);

        Suite {
            app,
            admin,
            astro: ASTRO_DENOM.to_string(),
            xastro,
            astro_staking_contract,
            eclipastro: eclipastro.clone(),
            beclip,
            eclip: eclip.clone(),
            single_staking_contract,
            lp_staking_contract,
            lockdrop_contract,
            minter_contract,
            voter_contract,
            funding_dao,
            eclipastro_xastro_lp_contract,
            eclipastro_xastro_lp_token,
            astroport_incentives,
            astroport_vesting,
            treasury,
        }
    }
}

pub struct Suite {
    app: TestApp,
    admin: Addr,
    astro: String,
    astro_staking_contract: Addr,
    xastro: String,
    eclipastro: String,
    beclip: Addr,
    eclip: String,
    single_staking_contract: Addr,
    lp_staking_contract: Addr,
    lockdrop_contract: Addr,
    minter_contract: Addr,
    voter_contract: Addr,
    funding_dao: Addr,
    eclipastro_xastro_lp_contract: Addr,
    eclipastro_xastro_lp_token: String,
    astroport_incentives: Addr,
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
    pub fn beclip(&self) -> String {
        self.beclip.to_string()
    }
    pub fn eclip(&self) -> String {
        self.eclip.clone()
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
    pub fn minter_contract(&self) -> String {
        self.minter_contract.to_string()
    }
    pub fn voter_contract(&self) -> String {
        self.voter_contract.to_string()
    }
    pub fn eclipastro_xastro_lp_contract(&self) -> String {
        self.eclipastro_xastro_lp_contract.to_string()
    }
    pub fn eclipastro_xastro_lp_token(&self) -> String {
        self.eclipastro_xastro_lp_token.to_string()
    }
    pub fn astroport_incentives(&self) -> String {
        self.astroport_incentives.to_string()
    }
    pub fn astroport_vesting(&self) -> String {
        self.astroport_vesting.to_string()
    }
    pub fn funding_dao(&self) -> String {
        self.funding_dao.to_string()
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
                self.voter_contract.clone(),
                &eclipse_base::voter::msg::ExecuteMsg::UpdateAddressConfig {
                    admin: None,
                    worker_list: Some(vec![
                        self.single_staking_contract.to_string(),
                        self.lp_staking_contract.to_string(),
                    ]),
                    eclipse_dao: None,
                    eclipsepad_foundry: None,
                    eclipsepad_minter: None,
                    eclipsepad_staking: None,
                    eclipsepad_tribute_market: None,
                    eclipse_single_sided_vault: Some(self.single_staking_contract.to_string()),
                    astroport_staking: None,
                    astroport_assembly: None,
                    astroport_voting_escrow: None,
                    astroport_emission_controller: None,
                    astroport_router: None,
                    astroport_tribute_market: None,
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
        self.app
            .execute_contract(
                self.admin.clone(),
                self.lp_staking_contract.clone(),
                &LpStakingExecuteMsg::AllowUsers {
                    users: vec![self.lockdrop_contract.to_string()],
                },
                &[],
            )
            .unwrap();
    }

    pub fn mint_native(
        &mut self,
        recipient: impl ToString,
        denom: String,
        amount: u128,
    ) -> AnyResult<AppResponse> {
        self.app.sudo(cw_multi_test::SudoMsg::Bank(
            cw_multi_test::BankSudo::Mint {
                to_address: recipient.to_string(),
                amount: vec![coin(amount, denom)],
            },
        ))
    }

    pub fn query_balance_native(
        &mut self,
        recipient: impl ToString,
        denom: String,
    ) -> StdResult<u128> {
        let balance = self
            .app
            .wrap()
            .query_balance(recipient.to_string(), denom)
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

    pub fn unstake_astro(&mut self, sender: &str, xastro_amount: u128) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.astro_staking_contract.clone(),
            &AstroStakingExecuteMsg::Leave {},
            &[coin(xastro_amount, self.xastro())],
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
            self.voter_contract.clone(),
            &eclipse_base::voter::msg::ExecuteMsg::SwapToEclipAstro {},
            &[coin(amount, self.astro())],
        )
    }

    pub fn convert_xastro(&mut self, sender: &str, amount: u128) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.voter_contract.clone(),
            &eclipse_base::voter::msg::ExecuteMsg::SwapToEclipAstro {},
            &[coin(amount, self.xastro())],
        )
    }

    pub fn claim_astro_rewards(&mut self, sender: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.voter_contract.clone(),
            &eclipse_base::voter::msg::ExecuteMsg::ClaimAstroRewards {},
            &[],
        )
    }

    pub fn claim_treasury_rewards(&mut self, sender: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.voter_contract.clone(),
            &eclipse_base::voter::msg::ExecuteMsg::ClaimTreasuryRewards {},
            &[],
        )
    }

    pub fn query_eclipastro_balance(&self, address: &str) -> StdResult<u128> {
        let balance = self
            .app
            .wrap()
            .query_balance(address.to_owned(), self.eclipastro.clone())?;
        Ok(balance.amount.u128())
    }

    // TODO: ?
    // pub fn query_converter_withdrawable_balance(&self) -> StdResult<u128> {
    //     let balance: Uint128 = self.app.wrap().query_wasm_smart(
    //         self.converter_contract.clone(),
    //         &ConverterQueryMsg::WithdrawableBalance {},
    //     )?;
    //     Ok(balance.u128())
    // }

    pub fn query_voter_astro_staking_rewards(&self) -> StdResult<AstroStakingRewardResponse> {
        let rewards: AstroStakingRewardResponse = self.app.wrap().query_wasm_smart(
            self.voter_contract.clone(),
            &eclipse_base::voter::msg::QueryMsg::AstroStakingRewards {},
        )?;
        Ok(rewards)
    }

    // pub fn query_converter_stake_info(&self) -> StdResult<ConverterStakeInfo> {
    //     let info: ConverterStakeInfo = self.app.wrap().query_wasm_smart(
    //         self.converter_contract.clone(),
    //         &ConverterQueryMsg::StakeInfo {},
    //     )?;
    //     Ok(info)
    // }
    // pub fn query_converter_config(&self) -> StdResult<ConverterConfig> {
    //     let info: ConverterConfig = self.app.wrap().query_wasm_smart(
    //         self.converter_contract.clone(),
    //         &ConverterQueryMsg::Config {},
    //     )?;
    //     Ok(info)
    // }

    pub fn query_astro_staking_rewards(&self) -> StdResult<AstroStakingRewardResponse> {
        self.app.wrap().query_wasm_smart(
            self.voter_contract.clone(),
            &eclipse_base::voter::msg::QueryMsg::AstroStakingRewards {},
        )
    }

    pub fn query_astro_staking_treasury_rewards(&self) -> StdResult<Uint128> {
        self.app.wrap().query_wasm_smart(
            self.voter_contract.clone(),
            &eclipse_base::voter::msg::QueryMsg::AstroStakingTreasuryRewards {},
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
            min_lp_to_receive: None,
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
            self.astroport_incentives.clone(),
            &msg,
            &[],
        )
    }
    pub fn incentives_set_tokens_per_second(
        &mut self,
        sender: &str,
        amount: u128,
    ) -> AnyResult<AppResponse> {
        let msg = IncentivesExecuteMsg::SetTokensPerSecond {
            amount: Uint128::from(amount),
        };

        self.app.execute_contract(
            Addr::unchecked(sender),
            self.astroport_incentives.clone(),
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
    pub fn query_lp_staking_reward_distribution(&self) -> StdResult<RewardDistribution> {
        let config: RewardDistribution = self.app.wrap().query_wasm_smart(
            self.lp_staking_contract.clone(),
            &LpStakingQueryMsg::RewardDistribution {},
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
        let res = self
            .app
            .wrap()
            .query_balance(address, self.eclipastro_xastro_lp_token.clone())?;
        Ok(res.amount)
    }
    pub fn query_incentive_pending_rewards(&self, address: &str) -> StdResult<Vec<Asset>> {
        let res: Vec<Asset> = self.app.wrap().query_wasm_smart(
            self.astroport_incentives.clone(),
            &IncentivesQueryMsg::PendingRewards {
                lp_token: self.eclipastro_xastro_lp_token(),
                user: address.to_string(),
            },
        )?;
        Ok(res)
    }

    pub fn query_incentive_deposit(&self, lp_token: &str, address: &str) -> StdResult<Uint128> {
        let res: Uint128 = self.app.wrap().query_wasm_smart(
            self.astroport_incentives.clone(),
            &IncentivesQueryMsg::Deposit {
                lp_token: lp_token.to_string(),
                user: address.to_string(),
            },
        )?;
        Ok(res)
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
    pub fn add_single_sided_vault_reward(
        &mut self,
        sender: &str,
        from: Option<u64>,
        duration: Option<u64>,
        eclip: u128,
        beclip: u128,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.single_staking_contract.clone(),
            &SingleSidedStakingExecuteMsg::AddRewards {
                from,
                duration,
                eclip: Uint128::from(eclip),
                beclip: Uint128::from(beclip),
            },
            &coins(eclip + beclip, self.eclip.clone()),
        )
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
            self.single_staking_contract.clone(),
            &SingleSidedStakingExecuteMsg::Stake {
                duration,
                recipient,
            },
            &[coin(amount, self.eclipastro())],
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

    pub fn single_sided_unbond(
        &mut self,
        sender: &str,
        duration: u64,
        locked_at: u64,
        period: u64,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.single_staking_contract.clone(),
            &SingleSidedStakingExecuteMsg::Unbond {
                duration,
                locked_at,
                period,
            },
            &[],
        )
    }

    pub fn single_sided_withdraw(
        &mut self,
        sender: &str,
        recipient: Option<String>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.single_staking_contract.clone(),
            &SingleSidedStakingExecuteMsg::Withdraw { recipient },
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
        duration: u64,
        locked_at: u64,
    ) -> StdResult<UserReward> {
        let reward: UserReward = self.app.wrap().query_wasm_smart(
            self.single_staking_contract.clone(),
            &SingleStakingQueryMsg::Reward {
                user: user.to_string(),
                duration,
                locked_at,
            },
        )?;
        Ok(reward)
    }
    pub fn query_single_sided_blacklisted_reward(&self) -> StdResult<UserReward> {
        let reward: UserReward = self.app.wrap().query_wasm_smart(
            self.single_staking_contract.clone(),
            &SingleStakingQueryMsg::BlacklistRewards {},
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
        assets: Option<Vec<AssetInfo>>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.single_staking_contract.clone(),
            &SingleSidedStakingExecuteMsg::Claim {
                duration,
                locked_at: Some(locked_at),
                assets,
            },
            &[],
        )
    }
    pub fn single_blacklist_claim(&mut self) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(self.admin()),
            self.single_staking_contract.clone(),
            &SingleSidedStakingExecuteMsg::ClaimBlacklistRewards {},
            &[],
        )
    }
    pub fn single_stake_claim_all(
        &mut self,
        sender: &str,
        with_flexible: bool,
        assets: Option<Vec<AssetInfo>>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.single_staking_contract.clone(),
            &SingleSidedStakingExecuteMsg::ClaimAll {
                with_flexible,
                assets,
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
    // pub fn query_lockdrop_reward_weights(&self) -> StdResult<LockdropConfig> {
    //     let res: LockdropConfig = self
    //         .app
    //         .wrap()
    //         .query_wasm_smart(self.lockdrop_contract.clone(), &LockdropQueryMsg::reward {})?;
    //     Ok(res)
    // }
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
    pub fn query_lockdrop_blacklist_rewards(&self) -> StdResult<BlacklistRewards> {
        let res: BlacklistRewards = self.app.wrap().query_wasm_smart(
            self.lockdrop_contract.clone(),
            &LockdropQueryMsg::BlacklistRewards {},
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
    pub fn add_lp_vault_reward(
        &mut self,
        sender: &str,
        from: Option<u64>,
        duration: Option<u64>,
        eclip: u128,
        beclip: u128,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lp_staking_contract.clone(),
            &LpStakingExecuteMsg::AddRewards {
                from,
                duration,
                eclip: Uint128::from(eclip),
                beclip: Uint128::from(beclip),
            },
            &coins(eclip + beclip, self.eclip.clone()),
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
    pub fn lockdrop_blacklist_rewards_claim(&mut self) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(self.admin()),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::ClaimBlacklistRewards {},
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
    pub fn fund_eclip(
        &mut self,
        sender: &str,
        amount: u128,
        rewards: Vec<IncentiveRewards>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropCw20HookMsg::IncreaseIncentives { rewards },
            &[coin(amount, self.eclip.clone())],
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
    pub fn query_total_lockdrop_incentives(
        &self,
        stake_type: StakeType,
    ) -> StdResult<IncentiveAmounts> {
        let res: IncentiveAmounts = self.app.wrap().query_wasm_smart(
            self.lockdrop_contract.clone(),
            &LockdropQueryMsg::Incentives { stake_type },
        )?;
        Ok(res)
    }
    pub fn single_lockdrop_claim_rewards(
        &mut self,
        sender: &str,
        duration: u64,
        assets: Option<Vec<AssetInfo>>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::ClaimRewards {
                stake_type: StakeType::SingleStaking,
                duration,
                assets,
            },
            &[],
        )
    }
    pub fn single_lockdrop_claim_all_rewards(
        &mut self,
        sender: &str,
        assets: Option<Vec<AssetInfo>>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::ClaimAllRewards {
                stake_type: StakeType::SingleStaking,
                with_flexible: true,
                assets,
            },
            &[],
        )
    }
    pub fn lp_lockdrop_claim_rewards(
        &mut self,
        sender: &str,
        duration: u64,
        assets: Option<Vec<AssetInfo>>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::ClaimRewards {
                stake_type: StakeType::LpStaking,
                duration,
                assets,
            },
            &[],
        )
    }
    pub fn lp_lockdrop_claim_all_rewards(
        &mut self,
        sender: &str,
        assets: Option<Vec<AssetInfo>>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lockdrop_contract.clone(),
            &LockdropExecuteMsg::ClaimAllRewards {
                stake_type: StakeType::LpStaking,
                with_flexible: true,
                assets,
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
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.single_staking_contract.clone(),
            &LockdropExecuteMsg::ExtendLock {
                stake_type: StakeType::SingleStaking,
                from,
                to,
            },
            &[coin(amount, asset)],
        )
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
    pub fn stake_lp_token(&mut self, sender: &str, amount: u128) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lp_staking_contract.clone(),
            &LpStakingExecuteMsg::Stake { recipient: None },
            &[coin(amount, self.eclipastro_xastro_lp_token.clone())],
        )
    }

    pub fn unbond_lp_token(
        &mut self,
        sender: &str,
        amount: Option<u128>,
        period: u64,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lp_staking_contract.clone(),
            &LpStakingExecuteMsg::Unbond {
                amount: amount.map(Uint128::new),
                period,
            },
            &[],
        )
    }

    pub fn withdraw_lp_token(
        &mut self,
        sender: &str,
        recipient: Option<String>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lp_staking_contract.clone(),
            &LpStakingExecuteMsg::Withdraw { recipient },
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

    pub fn query_user_lp_token_unbonded(&self, user: &str) -> StdResult<Vec<UnbondedItem>> {
        self.app.wrap().query_wasm_smart(
            self.lp_staking_contract.clone(),
            &LpStakingQueryMsg::Unbonded {
                user: user.to_string(),
            },
        )
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
    pub fn query_user_reward_weights(&self, user: String) -> StdResult<Vec<LpStakingRewardWeight>> {
        let res: Vec<LpStakingRewardWeight> = self.app.wrap().query_wasm_smart(
            self.lp_staking_contract.clone(),
            &LpStakingQueryMsg::UserRewardWeights { user },
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
    pub fn query_lp_blacklisted_reward(&self) -> StdResult<Vec<LpStakingRewardAmount>> {
        let reward: Vec<LpStakingRewardAmount> = self.app.wrap().query_wasm_smart(
            self.lp_staking_contract.clone(),
            &LpStakingQueryMsg::BlacklistRewards {},
        )?;
        Ok(reward)
    }
    pub fn lp_staking_claim_rewards(&mut self, sender: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lp_staking_contract.clone(),
            &LpStakingExecuteMsg::Claim { assets: None },
            &[],
        )
    }
    pub fn lp_blacklist_claim(&mut self) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(self.admin()),
            self.lp_staking_contract.clone(),
            &LpStakingExecuteMsg::ClaimBlacklistRewards {},
            &[],
        )
    }

    pub fn lp_remove_from_blacklist(&mut self, user: impl ToString) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(self.admin()),
            self.lp_staking_contract.clone(),
            &LpStakingExecuteMsg::RemoveFromBlacklist {
                user: user.to_string(),
            },
            &[],
        )
    }

    pub fn lp_unstake(
        &mut self,
        sender: &str,
        amount: u128,
        recipient: Option<String>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.lp_staking_contract.clone(),
            &LpStakingExecuteMsg::Unstake {
                amount: Uint128::from(amount),
                recipient,
            },
            &[],
        )
    }
    pub fn send_denom(
        &mut self,
        denom: String,
        sender: &str,
        amount: u128,
        recipient: &str,
    ) -> AnyResult<AppResponse> {
        self.app.send_tokens(
            Addr::unchecked(sender),
            Addr::unchecked(recipient),
            &[coin(amount, denom)],
        )
    }
    pub fn send_cw20(
        &mut self,
        asset: Addr,
        sender: &str,
        amount: u128,
        recipient: String,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            asset,
            &Cw20ExecuteMsg::Transfer {
                recipient,
                amount: Uint128::from(amount),
            },
            &[],
        )
    }
}
