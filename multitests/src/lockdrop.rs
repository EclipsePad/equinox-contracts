use astroport::{
    asset::{Asset, AssetInfo},
    vesting::{VestingAccount, VestingSchedule, VestingSchedulePoint},
};
use cosmwasm_std::{Addr, Uint128};
use cw_controllers::AdminError;
use equinox_msg::lockdrop::UpdateConfigMsg;
use lockdrop::error::ContractError;

use crate::suite::{Suite, SuiteBuilder};

const ONE_MONTH: u64 = 86400 * 30;
const THREE_MONTH: u64 = 86400 * 30 * 3;
const SIX_MONTH: u64 = 86400 * 30 * 6;
const NINE_MONTH: u64 = 86400 * 30 * 9;
const ONE_YEAR: u64 = 86400 * 365;

const ALICE: &str = "alice";
const BOB: &str = "bob";
const CAROL: &str = "carol";
// const ATTACKER: &str = "attacker";
// const VICTIM: &str = "victim";

fn instantiate() -> Suite {
    let astroport_organizer = "astroport_organizer";
    let astro_initial_balances = vec![
        (astroport_organizer, 2_000_000_000),
        (ALICE, 1_000_000),
        (BOB, 1_000_000),
        (CAROL, 1_000_000),
    ];
    let timelock_config = vec![
        (ONE_MONTH, 5000),
        (THREE_MONTH, 5000),
        (SIX_MONTH, 5000),
        (NINE_MONTH, 5000),
        (ONE_YEAR, 5000),
    ];
    let eclip_daily_reward = 100_000u128;
    let locking_reward_config = vec![
        (0, 1),
        (ONE_MONTH, 2),
        (THREE_MONTH, 3),
        (SIX_MONTH, 4),
        (NINE_MONTH, 5),
        (ONE_YEAR, 6),
    ];
    let lockdrop_reward_config = vec![
        (0, 1, 5000),
        (ONE_MONTH, 2, 5000),
        (THREE_MONTH, 3, 5000),
        (SIX_MONTH, 4, 5000),
        (NINE_MONTH, 5, 5000),
        (ONE_YEAR, 6, 5000),
    ];

    let mut suite = SuiteBuilder::new()
        .with_initial_balances(astro_initial_balances)
        .with_timelock_config(timelock_config)
        .with_eclip_daily_reward(eclip_daily_reward)
        .with_lp_staking_eclip_daily_reward(eclip_daily_reward)
        .with_locking_reward_config(locking_reward_config.clone())
        .with_lock_configs(lockdrop_reward_config)
        .build();

    suite.update_config();

    // ready ASTRO staking pool
    suite.stake_astro(astroport_organizer, 1_000_000).unwrap();

    // change ASTRO/xASTRO rate 1.1:1
    suite
        .send_astro(
            astroport_organizer,
            &suite.astro_staking_contract(),
            100_000,
        )
        .unwrap();
    suite.stake_astro(astroport_organizer, 10_000).unwrap();

    // convert ASTRO to eclipASTRO
    suite
        .convert_astro(&astroport_organizer, 1_100_000)
        .unwrap();

    // provide liquidity
    suite
        .provide_liquidity(
            &astroport_organizer,
            Addr::unchecked(suite.eclipastro_xastro_lp_contract()),
            vec![
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: Addr::unchecked(suite.eclipastro_contract()),
                    },
                    amount: Uint128::from(1_100_000u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: Addr::unchecked(suite.xastro_contract()),
                    },
                    amount: Uint128::from(1_000_000u128),
                },
            ],
            None,
            None,
        )
        .unwrap();

    // setup ASTRO incentives
    suite
        .setup_pools(
            &suite.admin(),
            vec![(
                suite.eclipastro_xastro_lp_token_contract(),
                Uint128::from(100u128),
            )],
        )
        .unwrap();

    suite
        .generator_set_tokens_per_second(&suite.admin(), 10u128)
        .unwrap();

    suite
        .send_astro(astroport_organizer, &suite.admin(), 1_000_000_000u128)
        .unwrap();
    let start_time = suite.get_time();
    let end_time = suite.get_time() + 86400 * 1000;
    suite
        .register_vesting_accounts(
            &suite.admin(),
            vec![VestingAccount {
                address: suite.astroport_generator_contract(),
                schedules: vec![VestingSchedule {
                    start_point: VestingSchedulePoint {
                        time: start_time,
                        amount: Uint128::zero(),
                    },
                    end_point: Some(VestingSchedulePoint {
                        time: end_time,
                        amount: Uint128::from(1_000_000_000u128),
                    }),
                }],
            }],
            1_000_000_000u128,
        )
        .unwrap();

    suite
}

#[test]
fn update_config() {
    let mut suite = instantiate();
    let new_config = UpdateConfigMsg {
        flexible_staking: Some(suite.flexible_staking_contract()),
        timelock_staking: Some(suite.timelock_staking_contract()),
        lp_staking: Some(suite.lp_staking()),
        reward_distributor: Some(suite.reward_distributor_contract()),
        dao_treasury_address: Some(Addr::unchecked("dao_treasury_address").to_string()),
    };
    suite
        .update_lockdrop_config(&suite.admin(), new_config)
        .unwrap();

    let res = suite.query_lockdrop_config().unwrap();
    assert_eq!(
        res.flexible_staking,
        Some(Addr::unchecked(suite.flexible_staking_contract()))
    );
    assert_eq!(
        res.timelock_staking,
        Some(Addr::unchecked(suite.timelock_staking_contract()))
    );
    assert_eq!(res.lp_staking, Some(Addr::unchecked(suite.lp_staking())));
    assert_eq!(
        res.reward_distributor,
        Some(Addr::unchecked(suite.reward_distributor_contract()))
    );
}

