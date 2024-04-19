use cosmwasm_std::{Addr, Uint128};
use cw_controllers::AdminError;
use equinox_msg::token_converter::{
    Config as ConverterConfig, Reward, RewardConfig as ConverterRewardConfig, RewardResponse,
    UpdateConfig,
};
use token_converter::ContractError;

use super::suite::SuiteBuilder;

const ONE_MONTH: u64 = 86400 * 30;
const THREE_MONTH: u64 = 86400 * 30 * 3;
const SIX_MONTH: u64 = 86400 * 30 * 6;
const NINE_MONTH: u64 = 86400 * 30 * 9;
const ONE_YEAR: u64 = 86400 * 30 * 12;

const ALICE: &str = "alice";
const BOB: &str = "bob";
const CAROL: &str = "carol";
// const ATTACKER: &str = "attacker";
// const VICTIM: &str = "victim";

#[test]
fn instantiate() {
    let astro_staking_initiator = "astro_staking_initiator";
    let astro_initial_balances = vec![
        (astro_staking_initiator, 1_000_000_000),
        (ALICE, 1_000_000),
        (BOB, 1_000_000),
        (CAROL, 1_000_000),
    ];
    let timelock_config = vec![
        (ONE_MONTH, 100),
        (THREE_MONTH, 100),
        (SIX_MONTH, 100),
        (NINE_MONTH, 100),
        (ONE_YEAR, 100),
    ];
    let eclip_daily_reward = 1_000_000u128;
    let locking_reward_config = vec![
        (0, 1),
        (ONE_MONTH, 2),
        (THREE_MONTH, 3),
        (SIX_MONTH, 4),
        (NINE_MONTH, 5),
        (ONE_YEAR, 6),
    ];

    let mut suite = SuiteBuilder::new()
        .with_initial_balances(astro_initial_balances)
        .with_timelock_config(timelock_config)
        .with_eclip_daily_reward(eclip_daily_reward)
        .with_locking_reward_config(locking_reward_config)
        .build();

    suite.update_config();

    // ready astro_staking_pool
    suite
        .stake_astro(astro_staking_initiator, 1_000_000)
        .unwrap();
}

#[test]
fn update_config() {
    let astro_staking_initiator = "astro_staking_initiator";
    let astro_initial_balances = vec![
        (astro_staking_initiator, 1_000_000_000),
        (ALICE, 1_000_000),
        (BOB, 1_000_000),
        (CAROL, 1_000_000),
    ];
    let timelock_config = vec![
        (ONE_MONTH, 100),
        (THREE_MONTH, 100),
        (SIX_MONTH, 100),
        (NINE_MONTH, 100),
        (ONE_YEAR, 100),
    ];
    let eclip_daily_reward = 1_000_000;
    let locking_reward_config = vec![
        (0, 1),
        (ONE_MONTH, 2),
        (THREE_MONTH, 3),
        (SIX_MONTH, 4),
        (NINE_MONTH, 5),
        (ONE_YEAR, 6),
    ];

    let mut suite = SuiteBuilder::new()
        .with_initial_balances(astro_initial_balances)
        .with_timelock_config(timelock_config)
        .with_eclip_daily_reward(eclip_daily_reward)
        .with_locking_reward_config(locking_reward_config)
        .build();

    let converter_config = suite.query_converter_config().unwrap();
    assert_eq!(
        converter_config,
        ConverterConfig {
            token_in: Addr::unchecked(suite.astro_contract()),
            token_out: Addr::unchecked(suite.eclipastro_contract()),
            xtoken: Addr::unchecked(suite.xastro_contract()),
            vxtoken_holder: Addr::unchecked(""),
            treasury: Addr::unchecked(suite.eclipse_treasury()),
            stability_pool: Addr::unchecked(""),
            staking_reward_distributor: Addr::unchecked(""),
            ce_reward_distributor: Addr::unchecked(""),
        }
    );

    suite.update_config();
    let converter_config = suite.query_converter_config().unwrap();
    assert_eq!(
        converter_config,
        ConverterConfig {
            token_in: Addr::unchecked(suite.astro_contract()),
            token_out: Addr::unchecked(suite.eclipastro_contract()),
            xtoken: Addr::unchecked(suite.xastro_contract()),
            vxtoken_holder: Addr::unchecked(suite.voter_contract()),
            treasury: Addr::unchecked(suite.eclipse_treasury()),
            stability_pool: Addr::unchecked(suite.eclipse_stability_pool()),
            staking_reward_distributor: Addr::unchecked(suite.reward_distributor_contract()),
            ce_reward_distributor: Addr::unchecked(suite.eclipse_ce_reward_distributor()),
        }
    );
    let test_config = UpdateConfig {
        token_in: Some(Addr::unchecked("test").into_string()),
        token_out: Some(Addr::unchecked("test").into_string()),
        xtoken: Some(Addr::unchecked("test").into_string()),
        vxtoken_holder: Some(Addr::unchecked("test").into_string()),
        treasury: Some(Addr::unchecked("test").into_string()),
        stability_pool: Some(Addr::unchecked("test").into_string()),
        staking_reward_distributor: Some(Addr::unchecked("test").into_string()),
        ce_reward_distributor: Some(Addr::unchecked("test").into_string()),
    };
    suite.update_converter_config(test_config);
    let converter_config = suite.query_converter_config().unwrap();
    assert_eq!(
        converter_config,
        ConverterConfig {
            token_in: Addr::unchecked("test"),
            token_out: Addr::unchecked("test"),
            xtoken: Addr::unchecked("test"),
            vxtoken_holder: Addr::unchecked("test"),
            treasury: Addr::unchecked("test"),
            stability_pool: Addr::unchecked("test"),
            staking_reward_distributor: Addr::unchecked("test"),
            ce_reward_distributor: Addr::unchecked("test"),
        }
    );
}

