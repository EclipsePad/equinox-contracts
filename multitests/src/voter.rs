use cosmwasm_std::Addr;
use cw_controllers::AdminError;
use eclipse_base::staking::state::ECLIP_MAINNET;
use equinox_msg::voter::{Config, UpdateConfig};
use voter::ContractError;

use crate::suite::SuiteBuilder;

const ONE_MONTH: u64 = 86400 * 30;
const THREE_MONTH: u64 = 86400 * 30 * 3;
const SIX_MONTH: u64 = 86400 * 30 * 6;
const NINE_MONTH: u64 = 86400 * 30 * 9;
const ONE_YEAR: u64 = 86400 * 30 * 12;

const ALICE: &str = "alice";
const BOB: &str = "bob";
const CAROL: &str = "carol";
const ATTACKER: &str = "attacker";

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

    let config = suite.query_voter_config().unwrap();
    assert_eq!(
        config,
        Config {
            base_token: Addr::unchecked(suite.astro_contract()),
            xtoken: Addr::unchecked(suite.xastro_contract()),
            vxtoken: Addr::unchecked(suite.vxastro_contract()),
            staking_contract: Addr::unchecked(suite.astro_staking_contract()),
            converter_contract: Addr::unchecked(suite.converter_contract()),
            gauge_contract: Addr::unchecked(""),
            astroport_gauge_contract: Addr::unchecked(""),
            astroport_voting_escrow_contract: Addr::unchecked(
                suite.astroport_voting_escrow_contract()
            ),
            astroport_generator_controller: Addr::unchecked(
                suite.astroport_generator_controller_contract()
            ),
            eclipsepad_staking_contract: Addr::unchecked(suite.eclipsepad_staking_contract())
        }
    );
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

    suite.update_config();

    let test_config = UpdateConfig {
        base_token: Some(Addr::unchecked("test").into_string()),
        xtoken: Some(Addr::unchecked("test").into_string()),
        vxtoken: Some(Addr::unchecked("test").into_string()),
        staking_contract: Some(Addr::unchecked("test").into_string()),
        converter_contract: Some(Addr::unchecked("test").into_string()),
        gauge_contract: Some(Addr::unchecked("test").into_string()),
        astroport_gauge_contract: Some(Addr::unchecked("test").into_string()),
        astroport_voting_escrow_contract: Some(Addr::unchecked("test").into_string()),
        astroport_generator_controller: Some(Addr::unchecked("test").to_string()),
        eclipsepad_staking_contract: Some(Addr::unchecked("test").into_string()),
    };

    // attacker
    let err = suite
        .update_voter_config(ATTACKER, test_config.clone())
        .unwrap_err();
    assert_eq!(
        ContractError::Admin(AdminError::NotAdmin {}),
        err.downcast().unwrap()
    );

    suite
        .update_voter_config(&suite.admin(), test_config)
        .unwrap();
    assert_eq!(
        suite.query_voter_config().unwrap(),
        Config {
            base_token: Addr::unchecked("test"),
            xtoken: Addr::unchecked("test"),
            vxtoken: Addr::unchecked("test"),
            staking_contract: Addr::unchecked("test"),
            converter_contract: Addr::unchecked("test"),
            gauge_contract: Addr::unchecked("test"),
            astroport_gauge_contract: Addr::unchecked("test"),
            astroport_voting_escrow_contract: Addr::unchecked("test"),
            astroport_generator_controller: Addr::unchecked("test"),
            eclipsepad_staking_contract: Addr::unchecked("test"),
        }
    );
}