#[test]
fn handle_lockdrop() {
    let mut suite = instantiate();

    // lockdrop will fail as lockdrop is not started
    let err = suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 100u128, 0)
        .unwrap_err();
    assert_eq!(
        ContractError::DepositWindowNotStarted {},
        err.downcast().unwrap(),
    );

    // update time and test single stake with all duration(invalid duration check, withdraw flag is false)
    suite.update_time(86400u64 * 2);
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info[0].duration, 0u64);
    let total_shares = suite.query_astro_staking_total_shares().unwrap();
    let total_deposit = suite.query_astro_staking_total_deposit().unwrap();
    assert_eq!(
        single_lockup_info[0].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares / total_deposit
    );
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info[0].duration, 0u64);
    assert_eq!(
        alice_single_lockup_info[0].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares / total_deposit
    );

    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    let lp_lockup_info = suite.query_lp_lockup_info().unwrap();
    assert_eq!(lp_lockup_info[0].duration, 0u64);
    assert_eq!(
        lp_lockup_info[0].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares / total_deposit
    );
    let alice_lp_lockup_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(alice_lp_lockup_info[0].duration, 0u64);
    assert_eq!(
        alice_lp_lockup_info[0].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares / total_deposit
    );

    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info[0].duration, 0u64);
    assert_eq!(
        single_lockup_info[0].xastro_amount_in_lockups.u128(),
        2_000u128 * total_shares / total_deposit
    );
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info[0].duration, 0u64);
    assert_eq!(
        alice_single_lockup_info[0].xastro_amount_in_lockups.u128(),
        2_000u128 * total_shares / total_deposit
    );

    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    let lp_lockup_info = suite.query_lp_lockup_info().unwrap();
    assert_eq!(lp_lockup_info[0].duration, 0u64);
    assert_eq!(
        lp_lockup_info[0].xastro_amount_in_lockups.u128(),
        2_000u128 * total_shares / total_deposit
    );
    let alice_lp_lockup_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(alice_lp_lockup_info[0].duration, 0u64);
    assert_eq!(
        alice_lp_lockup_info[0].xastro_amount_in_lockups.u128(),
        2_000u128 * total_shares / total_deposit
    );

    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, ONE_MONTH)
        .unwrap();
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info[1].duration, ONE_MONTH);
    assert_eq!(
        single_lockup_info[1].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares / total_deposit
    );
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info[1].duration, ONE_MONTH);
    assert_eq!(
        alice_single_lockup_info[1].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares / total_deposit
    );

    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, THREE_MONTH)
        .unwrap();
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info[2].duration, THREE_MONTH);
    assert_eq!(
        single_lockup_info[2].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares / total_deposit
    );
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info[2].duration, THREE_MONTH);
    assert_eq!(
        alice_single_lockup_info[2].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares / total_deposit
    );

    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, SIX_MONTH)
        .unwrap();
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info[3].duration, SIX_MONTH);
    assert_eq!(
        single_lockup_info[3].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares / total_deposit
    );
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info[3].duration, SIX_MONTH);
    assert_eq!(
        alice_single_lockup_info[3].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares / total_deposit
    );

    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, ONE_YEAR)
        .unwrap();
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info[4].duration, ONE_YEAR);
    assert_eq!(
        single_lockup_info[4].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares / total_deposit
    );
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info[4].duration, ONE_YEAR);
    assert_eq!(
        alice_single_lockup_info[4].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares / total_deposit
    );

    suite.stake_astro(ALICE, 4_000u128).unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.xastro_contract(), 1_000u128, ONE_MONTH)
        .unwrap();
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info[1].duration, ONE_MONTH);
    assert_eq!(
        single_lockup_info[1].xastro_amount_in_lockups.u128(),
        1_000u128 + 1_000u128 * total_shares / total_deposit
    );
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info[1].duration, ONE_MONTH);
    assert_eq!(
        alice_single_lockup_info[1].xastro_amount_in_lockups.u128(),
        1_000u128 + 1_000u128 * total_shares / total_deposit
    );

    suite
        .lp_staking_increase_lockdrop(ALICE, suite.xastro_contract(), 1_000u128, ONE_MONTH)
        .unwrap();
    let lp_lockup_info = suite.query_lp_lockup_info().unwrap();
    assert_eq!(lp_lockup_info[1].duration, ONE_MONTH);
    assert_eq!(lp_lockup_info[1].xastro_amount_in_lockups.u128(), 1_000u128);
    let alice_lp_lockup_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(alice_lp_lockup_info[1].duration, ONE_MONTH);
    assert_eq!(
        alice_lp_lockup_info[1].xastro_amount_in_lockups.u128(),
        1_000u128
    );

    // update time to withdraw window
    // deposit will fail, withdraw will only allow 50% and only once
    suite.update_time(86400u64 * 4 + 43200u64);
    let err = suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, ONE_MONTH)
        .unwrap_err();
    assert_eq!(
        ContractError::DepositWindowClosed {},
        err.downcast().unwrap()
    );

    let err = suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, ONE_MONTH)
        .unwrap_err();
    assert_eq!(
        ContractError::DepositWindowClosed {},
        err.downcast().unwrap()
    );
}