#[test]
fn reward_config() {
    let astro_staking_initiator = "astro_staking_initiator";
    let astro_initial_balances = vec![
        (astro_staking_initiator, 1_000_000_000),
        (ALICE, 1_000_000),
        (BOB, 1_000_000),
        (CAROL, 1_000_000),
    ];
    let timelock_config = vec![
        (ONE_MONTH, 100),
        (THREE_MONTH, 100),
        (SIX_MONTH, 100),
        (NINE_MONTH, 100),
        (ONE_YEAR, 100),
    ];
    let eclip_daily_reward = 1_000_000;
    let locking_reward_config = vec![
        (0, 1),
        (ONE_MONTH, 2),
        (THREE_MONTH, 3),
        (SIX_MONTH, 4),
        (NINE_MONTH, 5),
        (ONE_YEAR, 6),
    ];

    let mut suite = SuiteBuilder::new()
        .with_initial_balances(astro_initial_balances)
        .with_timelock_config(timelock_config)
        .with_eclip_daily_reward(eclip_daily_reward)
        .with_locking_reward_config(locking_reward_config)
        .build();

    let reward_config = suite.query_reward_config().unwrap();
    assert_eq!(
        reward_config,
        ConverterRewardConfig {
            users: 8000,
            treasury: 1350,
            ce_holders: 400,
            stability_pool: 250,
        }
    );
    let invalid_config = ConverterRewardConfig {
        users: 8000,
        treasury: 0,
        ce_holders: 0,
        stability_pool: 0,
    };
    let err = suite
        .update_reward_config(invalid_config.clone())
        .unwrap_err(); // this is vested amount
    assert_eq!(
        ContractError::RewardDistributionErr {},
        err.downcast().unwrap()
    );
    let valid_config = ConverterRewardConfig {
        users: 6000,
        treasury: 2000,
        ce_holders: 1000,
        stability_pool: 1000,
    };
    suite.update_reward_config(valid_config.clone()).unwrap();
    let reward_config = suite.query_reward_config().unwrap();
    assert_eq!(reward_config, valid_config.clone());
}

#[test]
fn update_owner() {
    let astro_staking_initiator = "astro_staking_initiator";
    let astro_initial_balances = vec![
        (astro_staking_initiator, 1_000_000_000),
        (ALICE, 1_000_000),
        (BOB, 1_000_000),
        (CAROL, 1_000_000),
    ];
    let timelock_config = vec![
        (ONE_MONTH, 100),
        (THREE_MONTH, 100),
        (SIX_MONTH, 100),
        (NINE_MONTH, 100),
        (ONE_YEAR, 100),
    ];
    let eclip_daily_reward = 1_000_000;
    let locking_reward_config = vec![
        (0, 1),
        (ONE_MONTH, 2),
        (THREE_MONTH, 3),
        (SIX_MONTH, 4),
        (NINE_MONTH, 5),
        (ONE_YEAR, 6),
    ];

    let mut suite = SuiteBuilder::new()
        .with_initial_balances(astro_initial_balances)
        .with_timelock_config(timelock_config)
        .with_eclip_daily_reward(eclip_daily_reward)
        .with_locking_reward_config(locking_reward_config)
        .build();

    assert_eq!(
        suite.query_converter_owner().unwrap().into_string(),
        suite.admin()
    );

    let new_owner = "new";
    suite.update_owner(new_owner).unwrap();

    assert_eq!(
        suite.query_converter_owner().unwrap().into_string(),
        new_owner.to_string()
    );
}