#[test]
fn update_owner() {
    let astro_staking_initiator = "astro_staking_initiator";
    let astro_initial_balances = vec![
        (astro_staking_initiator, 1_000_000_000),
        (ALICE, 1_000_000),
        (BOB, 1_000_000),
        (CAROL, 1_000_000),
        (ATTACKER, 1_000),
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

    // attacker
    let err = suite.update_voter_owner(ATTACKER, ATTACKER).unwrap_err();
    assert_eq!(
        ContractError::Admin(AdminError::NotAdmin {}),
        err.downcast().unwrap()
    );

    suite.update_voter_owner(&suite.admin(), ALICE).unwrap();
    assert_eq!(suite.query_voter_owner().unwrap(), ALICE);
}

#[test]
fn stake() {
    let astro_staking_initiator = "astro_staking_initiator";
    let astro_initial_balances = vec![
        (astro_staking_initiator, 1_000_000_000),
        (ALICE, 1_000_000),
        (BOB, 1_000_000),
        (CAROL, 1_000_000),
        (ATTACKER, 1_000),
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

    suite
        .stake_astro(astro_staking_initiator, 1_000_000)
        .unwrap();

    suite.convert_astro(BOB, 1_000).unwrap();

    assert_eq!(
        suite.query_xastro_balance(&suite.voter_contract()).unwrap(),
        1_000
    );

    // check attacker
    let err = suite.voter_stake(ATTACKER, 1_000).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());
}

// withdraw function was checked on token_converter.

#[test]
fn swap_to_eclip_astro_default() {
    let astro_staking_initiator = "astro_staking_initiator";
    let astro_initial_balances = vec![
        (astro_staking_initiator, 1_000_000_000),
        (ALICE, 1_000_000),
        (BOB, 1_000_000),
        (CAROL, 1_000_000),
        (ATTACKER, 1_000),
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
    suite.stake_astro(astro_staking_initiator, 10_000).unwrap();

    let astro = &Addr::unchecked(suite.astro_contract());
    let xastro = &Addr::unchecked(suite.xastro_contract());

    suite.stake_astro(CAROL, 3_000).unwrap();

    let alice_astro = suite.query_astro_balance(ALICE).unwrap();
    let alice_xastro = suite.query_xastro_balance(ALICE).unwrap();
    let alice_eclip_astro = suite.query_eclipastro_balance(ALICE).unwrap();
    assert_eq!(alice_astro, 1_000_000);
    assert_eq!(alice_xastro, 0);
    assert_eq!(alice_eclip_astro, 0);

    let bob_astro = suite.query_astro_balance(BOB).unwrap();
    let bob_xastro = suite.query_xastro_balance(BOB).unwrap();
    let bob_eclip_astro = suite.query_eclipastro_balance(BOB).unwrap();
    assert_eq!(bob_astro, 1_000_000);
    assert_eq!(bob_xastro, 0);
    assert_eq!(bob_eclip_astro, 0);

    let carol_astro = suite.query_astro_balance(CAROL).unwrap();
    let carol_xastro = suite.query_xastro_balance(CAROL).unwrap();
    let carol_eclip_astro = suite.query_eclipastro_balance(CAROL).unwrap();
    assert_eq!(carol_astro, 997_000);
    assert_eq!(carol_xastro, 3_000);
    assert_eq!(carol_eclip_astro, 0);

    suite
        .voter_swap_to_eclip_astro(ALICE, 1_000, astro)
        .unwrap();
    suite.voter_swap_to_eclip_astro(BOB, 2_000, astro).unwrap();
    suite
        .voter_swap_to_eclip_astro(CAROL, 3_000, xastro)
        .unwrap();

    let alice_astro = suite.query_astro_balance(ALICE).unwrap();
    let alice_xastro = suite.query_xastro_balance(ALICE).unwrap();
    let alice_eclip_astro = suite.query_eclipastro_balance(ALICE).unwrap();
    assert_eq!(alice_astro, 999_000);
    assert_eq!(alice_xastro, 0);
    assert_eq!(alice_eclip_astro, 1_000);

    let bob_astro = suite.query_astro_balance(BOB).unwrap();
    let bob_xastro = suite.query_xastro_balance(BOB).unwrap();
    let bob_eclip_astro = suite.query_eclipastro_balance(BOB).unwrap();
    assert_eq!(bob_astro, 998_000);
    assert_eq!(bob_xastro, 0);
    assert_eq!(bob_eclip_astro, 2_000);

    let carol_astro = suite.query_astro_balance(CAROL).unwrap();
    let carol_xastro = suite.query_xastro_balance(CAROL).unwrap();
    let carol_eclip_astro = suite.query_eclipastro_balance(CAROL).unwrap();
    assert_eq!(carol_astro, 997_000);
    assert_eq!(carol_xastro, 0);
    assert_eq!(carol_eclip_astro, 3_000);
}

#[test]
fn voting_power_default() {
    let astro_staking_initiator = "astro_staking_initiator";
    let astro_initial_balances = vec![
        (astro_staking_initiator, 1_000_000_000),
        (ALICE, 1_000_000),
        (BOB, 1_000_000),
        (CAROL, 1_000_000),
        (ATTACKER, 1_000),
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
    suite.stake_astro(astro_staking_initiator, 10_000).unwrap();

    let astro = &Addr::unchecked(suite.astro_contract());

    suite
        .voter_swap_to_eclip_astro(CAROL, 1_000, astro)
        .unwrap();

    // stake and lock in eclipsepad staking
    for user in [ALICE, BOB] {
        suite
            .mint_native(user.to_string(), ECLIP_MAINNET.to_string(), 1_000)
            .unwrap();
    }

    suite
        .eclipsepad_staking_try_stake(ALICE, 1_000, ECLIP_MAINNET)
        .unwrap();
    suite
        .eclipsepad_staking_try_stake(BOB, 1_000, ECLIP_MAINNET)
        .unwrap();
    suite.eclipsepad_staking_try_lock(BOB, 1_000, 0).unwrap();

    // check essence after 2 months
    suite.update_time(2 * 30 * 24 * 3600);

    let alice_essence = suite.eclipsepad_staking_query_essence(ALICE).unwrap();
    let bob_essence = suite.eclipsepad_staking_query_essence(BOB).unwrap();
    let total_essence = suite.eclipsepad_staking_query_total_essence().unwrap();
    assert_eq!(alice_essence.essence.u128(), 164);
    assert_eq!(bob_essence.essence.u128(), 82);
    assert_eq!(total_essence.essence.u128(), 246);

    // check voting power
    let alice_voting_power = suite.voter_query_voting_power(ALICE).unwrap();
    let bob_voting_power = suite.voter_query_voting_power(BOB).unwrap();
    let voter_voting_power = suite
        .voter_query_voting_power(&suite.voter_contract())
        .unwrap();
    assert_eq!(alice_voting_power.u128(), 1_520);
    assert_eq!(bob_voting_power.u128(), 760);
    assert_eq!(voter_voting_power.u128(), 2_280);

    // voting power must decreasing over time
    suite.update_time(2 * 30 * 24 * 3600);

    let voter_voting_power = suite
        .voter_query_voting_power(&suite.voter_contract())
        .unwrap();
    assert_eq!(voter_voting_power.u128(), 2_064);
}