// extend lock is only allowed during deposit window
#[test]
fn extend_lock_single_sided_flexible() {
    let mut suite = instantiate();

    // start lockdrop deposit window(deposit window day 2)
    suite.update_time(86400u64 * 2);

    // alice deposits 1000 astro and extends from single sided flexible to 3 months with no deposit
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    suite
        .single_staking_extend_duration_without_deposit(ALICE, 0, 86400 * 30 * 3)
        .unwrap();
    let user_deposits = suite.query_user_single_lockup_info(ALICE).unwrap();
    // flexible deposit must be zero
    assert_eq!(user_deposits.iter().find(|d| { d.duration == 0 }), None);
    // 3 months deposit must be exist
    assert_eq!(
        user_deposits
            .iter()
            .find(|d| { d.duration == 7776000 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );

    // alice deposits 1000 astro and extends from single sided flexible to 3 months with 500 astro deposit
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    suite
        .single_staking_extend_duration_with_deposit(
            ALICE,
            suite.astro_contract(),
            500u128,
            0,
            86400 * 30 * 3,
        )
        .unwrap();
    let user_deposits = suite.query_user_single_lockup_info(ALICE).unwrap();
    // flexible deposit must be zero
    assert_eq!(user_deposits.iter().find(|d| { d.duration == 0 }), None);
    // 3 months deposit must be exist
    assert_eq!(
        user_deposits
            .iter()
            .find(|d| { d.duration == 7776000 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        2272u128
    );

    // alice deposits 1000 astro and extends from single sided flexible to flexible with no deposit
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    let err = suite
        .single_staking_extend_duration_without_deposit(ALICE, 0, 0)
        .unwrap_err();
    assert_eq!(
        ContractError::ExtendDurationErr(0, 0),
        err.downcast().unwrap()
    );

    // alice deposits 1000 astro and extends from single sided flexible to flexible with 500 deposit
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    let err = suite
        .single_staking_extend_duration_with_deposit(ALICE, suite.astro_contract(), 500u128, 0, 0)
        .unwrap_err();
    assert_eq!(
        ContractError::ExtendDurationErr(0, 0),
        err.downcast().unwrap()
    );

    // alice tries to extend invalid duration
    let err = suite
        .single_staking_extend_duration_without_deposit(ALICE, 0, 100)
        .unwrap_err();
    assert_eq!(ContractError::InvalidDuration(100), err.downcast().unwrap());

    //alice deposits 1000 astro and after deposit window, try to extend
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    suite.update_time(86400 * 5);
    let err = suite
        .single_staking_extend_duration_without_deposit(ALICE, 0, 7776000)
        .unwrap_err();
    assert_eq!(
        ContractError::DepositWindowClosed {},
        err.downcast().unwrap()
    );
    let err = suite
        .single_staking_extend_duration_with_deposit(
            ALICE,
            suite.astro_contract(),
            500u128,
            0,
            7776000,
        )
        .unwrap_err();
    assert_eq!(
        ContractError::DepositWindowClosed {},
        err.downcast().unwrap()
    );
}

#[test]
fn extend_lock_single_sided_timelocked() {
    let mut suite = instantiate();

    // start lockdrop deposit window(deposit window day 2)
    suite.update_time(86400u64 * 2);

    // alice deposits 1000 astro and extends from 1 month to 3 months with no deposit
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 2592000)
        .unwrap();
    suite
        .single_staking_extend_duration_without_deposit(ALICE, 2592000, 7776000)
        .unwrap();
    let user_deposits = suite.query_user_single_lockup_info(ALICE).unwrap();
    // 1 month deposit must be zero
    assert_eq!(
        user_deposits.iter().find(|d| { d.duration == 2592000 }),
        None
    );
    // 3 months deposit must be exist
    assert_eq!(
        user_deposits
            .iter()
            .find(|d| { d.duration == 7776000 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );

    // alice deposits 1000 astro and extends from single sided 1 month to 3 months with 500 astro deposit
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 2592000)
        .unwrap();
    suite
        .single_staking_extend_duration_with_deposit(
            ALICE,
            suite.astro_contract(),
            500u128,
            2592000,
            7776000,
        )
        .unwrap();
    let user_deposits = suite.query_user_single_lockup_info(ALICE).unwrap();
    // flexible deposit must be zero
    assert_eq!(
        user_deposits.iter().find(|d| { d.duration == 2592000 }),
        None
    );
    // 3 months deposit must be exist
    assert_eq!(
        user_deposits
            .iter()
            .find(|d| { d.duration == 7776000 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        2272u128
    );

    // alice deposits 1000 astro and extends from single sided 1 month to 1 month with no deposit
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 2592000)
        .unwrap();
    let err = suite
        .single_staking_extend_duration_without_deposit(ALICE, 2592000, 2592000)
        .unwrap_err();
    assert_eq!(
        ContractError::ExtendDurationErr(2592000, 2592000),
        err.downcast().unwrap()
    );

    // alice deposits 1000 astro and extends from single sided 1 month to 1 month with 500 deposit
    let err = suite
        .single_staking_extend_duration_with_deposit(
            ALICE,
            suite.astro_contract(),
            500u128,
            2592000,
            2592000,
        )
        .unwrap_err();
    assert_eq!(
        ContractError::ExtendDurationErr(2592000, 2592000),
        err.downcast().unwrap()
    );

    // alice tries to extend invalid duration
    let err = suite
        .single_staking_extend_duration_without_deposit(ALICE, 2592000, 3000000)
        .unwrap_err();
    assert_eq!(
        ContractError::InvalidDuration(3000000),
        err.downcast().unwrap()
    );

    //alice deposits 1000 astro and after deposit window, try to extend
    suite.update_time(86400 * 5);
    let err = suite
        .single_staking_extend_duration_without_deposit(ALICE, 2592000, 7776000)
        .unwrap_err();
    assert_eq!(
        ContractError::DepositWindowClosed {},
        err.downcast().unwrap()
    );
    let err = suite
        .single_staking_extend_duration_with_deposit(
            ALICE,
            suite.astro_contract(),
            500u128,
            2592000,
            7776000,
        )
        .unwrap_err();
    assert_eq!(
        ContractError::DepositWindowClosed {},
        err.downcast().unwrap()
    );
}

#[test]
fn extend_lock_lp_flexible() {
    let mut suite = instantiate();

    // start lockdrop deposit window(deposit window day 2)
    suite.update_time(86400u64 * 2);

    // alice deposits 1000 astro and extends from single sided flexible to 3 months with no deposit
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    suite
        .lp_lockup_extend_duration_without_deposit(ALICE, 0, 86400 * 30 * 3)
        .unwrap();
    let user_deposits = suite.query_user_lp_lockup_info(ALICE).unwrap();
    // flexible deposit must be zero
    assert_eq!(user_deposits.iter().find(|d| { d.duration == 0 }), None);
    // 3 months deposit must be exist
    assert_eq!(
        user_deposits
            .iter()
            .find(|d| { d.duration == 7776000 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );

    // alice deposits 1000 astro and extends from single sided flexible to 3 months with 500 astro deposit
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    suite
        .lp_lockup_extend_duration_with_deposit(
            ALICE,
            suite.astro_contract(),
            500u128,
            0,
            86400 * 30 * 3,
        )
        .unwrap();
    let user_deposits = suite.query_user_lp_lockup_info(ALICE).unwrap();
    // flexible deposit must be zero
    assert_eq!(user_deposits.iter().find(|d| { d.duration == 0 }), None);
    // 3 months deposit must be exist
    assert_eq!(
        user_deposits
            .iter()
            .find(|d| { d.duration == 7776000 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        2272u128
    );

    // alice deposits 1000 astro and extends from single sided flexible to flexible with no deposit
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    let err = suite
        .lp_lockup_extend_duration_without_deposit(ALICE, 0, 0)
        .unwrap_err();
    assert_eq!(
        ContractError::ExtendDurationErr(0, 0),
        err.downcast().unwrap()
    );

    // alice deposits 1000 astro and extends from single sided flexible to flexible with 500 deposit
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    let err = suite
        .lp_lockup_extend_duration_with_deposit(ALICE, suite.astro_contract(), 500u128, 0, 0)
        .unwrap_err();
    assert_eq!(
        ContractError::ExtendDurationErr(0, 0),
        err.downcast().unwrap()
    );

    // alice tries to extend invalid duration
    let err = suite
        .lp_lockup_extend_duration_without_deposit(ALICE, 0, 100)
        .unwrap_err();
    assert_eq!(ContractError::InvalidDuration(100), err.downcast().unwrap());

    //alice deposits 1000 astro and after deposit window, try to extend
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    suite.update_time(86400 * 5);
    let err = suite
        .lp_lockup_extend_duration_without_deposit(ALICE, 0, 7776000)
        .unwrap_err();
    assert_eq!(
        ContractError::DepositWindowClosed {},
        err.downcast().unwrap()
    );
    let err = suite
        .lp_lockup_extend_duration_with_deposit(ALICE, suite.astro_contract(), 500u128, 0, 7776000)
        .unwrap_err();
    assert_eq!(
        ContractError::DepositWindowClosed {},
        err.downcast().unwrap()
    );
}

#[test]
fn single_sided_withdraw() {
    let mut suite = instantiate();

    // start lockdrop deposit window(deposit window day 2)
    suite.update_time(86400u64 * 2);

    // alice deposits 1000 astro and withdraw it
    let old_alice_xastro_balance = suite.query_xastro_balance(ALICE).unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 2592000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 7776000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 23328000)
        .unwrap();
    // start withdraw window(withdraw window started)
    suite.update_time(86400u64 * 4);
    // if withdraw amount is greater than max withdrawal amount, only withdraws max withdrawal amount
    // let err = suite
    //     .single_staking_lockdrop_withdraw(ALICE, Some(Uint128::from(1000u128)), 0)
    //     .unwrap_err();
    // assert_eq!(
    //     ContractError::WithdrawLimitExceed(454u128.to_string()),
    //     err.downcast().unwrap()
    // );
    // alice withdraws 300
    suite
        .single_staking_lockdrop_withdraw(ALICE, Some(Uint128::from(300u128)), 0)
        .unwrap();
    let alice_xastro_balance = suite.query_xastro_balance(ALICE).unwrap();
    assert_eq!(alice_xastro_balance - old_alice_xastro_balance, 300u128);
    // alice tries again, and it is failed
    let err = suite
        .single_staking_lockdrop_withdraw(ALICE, Some(Uint128::from(100u128)), 0)
        .unwrap_err();
    assert_eq!(ContractError::AlreadyWithdrawed {}, err.downcast().unwrap());
    let user_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        609u128
    );
    // alice withdraws maximum without setting amount
    let old_alice_xastro_balance = suite.query_xastro_balance(ALICE).unwrap();
    suite
        .single_staking_lockdrop_withdraw(ALICE, None, 2592000)
        .unwrap();
    let alice_xastro_balance = suite.query_xastro_balance(ALICE).unwrap();
    assert_eq!(alice_xastro_balance - old_alice_xastro_balance, 454u128);
    let user_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2592000u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        455u128
    );

    // withdraw window passed 3/4 checking decrease
    suite.update_time(86400u64 + 43200u64);
    let old_alice_xastro_balance = suite.query_xastro_balance(ALICE).unwrap();
    suite
        .single_staking_lockdrop_withdraw(ALICE, None, 7776000)
        .unwrap();
    let alice_xastro_balance = suite.query_xastro_balance(ALICE).unwrap();
    assert_eq!(alice_xastro_balance - old_alice_xastro_balance, 227u128);
    let user_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 7776000u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        682u128
    );

    // withdraw ended
    suite.update_time(86400u64);
    let err = suite
        .single_staking_lockdrop_withdraw(ALICE, None, 23328000)
        .unwrap_err();
    assert_eq!(ContractError::LockdropFinished {}, err.downcast().unwrap());
}

#[test]
fn lp_withdraw() {
    let mut suite = instantiate();

    // start lockdrop deposit window(deposit window day 2)
    suite.update_time(86400u64 * 2);

    // alice deposits 1000 astro and withdraw it
    let old_alice_xastro_balance = suite.query_xastro_balance(ALICE).unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 2592000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 7776000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 23328000)
        .unwrap();
    // start withdraw window(withdraw window started)
    suite.update_time(86400u64 * 4);
    let err = suite
        .lp_staking_lockdrop_withdraw(ALICE, Some(Uint128::from(1000u128)), 0)
        .unwrap_err();
    assert_eq!(
        ContractError::WithdrawLimitExceed(454u128.to_string()),
        err.downcast().unwrap()
    );
    // alice withdraws 300
    suite
        .lp_staking_lockdrop_withdraw(ALICE, Some(Uint128::from(300u128)), 0)
        .unwrap();
    let alice_xastro_balance = suite.query_xastro_balance(ALICE).unwrap();
    assert_eq!(alice_xastro_balance - old_alice_xastro_balance, 300u128);
    // alice tries again, and it is failed
    let err = suite
        .lp_staking_lockdrop_withdraw(ALICE, Some(Uint128::from(100u128)), 0)
        .unwrap_err();
    assert_eq!(ContractError::AlreadyWithdrawed {}, err.downcast().unwrap());
    let user_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        609u128
    );
    // alice withdraws maximum without setting amount
    let old_alice_xastro_balance = suite.query_xastro_balance(ALICE).unwrap();
    suite
        .lp_staking_lockdrop_withdraw(ALICE, None, 2592000)
        .unwrap();
    let alice_xastro_balance = suite.query_xastro_balance(ALICE).unwrap();
    assert_eq!(alice_xastro_balance - old_alice_xastro_balance, 454u128);
    let user_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2592000u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        455u128
    );

    // withdraw window passed 3/4 checking decrease
    suite.update_time(86400u64 + 43200u64);
    let old_alice_xastro_balance = suite.query_xastro_balance(ALICE).unwrap();
    suite
        .lp_staking_lockdrop_withdraw(ALICE, None, 7776000)
        .unwrap();
    let alice_xastro_balance = suite.query_xastro_balance(ALICE).unwrap();
    assert_eq!(alice_xastro_balance - old_alice_xastro_balance, 227u128);
    let user_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 7776000u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        682u128
    );

    // withdraw ended
    suite.update_time(86400u64);
    let err = suite
        .lp_staking_lockdrop_withdraw(ALICE, None, 23328000)
        .unwrap_err();
    assert_eq!(ContractError::LockdropFinished {}, err.downcast().unwrap());
}

// test funding
#[test]
fn fund_incentives() {
    let mut suite = instantiate();

    suite
        .mint_native(ALICE.to_string(), suite.eclip(), 1_000_000_000)
        .unwrap();
    suite
        .mint_native(suite.admin(), suite.eclip(), 1_000_000_000)
        .unwrap();
    let err = suite
        .increase_eclip_incentives_lockdrop(ALICE, 1_000_000u128)
        .unwrap_err();
    assert_eq!(
        ContractError::Admin(AdminError::NotAdmin {}),
        err.downcast().unwrap()
    );
    // test increase incentives before deposit window
    suite
        .increase_eclip_incentives_lockdrop(&suite.admin(), 1_000_000u128)
        .unwrap();

    // test increase incentives on deposit window
    suite.update_time(86400u64 * 2);
    suite
        .increase_eclip_incentives_lockdrop(&suite.admin(), 1_000_000u128)
        .unwrap();

    // test increase incentives on withdraw window
    suite.update_time(86400u64 * 5);
    let err = suite
        .increase_eclip_incentives_lockdrop(&suite.admin(), 1_000_000u128)
        .unwrap_err();
    assert_eq!(
        ContractError::DepositWindowClosed {},
        err.downcast().unwrap()
    );
}

// test staking assets to vaults
#[test]
fn stake_assets_to_vaults() {
    let mut suite = instantiate();

    suite
        .update_lockdrop_config(
            &suite.admin(),
            UpdateConfigMsg {
                flexible_staking: Some(suite.flexible_staking_contract()),
                timelock_staking: Some(suite.timelock_staking_contract()),
                lp_staking: Some(suite.lp_staking()),
                reward_distributor: Some(suite.reward_distributor_contract()),
                dao_treasury_address: Some(Addr::unchecked("dao_treasury_address").to_string()),
            },
        )
        .unwrap();

    suite.update_time(86400u64 * 2);
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 2592000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 7776000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 23328000)
        .unwrap();

    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 2592000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 7776000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 23328000)
        .unwrap();

    // test on deposit window
    let err = suite.lockdrop_stake_to_vaults(ALICE).unwrap_err();
    assert_eq!(
        ContractError::Admin(AdminError::NotAdmin {}),
        err.downcast().unwrap()
    );
    let err = suite.lockdrop_stake_to_vaults(&suite.admin()).unwrap_err();
    assert_eq!(
        ContractError::LockdropNotFinished {},
        err.downcast().unwrap()
    );

    // test on withdraw window
    suite.update_time(86400u64 * 5);
    let err = suite.lockdrop_stake_to_vaults(&suite.admin()).unwrap_err();
    assert_eq!(
        ContractError::LockdropNotFinished {},
        err.downcast().unwrap()
    );

    // test after lockdrop finished
    suite.update_time(86400u64 * 2);
    suite.lockdrop_stake_to_vaults(&suite.admin()).unwrap();
    let single_state = suite.query_single_lockup_state().unwrap();
    let single_info = suite.query_single_lockup_info().unwrap();

    assert_eq!(
        single_info
            .iter()
            .find(|i| { i.duration == 0u64 })
            .unwrap()
            .total_staked
            .u128(),
        999u128
    );
    assert_eq!(
        single_info
            .iter()
            .find(|i| { i.duration == 2592000u64 })
            .unwrap()
            .total_staked
            .u128(),
        999u128
    );
    assert_eq!(
        single_info
            .iter()
            .find(|i| { i.duration == 7776000u64 })
            .unwrap()
            .total_staked
            .u128(),
        999u128
    );
    assert_eq!(
        single_info
            .iter()
            .find(|i| { i.duration == 23328000u64 })
            .unwrap()
            .total_staked
            .u128(),
        999u128
    );

    assert_eq!(
        single_info
            .iter()
            .find(|i| { i.duration == 0u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );
    assert_eq!(
        single_info
            .iter()
            .find(|i| { i.duration == 2592000u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );
    assert_eq!(
        single_info
            .iter()
            .find(|i| { i.duration == 7776000u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );
    assert_eq!(
        single_info
            .iter()
            .find(|i| { i.duration == 23328000u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );

    assert_eq!(
        single_info
            .iter()
            .find(|i| { i.duration == 0u64 })
            .unwrap()
            .total_withdrawed
            .u128(),
        0u128
    );
    assert_eq!(
        single_info
            .iter()
            .find(|i| { i.duration == 2592000u64 })
            .unwrap()
            .total_withdrawed
            .u128(),
        0u128
    );
    assert_eq!(
        single_info
            .iter()
            .find(|i| { i.duration == 7776000u64 })
            .unwrap()
            .total_withdrawed
            .u128(),
        0u128
    );
    assert_eq!(
        single_info
            .iter()
            .find(|i| { i.duration == 23328000u64 })
            .unwrap()
            .total_withdrawed
            .u128(),
        0u128
    );

    assert_eq!(single_state.is_staked, true);
    assert_eq!(single_state.are_claims_allowed, true);
    assert_eq!(single_state.countdown_start_at, suite.get_time());
    assert_eq!(single_state.total_eclipastro_lockup.u128(), 3_999u128);

    let lp_state = suite.query_lp_lockup_state().unwrap();
    let lp_info = suite.query_lp_lockup_info().unwrap();

    assert_eq!(
        lp_info
            .iter()
            .find(|i| { i.duration == 0u64 })
            .unwrap()
            .total_staked
            .u128(),
        476u128
    );
    assert_eq!(
        lp_info
            .iter()
            .find(|i| { i.duration == 2592000u64 })
            .unwrap()
            .total_staked
            .u128(),
        476u128
    );
    assert_eq!(
        lp_info
            .iter()
            .find(|i| { i.duration == 7776000u64 })
            .unwrap()
            .total_staked
            .u128(),
        476u128
    );
    assert_eq!(
        lp_info
            .iter()
            .find(|i| { i.duration == 23328000u64 })
            .unwrap()
            .total_staked
            .u128(),
        476u128
    );

    assert_eq!(
        lp_info
            .iter()
            .find(|i| { i.duration == 0u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );
    assert_eq!(
        lp_info
            .iter()
            .find(|i| { i.duration == 2592000u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );
    assert_eq!(
        lp_info
            .iter()
            .find(|i| { i.duration == 7776000u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );
    assert_eq!(
        lp_info
            .iter()
            .find(|i| { i.duration == 23328000u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );

    assert_eq!(
        lp_info
            .iter()
            .find(|i| { i.duration == 0u64 })
            .unwrap()
            .total_withdrawed
            .u128(),
        0u128
    );
    assert_eq!(
        lp_info
            .iter()
            .find(|i| { i.duration == 2592000u64 })
            .unwrap()
            .total_withdrawed
            .u128(),
        0u128
    );
    assert_eq!(
        lp_info
            .iter()
            .find(|i| { i.duration == 7776000u64 })
            .unwrap()
            .total_withdrawed
            .u128(),
        0u128
    );
    assert_eq!(
        lp_info
            .iter()
            .find(|i| { i.duration == 23328000u64 })
            .unwrap()
            .total_withdrawed
            .u128(),
        0u128
    );
    assert_eq!(lp_state.is_staked, true);
    assert_eq!(lp_state.are_claims_allowed, true);
    assert_eq!(lp_state.countdown_start_at, suite.get_time());
}

// test distribution of the eclip incentives
#[test]
fn single_sided_incentives_distribution() {
    let mut suite = instantiate();

    suite
        .update_lockdrop_config(
            &suite.admin(),
            UpdateConfigMsg {
                flexible_staking: Some(suite.flexible_staking_contract()),
                timelock_staking: Some(suite.timelock_staking_contract()),
                lp_staking: Some(suite.lp_staking()),
                reward_distributor: Some(suite.reward_distributor_contract()),
                dao_treasury_address: Some(Addr::unchecked("dao_treasury_address").to_string()),
            },
        )
        .unwrap();

    suite.update_time(86400u64 * 2);

    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 2592000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 7776000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 23328000)
        .unwrap();

    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 2592000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 7776000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 23328000)
        .unwrap();

    suite
        .mint_native(suite.admin(), suite.eclip(), 1_000_000_000)
        .unwrap();
    suite
        .increase_eclip_incentives_lockdrop(&suite.admin(), 1_000_000u128)
        .unwrap();
    suite
        .increase_eclip_incentives_lockdrop(&suite.admin(), 1_000_000u128)
        .unwrap();
    // withdraw window finished
    suite.update_time(86400u64 * 7);
    let cfg = suite.query_lockdrop_config().unwrap();
    assert!(cfg.init_timestamp + cfg.deposit_window + cfg.withdrawal_window < suite.get_time());

    // stake assets to single sided vaults and lp vault
    suite.lockdrop_stake_to_vaults(&suite.admin()).unwrap();

    // fund eclip to staking vaults daily reward is 1_000_000_000u128
    suite
        .mint_native(
            suite.reward_distributor_contract(),
            suite.eclip(),
            100_000_000_000u128,
        )
        .unwrap();
    suite
        .mint_native(suite.lp_staking(), suite.eclip(), 100_000_000_000u128)
        .unwrap();
    // test eclip incentives
    let prev_alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    let user_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .total_eclip_incentives
            .u128(),
        90_909u128
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2_592_000 })
            .unwrap()
            .total_eclip_incentives
            .u128(),
        181_818u128
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 7_776_000 })
            .unwrap()
            .total_eclip_incentives
            .u128(),
        272_727u128
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 23_328_000 })
            .unwrap()
            .total_eclip_incentives
            .u128(),
        454_545u128
    );
    suite
        .single_lockdrop_claim_rewards_and_optionally_unlock(ALICE, 0, None)
        .unwrap();
    let alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    assert_eq!(alice_eclip_balance - prev_alice_eclip_balance, 27_272u128); // 30%
    let user_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .claimed_eclip_incentives
            .u128(),
        27_272u128
    );
    suite
        .single_lockdrop_claim_rewards_and_optionally_unlock(ALICE, 2592000, None)
        .unwrap();
    let alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    assert_eq!(alice_eclip_balance - prev_alice_eclip_balance, 81817u128); // 30%
    let user_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2592000 })
            .unwrap()
            .claimed_eclip_incentives
            .u128(),
        54545u128
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2592000 })
            .unwrap()
            .pending_eclip_incentives
            .u128(),
        0u128
    );

    // update time 1 day
    suite.update_time(86400u64);
    let user_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .claimed_eclip_incentives
            .u128(),
        27_272u128
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .pending_eclip_incentives
            .u128(),
        707u128
    );
    let prev_alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    suite
        .single_lockdrop_claim_rewards_and_optionally_unlock(ALICE, 0, None)
        .unwrap();
    let alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    assert_eq!(alice_eclip_balance - prev_alice_eclip_balance, 9797u128); // 9090 + 707

    suite
        .single_lockdrop_claim_rewards_and_optionally_unlock(ALICE, 2592000, None)
        .unwrap();
    let alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    assert_eq!(alice_eclip_balance - prev_alice_eclip_balance, 29391u128);

    // update time 1 day
    suite.update_time(86400u64);
    let user_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .claimed_eclip_incentives
            .u128(),
        27_979u128
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .pending_eclip_incentives
            .u128(),
        707u128
    );
    let prev_alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    suite
        .single_lockdrop_claim_rewards_and_optionally_unlock(ALICE, 0, None)
        .unwrap();
    let alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    assert_eq!(alice_eclip_balance - prev_alice_eclip_balance, 9797u128); // 9090 + 707
}