#[test]
fn convert() {
    let astro_staking_initiator = "astro_staking_initiator";
    let bob_address = Addr::unchecked(BOB);
    let astro_initial_balances = vec![
        (astro_staking_initiator, 2000_000),
        (ALICE, 1_000_000),
        (BOB, 1_000_000),
        (CAROL, 1_000_000),
    ];
    let timelock_config = vec![
        (ONE_MONTH, 100),
        (THREE_MONTH, 100),
        (SIX_MONTH, 100),
        (NINE_MONTH, 100),
        (ONE_YEAR, 100),
    ];
    let eclip_daily_reward = 1_000_000;
    let locking_reward_config = vec![
        (0, 1),
        (ONE_MONTH, 2),
        (THREE_MONTH, 3),
        (SIX_MONTH, 4),
        (NINE_MONTH, 5),
        (ONE_YEAR, 6),
    ];

    let mut suite = SuiteBuilder::new()
        .with_initial_balances(astro_initial_balances)
        .with_timelock_config(timelock_config)
        .with_eclip_daily_reward(eclip_daily_reward)
        .with_locking_reward_config(locking_reward_config)
        .build();

    suite.update_config();

    // ready astro_staking_pool
    suite
        .stake_astro(astro_staking_initiator, 1_000_000)
        .unwrap();

    // Bob convert 1000 astro to eclipastro
    suite
        .convert_astro(&bob_address.clone().into_string(), 1_000)
        .unwrap();

    assert_eq!(
        suite
            .query_eclipastro_balance(&bob_address.clone().into_string())
            .unwrap(),
        1_000
    );
    assert_eq!(
        suite
            .query_astro_balance(&bob_address.clone().into_string())
            .unwrap(),
        999_000
    );
    assert_eq!(
        suite.query_xastro_balance(&suite.voter_contract()).unwrap(),
        1_000
    );

    // change xastro/astro rate
    suite
        .send_astro(
            astro_staking_initiator,
            &suite.astro_staking_contract(),
            100_000,
        )
        .unwrap();

    let total_deposit = suite.query_astro_staking_total_deposit().unwrap();
    let total_shares = suite.query_astro_staking_total_shares().unwrap();
    assert_eq!(total_deposit, 1_101_000);
    assert_eq!(total_shares, 1_001_000);
    // Bob convert 1000 astro to eclipastro
    suite
        .convert_astro(&bob_address.clone().into_string(), 1_000)
        .unwrap();

    assert_eq!(
        suite
            .query_eclipastro_balance(&bob_address.clone().into_string())
            .unwrap(),
        2_000
    );
    assert_eq!(
        suite
            .query_astro_balance(&bob_address.clone().into_string())
            .unwrap(),
        998_000
    );
    assert_eq!(
        suite.query_xastro_balance(&suite.voter_contract()).unwrap(),
        1_000 + 909 // 1_000 * 1_001_000 / 1_101_000
    );
}

