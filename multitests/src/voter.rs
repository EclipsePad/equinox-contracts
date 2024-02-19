use cosmwasm_std::Addr;
use cw_controllers::AdminError;
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