#[test]
fn lp_incentives_distribution() {
    let mut suite = instantiate();

    suite
        .update_lockdrop_config(
            &suite.admin(),
            UpdateConfigMsg {
                flexible_staking: Some(suite.flexible_staking_contract()),
                timelock_staking: Some(suite.timelock_staking_contract()),
                lp_staking: Some(suite.lp_staking()),
                reward_distributor: Some(suite.reward_distributor_contract()),
                dao_treasury_address: Some(Addr::unchecked("dao_treasury_address").to_string()),
            },
        )
        .unwrap();

    suite.update_time(86400u64 * 2);

    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 2592000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 7776000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 23328000)
        .unwrap();

    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 2592000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 7776000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 23328000)
        .unwrap();

    suite
        .mint_native(suite.admin(), suite.eclip(), 1_000_000_000)
        .unwrap();
    suite
        .increase_eclip_incentives_lockdrop(&suite.admin(), 1_000_000u128)
        .unwrap();
    suite
        .increase_eclip_incentives_lockdrop(&suite.admin(), 1_000_000u128)
        .unwrap();
    // withdraw window finished
    suite.update_time(86400u64 * 7);
    let cfg = suite.query_lockdrop_config().unwrap();
    assert!(cfg.init_timestamp + cfg.deposit_window + cfg.withdrawal_window < suite.get_time());

    // stake assets to single sided vaults and lp vault
    suite.lockdrop_stake_to_vaults(&suite.admin()).unwrap();

    // fund eclip to staking vaults daily reward is 1_000_000_000u128
    suite
        .mint_native(
            suite.reward_distributor_contract(),
            suite.eclip(),
            100_000_000_000u128,
        )
        .unwrap();
    suite
        .mint_native(suite.lp_staking(), suite.eclip(), 100_000_000_000u128)
        .unwrap();
    // test eclip incentives
    let prev_alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    let user_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .total_eclip_incentives
            .u128(),
        90_909u128
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2_592_000 })
            .unwrap()
            .total_eclip_incentives
            .u128(),
        181_818u128
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 7_776_000 })
            .unwrap()
            .total_eclip_incentives
            .u128(),
        272_727u128
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 23_328_000 })
            .unwrap()
            .total_eclip_incentives
            .u128(),
        454_545u128
    );
    suite
        .lp_lockdrop_claim_rewards_and_optionally_unlock(ALICE, 0, None)
        .unwrap();
    let alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    assert_eq!(alice_eclip_balance - prev_alice_eclip_balance, 27_272u128); // 30%
    let user_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .claimed_eclip_incentives
            .u128(),
        27_272u128
    );
    suite
        .lp_lockdrop_claim_rewards_and_optionally_unlock(ALICE, 2592000, None)
        .unwrap();
    let alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    assert_eq!(alice_eclip_balance - prev_alice_eclip_balance, 81817u128); // 30%
    let user_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2592000 })
            .unwrap()
            .claimed_eclip_incentives
            .u128(),
        54545u128
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2592000 })
            .unwrap()
            .pending_eclip_incentives
            .u128(),
        0u128
    );

    // update time 1 day
    suite.update_time(86400u64);
    let user_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .claimed_eclip_incentives
            .u128(),
        27_272u128
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .pending_eclip_incentives
            .u128(),
        707u128
    );
    let prev_alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    suite
        .lp_lockdrop_claim_rewards_and_optionally_unlock(ALICE, 0, None)
        .unwrap();
    let alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    assert_eq!(alice_eclip_balance - prev_alice_eclip_balance, 25706u128); // 24999 + 707

    suite
        .lp_lockdrop_claim_rewards_and_optionally_unlock(ALICE, 2592000, None)
        .unwrap();
    let alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    assert_eq!(alice_eclip_balance - prev_alice_eclip_balance, 52119u128); // 24999 + 1414

    // update time 1 day
    suite.update_time(86400u64);
    let user_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .claimed_eclip_incentives
            .u128(),
        27_979u128
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .pending_eclip_incentives
            .u128(),
        707u128
    );
    let prev_alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    suite
        .lp_lockdrop_claim_rewards_and_optionally_unlock(ALICE, 0, None)
        .unwrap();
    let alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    assert_eq!(alice_eclip_balance - prev_alice_eclip_balance, 25706u128); // 24999 + 707
}