#[test]
fn claim_treasury_reward() {
    let astro_staking_initiator = "astro_staking_initiator";
    let bob_address = Addr::unchecked(BOB);
    let astro_initial_balances = vec![
        (astro_staking_initiator, 2000_000),
        (ALICE, 1_000_000),
        (BOB, 1_000_000),
        (CAROL, 1_000_000),
    ];
    let timelock_config = vec![
        (ONE_MONTH, 100),
        (THREE_MONTH, 100),
        (SIX_MONTH, 100),
        (NINE_MONTH, 100),
        (ONE_YEAR, 100),
    ];
    let eclip_daily_reward = 1_000_000;
    let locking_reward_config = vec![
        (0, 1),
        (ONE_MONTH, 2),
        (THREE_MONTH, 3),
        (SIX_MONTH, 4),
        (NINE_MONTH, 5),
        (ONE_YEAR, 6),
    ];

    let mut suite = SuiteBuilder::new()
        .with_initial_balances(astro_initial_balances)
        .with_timelock_config(timelock_config)
        .with_eclip_daily_reward(eclip_daily_reward)
        .with_locking_reward_config(locking_reward_config)
        .build();

    suite.update_config();

    // ready astro_staking_pool
    suite
        .stake_astro(astro_staking_initiator, 1_000_000)
        .unwrap();

    // Bob convert 1000 astro to eclipastro
    suite
        .convert_astro(&bob_address.clone().into_string(), 1_000)
        .unwrap();

    // change xastro/astro rate
    suite
        .send_astro(
            astro_staking_initiator,
            &suite.astro_staking_contract(),
            100_000,
        )
        .unwrap();

    assert_eq!(
        suite.query_astro_staking_total_deposit().unwrap(),
        1_101_000
    );
    assert_eq!(suite.query_astro_staking_total_shares().unwrap(), 1_001_000);
    assert_eq!(
        suite
            .query_xastro_balance(suite.voter_contract().as_str())
            .unwrap(),
        1_000
    );

    let reward = suite.query_converter_reward().unwrap();
    // total_reward = (1000 * 1_101_000 / 1_001_000 - 1000) * 1_001_000 / 1_101_000 = 90
    // user_reward = 90 * 0.8 * 1_001_000 / 1_101_000 = 79(72)
    // ce_holders_reward = 90 * 0.04 = 3
    // stability_pool_reward = 90 * 0.025 = 2
    // treasury_reward = 90 - 72 - 3 - 2 = 13
    assert_eq!(
        reward,
        RewardResponse {
            users_reward: Reward {
                token: suite.eclipastro_contract(),
                amount: Uint128::from(79u128)
            },
            ce_holders_reward: Reward {
                token: suite.xastro_contract(),
                amount: Uint128::from(3u128)
            },
            stability_pool_reward: Reward {
                token: suite.xastro_contract(),
                amount: Uint128::from(2u128)
            },
            treasury_reward: Reward {
                token: suite.xastro_contract(),
                amount: Uint128::from(13u128)
            },
        }
    );

    let err = suite.claim_treasury_reward(100).unwrap_err();
    assert_eq!(ContractError::NotEnoughBalance {}, err.downcast().unwrap());
    suite.claim_treasury_reward(10).unwrap();
    assert_eq!(
        suite
            .query_eclipastro_balance(&suite.reward_distributor_contract())
            .unwrap(),
        79
    );
    assert_eq!(
        suite
            .query_xastro_balance(&suite.eclipse_ce_reward_distributor())
            .unwrap(),
        3
    );
    assert_eq!(
        suite
            .query_xastro_balance(&suite.eclipse_stability_pool())
            .unwrap(),
        2
    );
    assert_eq!(
        suite
            .query_xastro_balance(&suite.eclipse_treasury())
            .unwrap(),
        10
    );

    let reward = suite.query_converter_reward().unwrap();
    assert_eq!(
        reward,
        RewardResponse {
            users_reward: Reward {
                token: suite.eclipastro_contract(),
                amount: Uint128::from(0u128)
            },
            ce_holders_reward: Reward {
                token: suite.xastro_contract(),
                amount: Uint128::from(0u128)
            },
            stability_pool_reward: Reward {
                token: suite.xastro_contract(),
                amount: Uint128::from(0u128)
            },
            treasury_reward: Reward {
                token: suite.xastro_contract(),
                amount: Uint128::from(3u128)
            },
        }
    );
    suite.claim_treasury_reward(3).unwrap();
    assert_eq!(
        suite
            .query_xastro_balance(&suite.eclipse_treasury())
            .unwrap(),
        13
    );
}

#[test]
fn claim() {
    let astro_staking_initiator = "astro_staking_initiator";
    let bob_address = Addr::unchecked(BOB);
    let astro_initial_balances = vec![
        (astro_staking_initiator, 2000_000),
        (ALICE, 1_000_000),
        (BOB, 1_000_000),
        (CAROL, 1_000_000),
    ];
    let timelock_config = vec![
        (ONE_MONTH, 100),
        (THREE_MONTH, 100),
        (SIX_MONTH, 100),
        (NINE_MONTH, 100),
        (ONE_YEAR, 100),
    ];
    let eclip_daily_reward = 1_000_000;
    let locking_reward_config = vec![
        (0, 1),
        (ONE_MONTH, 2),
        (THREE_MONTH, 3),
        (SIX_MONTH, 4),
        (NINE_MONTH, 5),
        (ONE_YEAR, 6),
    ];

    let mut suite = SuiteBuilder::new()
        .with_initial_balances(astro_initial_balances)
        .with_timelock_config(timelock_config)
        .with_eclip_daily_reward(eclip_daily_reward)
        .with_locking_reward_config(locking_reward_config)
        .build();

    suite.update_config();

    // ready astro_staking_pool
    suite
        .stake_astro(astro_staking_initiator, 1_000_000)
        .unwrap();

    // suite.send_astro(&bob_address.clone().into_string(), &suite.astro_staking_contract(), 100000u128).unwrap();
    // assert_eq!(suite.query_astro_staking_total_deposit().unwrap(), 1100000u128);
    // assert_eq!(suite.query_astro_staking_total_shares().unwrap(), 1000000u128);
    // let bob_eclipastro_balance = suite.query_eclipastro_balance(&bob_address.clone().into_string()).unwrap();
    // suite.stake_astro(&bob_address.clone().into_string(), 2_000).unwrap();
    // // Bob convert 1000 astro to eclipastro
    // suite
    //     .convert_xastro(&bob_address.clone().into_string(), 1_000)
    //     .unwrap();
    // let new_bob_eclipastro_balance = suite.query_eclipastro_balance(&bob_address.clone().into_string()).unwrap();
    // assert_eq!(new_bob_eclipastro_balance - bob_eclipastro_balance, 0u128);
    // Bob convert 1000 astro to eclipastro
    suite
        .convert_astro(&bob_address.clone().into_string(), 1_000)
        .unwrap();

    // change xastro/astro rate
    suite
        .send_astro(
            astro_staking_initiator,
            &suite.astro_staking_contract(),
            100_000,
        )
        .unwrap();

    suite.claim().unwrap();
    assert_eq!(
        suite
            .query_eclipastro_balance(&suite.reward_distributor_contract())
            .unwrap(),
        79
    );
    assert_eq!(
        suite
            .query_xastro_balance(&suite.eclipse_ce_reward_distributor())
            .unwrap(),
        3
    );
    assert_eq!(
        suite
            .query_xastro_balance(&suite.eclipse_stability_pool())
            .unwrap(),
        2
    );
    assert_eq!(
        suite
            .query_xastro_balance(&suite.eclipse_treasury())
            .unwrap(),
        0
    );
}

#[test]
fn withdraw_xtoken() {
    let astro_staking_initiator = "astro_staking_initiator";
    let bob_address = Addr::unchecked(BOB);
    let astro_initial_balances = vec![
        (astro_staking_initiator, 2000_000),
        (ALICE, 1_000_000),
        (BOB, 1_000_000),
        (CAROL, 1_000_000),
    ];
    let timelock_config = vec![
        (ONE_MONTH, 100),
        (THREE_MONTH, 100),
        (SIX_MONTH, 100),
        (NINE_MONTH, 100),
        (ONE_YEAR, 100),
    ];
    let eclip_daily_reward = 1_000_000;
    let locking_reward_config = vec![
        (0, 1),
        (ONE_MONTH, 2),
        (THREE_MONTH, 3),
        (SIX_MONTH, 4),
        (NINE_MONTH, 5),
        (ONE_YEAR, 6),
    ];

    let mut suite = SuiteBuilder::new()
        .with_initial_balances(astro_initial_balances)
        .with_timelock_config(timelock_config)
        .with_eclip_daily_reward(eclip_daily_reward)
        .with_locking_reward_config(locking_reward_config)
        .build();

    suite.update_config();

    // ready astro_staking_pool
    suite
        .stake_astro(astro_staking_initiator, 1_000_000)
        .unwrap();

    // Bob convert 1000 astro to eclipastro
    suite
        .convert_astro(&bob_address.clone().into_string(), 1_000)
        .unwrap();

    // change xastro/astro rate
    suite
        .send_astro(
            astro_staking_initiator,
            &suite.astro_staking_contract(),
            100_000,
        )
        .unwrap();

    assert_eq!(suite.query_withdrawable_balance().unwrap(), 0);

    suite.claim().unwrap();
    assert_eq!(suite.query_withdrawable_balance().unwrap(), 72);
    let err = suite.withdraw_xtoken("hacker", 10, "hacker").unwrap_err();
    assert_eq!(
        ContractError::Admin(AdminError::NotAdmin {}),
        err.downcast().unwrap()
    );
    suite
        .withdraw_xtoken(&suite.admin(), 10, &suite.admin())
        .unwrap();
    assert_eq!(suite.query_xastro_balance(&suite.admin()).unwrap(), 10);
    let err = suite
        .withdraw_xtoken(&suite.admin(), 70, &suite.admin())
        .unwrap_err();
    assert_eq!(ContractError::NotEnoughBalance {}, err.downcast().unwrap());
    suite
        .withdraw_xtoken(&suite.admin(), 62, &suite.admin())
        .unwrap();
}