#[test]
fn restake_and_unlock() {
    let mut suite = instantiate();

    suite
        .update_lockdrop_config(
            &suite.admin(),
            UpdateConfigMsg {
                flexible_staking: Some(suite.flexible_staking_contract()),
                timelock_staking: Some(suite.timelock_staking_contract()),
                lp_staking: Some(suite.lp_staking()),
                reward_distributor: Some(suite.reward_distributor_contract()),
                dao_treasury_address: Some(Addr::unchecked("dao_treasury_address").to_string()),
            },
        )
        .unwrap();

    suite.update_time(86400u64 * 2);

    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 2592000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 7776000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 23328000)
        .unwrap();

    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 2592000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 7776000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 23328000)
        .unwrap();

    suite
        .mint_native(suite.admin(), suite.eclip(), 1_000_000_000)
        .unwrap();
    suite
        .increase_eclip_incentives_lockdrop(&suite.admin(), 1_000_000u128)
        .unwrap();
    suite
        .increase_eclip_incentives_lockdrop(&suite.admin(), 1_000_000u128)
        .unwrap();
    // withdraw window finished
    suite.update_time(86400u64 * 7);

    let err = suite.single_lockup_relock(ALICE, 0, 2592000).unwrap_err();
    assert_eq!(ContractError::RelockNotAllowed {}, err.downcast().unwrap());

    // stake assets to single sided vaults and lp vault
    suite.lockdrop_stake_to_vaults(&suite.admin()).unwrap();

    // fund eclip to staking vaults daily reward is 1_000_000_000u128
    suite
        .mint_native(
            suite.reward_distributor_contract(),
            suite.eclip(),
            100_000_000_000u128,
        )
        .unwrap();
    suite
        .mint_native(suite.lp_staking(), suite.eclip(), 100_000_000_000u128)
        .unwrap();

    // restake
    let prev_alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    suite.single_lockup_relock(ALICE, 0, 2592000).unwrap();
    let alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    assert_eq!(alice_eclip_balance - prev_alice_eclip_balance, 27_272u128);
    let alice_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(
        alice_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .eclipastro_withdrawed
            .u128(),
        999u128
    );
    let alice_timelocked_staking = suite.query_timelock_staking(ALICE).unwrap();
    assert_eq!(
        alice_timelocked_staking
            .iter()
            .find(|s| { s.duration == 2592000 })
            .unwrap()
            .staking[0]
            .amount
            .u128(),
        999u128
    );

    let prev_alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    suite.single_lockup_relock(ALICE, 2592000, 7776000).unwrap();
    let alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    assert_eq!(alice_eclip_balance - prev_alice_eclip_balance, 54_545u128);
    let alice_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(
        alice_info
            .iter()
            .find(|i| { i.duration == 2592000 })
            .unwrap()
            .eclipastro_withdrawed
            .u128(),
        999u128
    );
    let alice_timelocked_staking = suite.query_timelock_staking(ALICE).unwrap();
    assert_eq!(
        alice_timelocked_staking
            .iter()
            .find(|s| { s.duration == 7776000 })
            .unwrap()
            .staking[0]
            .amount
            .u128(),
        999u128
    );

    let err = suite
        .single_lockup_relock(ALICE, 2592000, 7776000)
        .unwrap_err();
    assert_eq!(
        ContractError::InvalidTokenBalance {},
        err.downcast().unwrap()
    );

    suite.update_time(86400u64);
    let prev_alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    suite
        .single_lockdrop_claim_rewards_and_optionally_unlock(ALICE, 2592000, None)
        .unwrap();
    let alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();

    let user_eclipastro_balance = suite.query_eclipastro_balance(ALICE).unwrap();
    assert_eq!(user_eclipastro_balance, 0);

    suite
        .single_lockdrop_claim_rewards_and_optionally_unlock(
            ALICE,
            7776000,
            Some(Uint128::from(100u128)),
        )
        .unwrap();

    let user_eclipastro_balance = suite.query_eclipastro_balance(ALICE).unwrap();
    assert_eq!(user_eclipastro_balance, 50); // 50% penalty

    suite
        .lp_lockdrop_claim_rewards_and_optionally_unlock(
            ALICE,
            7776000,
            Some(Uint128::from(400u128)),
        )
        .unwrap();
    let user_lp_token_balance = suite.query_lp_token_balance(ALICE).unwrap();
    assert_eq!(user_lp_token_balance.u128(), 200); // 50% penalty

    assert_eq!(alice_eclip_balance - prev_alice_eclip_balance, 1_414);

    let user_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2592000 })
            .unwrap()
            .eclipastro_staked
            .u128(),
        0
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2592000 })
            .unwrap()
            .eclipastro_withdrawed
            .u128(),
        999
    );

    let err = suite
        .single_lockdrop_claim_rewards_and_optionally_unlock(
            ALICE,
            2592000,
            Some(Uint128::from(100u128)),
        )
        .unwrap_err();
    assert_eq!(
        ContractError::InvalidTokenBalance {},
        err.downcast().unwrap()
    );

    suite
        .single_lockdrop_claim_rewards_and_optionally_unlock(ALICE, 0, None)
        .unwrap();

    let prev_alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    suite
        .lp_lockdrop_claim_rewards_and_optionally_unlock(ALICE, 2592000, None)
        .unwrap();
    let alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    assert_eq!(alice_eclip_balance - prev_alice_eclip_balance, 80958);

    let user_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2592000 })
            .unwrap()
            .lp_token_staked
            .u128(),
        476
    );
    suite
        .lp_lockdrop_claim_rewards_and_optionally_unlock(
            ALICE,
            2592000,
            Some(Uint128::from(476u128)),
        )
        .unwrap();
    let user_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2592000 })
            .unwrap()
            .lp_token_staked
            .u128(),
        0
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2592000 })
            .unwrap()
            .lp_token_withdrawed
            .u128(),
        476
    );

    let err = suite
        .lp_lockdrop_claim_rewards_and_optionally_unlock(
            ALICE,
            2592000,
            Some(Uint128::from(100u128)),
        )
        .unwrap_err();
    assert_eq!(
        ContractError::InvalidTokenBalance {},
        err.downcast().unwrap()
    );
    suite
        .lp_lockdrop_claim_rewards_and_optionally_unlock(ALICE, 0, Some(Uint128::from(100u128)))
        .unwrap();
}
