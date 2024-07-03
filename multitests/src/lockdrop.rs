use astroport::{
    asset::{Asset, AssetInfo},
    vesting::{VestingAccount, VestingSchedule, VestingSchedulePoint},
};
use cosmwasm_std::{Addr, Uint128};
use cw_controllers::AdminError;
use equinox_msg::lockdrop::{
    IncentiveRewards, StakeType, UpdateConfigMsg as LockdropUpdateConfigMsg,
};
// use equinox_msg::lockdrop::UpdateConfigMsg;
use lockdrop::error::ContractError;

use crate::suite::{Suite, SuiteBuilder, ALICE};

const ONE_MONTH: u64 = 86400 * 30;
const THREE_MONTH: u64 = 86400 * 30 * 3;
const SIX_MONTH: u64 = 86400 * 30 * 6;
const NINE_MONTH: u64 = 86400 * 30 * 9;
const ONE_YEAR: u64 = 86400 * 365;

fn instantiate() -> Suite {
    let mut suite = SuiteBuilder::new().build();
    suite.update_config();

    suite
        .mint_native(ALICE.to_string(), suite.astro(), 10_000_000)
        .unwrap();

    // ready ASTRO staking pool
    suite.stake_astro(ALICE, 1_000_000).unwrap();

    // change ASTRO/xASTRO rate 1.1:1
    suite
        .mint_native(suite.astro_staking_contract(), suite.astro(), 100_000)
        .unwrap();

    // ready ASTRO staking pool
    suite.stake_astro(ALICE, 100_000).unwrap();

    suite.convert_astro(ALICE, 1_100_000).unwrap();

    // provide liquidity
    suite
        .provide_liquidity(
            ALICE,
            Addr::unchecked(suite.eclipastro_xastro_lp_contract()),
            vec![
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: Addr::unchecked(suite.eclipastro()),
                    },
                    amount: Uint128::from(1_100_000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: suite.xastro(),
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
            vec![(suite.eclipastro_xastro_lp_token(), Uint128::from(100u128))],
        )
        .unwrap();

    suite
        .incentives_set_tokens_per_second(&suite.admin(), 10u128)
        .unwrap();

    let start_time = suite.get_time();
    let end_time = suite.get_time() + 86400 * 1000;
    suite
        .register_vesting_accounts(
            &suite.admin(),
            vec![VestingAccount {
                address: suite.astroport_incentives(),
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
fn handle_lockdrop() {
    let mut suite = instantiate();

    // lockdrop will fail as lockdrop is not started
    let err = suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 100u128, 0)
        .unwrap_err();
    assert_eq!(
        ContractError::DepositWindowNotStarted {},
        err.downcast().unwrap(),
    );

    // update time and test single stake with all duration(invalid duration check, withdraw flag is false)
    suite.update_time(86400u64 * 2);
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info.single_lockups[0].duration, 0u64);
    let (total_deposit, total_shares) = suite.query_astro_staking_data().unwrap();
    assert_eq!(
        single_lockup_info.single_lockups[0]
            .xastro_amount_in_lockups
            .u128(),
        1_000u128 * total_shares.u128() / total_deposit.u128()
    );
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info[0].duration, 0u64);
    assert_eq!(
        alice_single_lockup_info[0].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares.u128() / total_deposit.u128()
    );

    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    let lp_lockup_info = suite.query_lp_lockup_info().unwrap();
    assert_eq!(lp_lockup_info.lp_lockups[0].duration, 0u64);
    assert_eq!(
        lp_lockup_info.lp_lockups[0].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares.u128() / total_deposit.u128()
    );
    let alice_lp_lockup_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(alice_lp_lockup_info[0].duration, 0u64);
    assert_eq!(
        alice_lp_lockup_info[0].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares.u128() / total_deposit.u128()
    );

    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info.single_lockups[0].duration, 0u64);
    assert_eq!(
        single_lockup_info.single_lockups[0]
            .xastro_amount_in_lockups
            .u128(),
        1_000u128 * total_shares.u128() / total_deposit.u128() * 2
    );
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info[0].duration, 0u64);
    assert_eq!(
        alice_single_lockup_info[0].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares.u128() / total_deposit.u128() * 2
    );

    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    let lp_lockup_info = suite.query_lp_lockup_info().unwrap();
    assert_eq!(lp_lockup_info.lp_lockups[0].duration, 0u64);
    assert_eq!(
        lp_lockup_info.lp_lockups[0].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares.u128() / total_deposit.u128() * 2
    );
    let alice_lp_lockup_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(alice_lp_lockup_info[0].duration, 0u64);
    assert_eq!(
        alice_lp_lockup_info[0].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares.u128() / total_deposit.u128() * 2
    );

    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, ONE_MONTH)
        .unwrap();
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info.single_lockups[1].duration, ONE_MONTH);
    assert_eq!(
        single_lockup_info.single_lockups[1]
            .xastro_amount_in_lockups
            .u128(),
        1_000u128 * total_shares.u128() / total_deposit.u128()
    );
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info[1].duration, ONE_MONTH);
    assert_eq!(
        alice_single_lockup_info[1].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares.u128() / total_deposit.u128()
    );

    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, THREE_MONTH)
        .unwrap();
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info.single_lockups[2].duration, THREE_MONTH);
    assert_eq!(
        single_lockup_info.single_lockups[2]
            .xastro_amount_in_lockups
            .u128(),
        1_000u128 * total_shares.u128() / total_deposit.u128()
    );
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info[2].duration, THREE_MONTH);
    assert_eq!(
        alice_single_lockup_info[2].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares.u128() / total_deposit.u128()
    );

    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, SIX_MONTH)
        .unwrap();
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info.single_lockups[3].duration, SIX_MONTH);
    assert_eq!(
        single_lockup_info.single_lockups[3]
            .xastro_amount_in_lockups
            .u128(),
        1_000u128 * total_shares.u128() / total_deposit.u128()
    );
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info[3].duration, SIX_MONTH);
    assert_eq!(
        alice_single_lockup_info[3].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares.u128() / total_deposit.u128()
    );

    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, ONE_YEAR)
        .unwrap();
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info.single_lockups[4].duration, ONE_YEAR);
    assert_eq!(
        single_lockup_info.single_lockups[4]
            .xastro_amount_in_lockups
            .u128(),
        1_000u128 * total_shares.u128() / total_deposit.u128()
    );
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info[4].duration, ONE_YEAR);
    assert_eq!(
        alice_single_lockup_info[4].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares.u128() / total_deposit.u128()
    );

    suite.stake_astro(ALICE, 4_000u128).unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.xastro(), 1_000u128, ONE_MONTH)
        .unwrap();
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info.single_lockups[1].duration, ONE_MONTH);
    assert_eq!(
        single_lockup_info.single_lockups[1]
            .xastro_amount_in_lockups
            .u128(),
        1_000u128 + 1_000u128 * total_shares.u128() / total_deposit.u128()
    );
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info[1].duration, ONE_MONTH);
    assert_eq!(
        alice_single_lockup_info[1].xastro_amount_in_lockups.u128(),
        1_000u128 + 1_000u128 * total_shares.u128() / total_deposit.u128()
    );

    suite
        .lp_staking_increase_lockdrop(ALICE, suite.xastro(), 1_000u128, ONE_MONTH)
        .unwrap();
    let lp_lockup_info = suite.query_lp_lockup_info().unwrap();
    assert_eq!(lp_lockup_info.lp_lockups[1].duration, ONE_MONTH);
    assert_eq!(
        lp_lockup_info.lp_lockups[1].xastro_amount_in_lockups.u128(),
        1_000u128
    );
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
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, ONE_MONTH)
        .unwrap_err();
    assert_eq!(
        ContractError::DepositWindowClosed {},
        err.downcast().unwrap()
    );

    let err = suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, ONE_MONTH)
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
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
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
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    suite
        .single_staking_extend_duration_with_deposit(
            ALICE,
            suite.astro(),
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
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
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
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    let err = suite
        .single_staking_extend_duration_with_deposit(ALICE, suite.astro(), 500u128, 0, 0)
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
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    suite.update_time(86400 * 5);
    let err = suite
        .single_staking_extend_duration_without_deposit(ALICE, 0, 7776000)
        .unwrap_err();
    assert_eq!(ContractError::ExtendLockupError {}, err.downcast().unwrap());
    let err = suite
        .single_staking_extend_duration_with_deposit(ALICE, suite.astro(), 500u128, 0, 7776000)
        .unwrap_err();
    assert_eq!(ContractError::ExtendLockupError {}, err.downcast().unwrap());
}

#[test]
fn extend_lock_single_sided_timelocked() {
    let mut suite = instantiate();

    // start lockdrop deposit window(deposit window day 2)
    suite.update_time(86400u64 * 2);

    // alice deposits 1000 astro and extends from 1 month to 3 months with no deposit
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 2592000)
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
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 2592000)
        .unwrap();
    suite
        .single_staking_extend_duration_with_deposit(
            ALICE,
            suite.astro(),
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
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 2592000)
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
            suite.astro(),
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
    assert_eq!(ContractError::ExtendLockupError {}, err.downcast().unwrap());
    let err = suite
        .single_staking_extend_duration_with_deposit(
            ALICE,
            suite.astro(),
            500u128,
            2592000,
            7776000,
        )
        .unwrap_err();
    assert_eq!(ContractError::ExtendLockupError {}, err.downcast().unwrap());
}

#[test]
fn extend_lock_lp_flexible() {
    let mut suite = instantiate();

    // start lockdrop deposit window(deposit window day 2)
    suite.update_time(86400u64 * 2);

    // alice deposits 1000 astro and extends from single sided flexible to 3 months with no deposit
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
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
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    suite
        .lp_lockup_extend_duration_with_deposit(ALICE, suite.astro(), 500u128, 0, 86400 * 30 * 3)
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
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
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
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    let err = suite
        .lp_lockup_extend_duration_with_deposit(ALICE, suite.astro(), 500u128, 0, 0)
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
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    suite.update_time(86400 * 5);
    let err = suite
        .lp_lockup_extend_duration_without_deposit(ALICE, 0, 7776000)
        .unwrap_err();
    assert_eq!(ContractError::ExtendLockupError {}, err.downcast().unwrap());
    let err = suite
        .lp_lockup_extend_duration_with_deposit(ALICE, suite.astro(), 500u128, 0, 7776000)
        .unwrap_err();
    assert_eq!(ContractError::ExtendLockupError {}, err.downcast().unwrap());
}

#[test]
fn single_sided_withdraw() {
    let mut suite = instantiate();

    // start lockdrop deposit window(deposit window day 2)
    suite.update_time(86400u64 * 2);

    // alice deposits 1000 astro and withdraw it
    let old_alice_xastro_balance = suite
        .query_balance_native(ALICE.to_string(), suite.xastro())
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 2592000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 7776000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 23328000)
        .unwrap();
    // start withdraw window(withdraw window started)
    suite.update_time(86400u64 * 4);
    // if withdraw amount is greater than max withdrawal amount, only withdraws max withdrawal amount
    let err = suite
        .single_staking_lockdrop_withdraw(ALICE, Some(Uint128::from(1000u128)), 0)
        .unwrap_err();
    assert_eq!(
        ContractError::WithdrawLimitExceed(454u128.to_string()),
        err.downcast().unwrap()
    );
    // alice withdraws 300
    suite
        .single_staking_lockdrop_withdraw(ALICE, Some(Uint128::from(300u128)), 0)
        .unwrap();
    let alice_xastro_balance = suite
        .query_balance_native(ALICE.to_string(), suite.xastro())
        .unwrap();
    assert_eq!(alice_xastro_balance - old_alice_xastro_balance, 300u128);
    let current_time = suite.get_time();
    let config = suite.query_lockdrop_config().unwrap();
    assert_eq!(current_time, config.init_timestamp + config.deposit_window);
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
    let old_alice_xastro_balance = suite
        .query_balance_native(ALICE.to_string(), suite.xastro())
        .unwrap();
    suite
        .single_staking_lockdrop_withdraw(ALICE, None, 2592000)
        .unwrap();
    let alice_xastro_balance = suite
        .query_balance_native(ALICE.to_string(), suite.xastro())
        .unwrap();
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
    let old_alice_xastro_balance = suite
        .query_balance_native(ALICE.to_string(), suite.xastro())
        .unwrap();
    suite
        .single_staking_lockdrop_withdraw(ALICE, None, 7776000)
        .unwrap();
    let alice_xastro_balance = suite
        .query_balance_native(ALICE.to_string(), suite.xastro())
        .unwrap();
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
    assert_eq!(
        ContractError::ClaimRewardNotAllowed {},
        err.downcast().unwrap()
    );
}

#[test]
fn lp_withdraw() {
    let mut suite = instantiate();

    // start lockdrop deposit window(deposit window day 2)
    suite.update_time(86400u64 * 2);

    // alice deposits 1000 astro and withdraw it
    let old_alice_xastro_balance = suite
        .query_balance_native(ALICE.to_string(), suite.xastro())
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 2592000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 7776000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 23328000)
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
    let alice_xastro_balance = suite
        .query_balance_native(ALICE.to_string(), suite.xastro())
        .unwrap();
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
    let old_alice_xastro_balance = suite
        .query_balance_native(ALICE.to_string(), suite.xastro())
        .unwrap();
    suite
        .lp_staking_lockdrop_withdraw(ALICE, None, 2592000)
        .unwrap();
    let alice_xastro_balance = suite
        .query_balance_native(ALICE.to_string(), suite.xastro())
        .unwrap();
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
    let old_alice_xastro_balance = suite
        .query_balance_native(ALICE.to_string(), suite.xastro())
        .unwrap();
    suite
        .lp_staking_lockdrop_withdraw(ALICE, None, 7776000)
        .unwrap();
    let alice_xastro_balance = suite
        .query_balance_native(ALICE.to_string(), suite.xastro())
        .unwrap();
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
    assert_eq!(
        ContractError::ClaimRewardNotAllowed {},
        err.downcast().unwrap()
    );
}

// test funding
#[test]
fn fund_incentives() {
    let mut suite = instantiate();
    suite.mint_beclip(&suite.admin(), 1_000_000_000).unwrap();
    // test increase incentives before deposit window
    suite
        .fund_beclip(
            &suite.admin(),
            1_000_000u128,
            vec![
                IncentiveRewards {
                    stake_type: StakeType::SingleStaking,
                    amount: Uint128::from(300_000u128),
                },
                IncentiveRewards {
                    stake_type: StakeType::LpStaking,
                    amount: Uint128::from(700_000u128),
                },
            ],
        )
        .unwrap();

    // test increase incentives on deposit window
    suite.update_time(86400u64 * 2);
    suite
        .fund_beclip(
            &suite.admin(),
            1_000_000u128,
            vec![
                IncentiveRewards {
                    stake_type: StakeType::SingleStaking,
                    amount: Uint128::from(300_000u128),
                },
                IncentiveRewards {
                    stake_type: StakeType::LpStaking,
                    amount: Uint128::from(700_000u128),
                },
            ],
        )
        .unwrap();
}

// test staking assets to vaults
#[test]
fn stake_assets_to_vaults() {
    let mut suite = instantiate();

    suite.update_time(86400u64 * 2);
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 2592000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 7776000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 23328000)
        .unwrap();

    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 2592000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 7776000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 23328000)
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

    suite
        .update_lockdrop_config(
            &suite.admin(),
            LockdropUpdateConfigMsg {
                single_sided_staking: Some(Addr::unchecked(suite.single_staking_contract())),
                lp_staking: Some(Addr::unchecked(suite.lp_staking_contract())),
                liquidity_pool: Some(Addr::unchecked(suite.eclipastro_xastro_lp_contract())),
                eclipastro_token: Some(Addr::unchecked(suite.eclipastro())),
                converter: Some(Addr::unchecked(suite.converter_contract())),
                dao_treasury_address: Some(Addr::unchecked(suite.treasury())),
            },
        )
        .unwrap();

    suite.lockdrop_stake_to_vaults(&suite.admin()).unwrap();
    let single_state = suite.query_single_lockup_state().unwrap();
    let single_info = suite.query_single_lockup_info().unwrap();

    assert_eq!(
        single_info
            .single_lockups
            .iter()
            .find(|i| { i.duration == 0u64 })
            .unwrap()
            .total_eclipastro_staked
            .u128(),
        999u128
    );
    assert_eq!(
        single_info
            .single_lockups
            .iter()
            .find(|i| { i.duration == 2592000u64 })
            .unwrap()
            .total_eclipastro_staked
            .u128(),
        999u128
    );
    assert_eq!(
        single_info
            .single_lockups
            .iter()
            .find(|i| { i.duration == 7776000u64 })
            .unwrap()
            .total_eclipastro_staked
            .u128(),
        999u128
    );
    assert_eq!(
        single_info
            .single_lockups
            .iter()
            .find(|i| { i.duration == 23328000u64 })
            .unwrap()
            .total_eclipastro_staked
            .u128(),
        999u128
    );

    assert_eq!(
        single_info
            .single_lockups
            .iter()
            .find(|i| { i.duration == 0u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );
    assert_eq!(
        single_info
            .single_lockups
            .iter()
            .find(|i| { i.duration == 2592000u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );
    assert_eq!(
        single_info
            .single_lockups
            .iter()
            .find(|i| { i.duration == 7776000u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );
    assert_eq!(
        single_info
            .single_lockups
            .iter()
            .find(|i| { i.duration == 23328000u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );

    assert_eq!(
        single_info
            .single_lockups
            .iter()
            .find(|i| { i.duration == 0u64 })
            .unwrap()
            .total_eclipastro_withdrawed
            .u128(),
        0u128
    );
    assert_eq!(
        single_info
            .single_lockups
            .iter()
            .find(|i| { i.duration == 2592000u64 })
            .unwrap()
            .total_eclipastro_withdrawed
            .u128(),
        0u128
    );
    assert_eq!(
        single_info
            .single_lockups
            .iter()
            .find(|i| { i.duration == 7776000u64 })
            .unwrap()
            .total_eclipastro_withdrawed
            .u128(),
        0u128
    );
    assert_eq!(
        single_info
            .single_lockups
            .iter()
            .find(|i| { i.duration == 23328000u64 })
            .unwrap()
            .total_eclipastro_withdrawed
            .u128(),
        0u128
    );

    assert_eq!(single_state.are_claims_allowed, true);
    assert_eq!(single_state.countdown_start_at, suite.get_time());
    assert_eq!(single_state.total_eclipastro_lockup.u128(), 3_996u128);

    let lp_state = suite.query_lp_lockup_state().unwrap();
    let lp_info = suite.query_lp_lockup_info().unwrap();

    assert_eq!(
        lp_info
            .lp_lockups
            .iter()
            .find(|i| { i.duration == 0u64 })
            .unwrap()
            .total_lp_staked
            .u128(),
        476u128
    );
    assert_eq!(
        lp_info
            .lp_lockups
            .iter()
            .find(|i| { i.duration == 2592000u64 })
            .unwrap()
            .total_lp_staked
            .u128(),
        476u128
    );
    assert_eq!(
        lp_info
            .lp_lockups
            .iter()
            .find(|i| { i.duration == 7776000u64 })
            .unwrap()
            .total_lp_staked
            .u128(),
        476u128
    );
    assert_eq!(
        lp_info
            .lp_lockups
            .iter()
            .find(|i| { i.duration == 23328000u64 })
            .unwrap()
            .total_lp_staked
            .u128(),
        476u128
    );

    assert_eq!(
        lp_info
            .lp_lockups
            .iter()
            .find(|i| { i.duration == 0u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );
    assert_eq!(
        lp_info
            .lp_lockups
            .iter()
            .find(|i| { i.duration == 2592000u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );
    assert_eq!(
        lp_info
            .lp_lockups
            .iter()
            .find(|i| { i.duration == 7776000u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );
    assert_eq!(
        lp_info
            .lp_lockups
            .iter()
            .find(|i| { i.duration == 23328000u64 })
            .unwrap()
            .xastro_amount_in_lockups
            .u128(),
        909u128
    );

    assert_eq!(
        lp_info
            .lp_lockups
            .iter()
            .find(|i| { i.duration == 0u64 })
            .unwrap()
            .total_lp_withdrawed
            .u128(),
        0u128
    );
    assert_eq!(
        lp_info
            .lp_lockups
            .iter()
            .find(|i| { i.duration == 2592000u64 })
            .unwrap()
            .total_lp_withdrawed
            .u128(),
        0u128
    );
    assert_eq!(
        lp_info
            .lp_lockups
            .iter()
            .find(|i| { i.duration == 7776000u64 })
            .unwrap()
            .total_lp_withdrawed
            .u128(),
        0u128
    );
    assert_eq!(
        lp_info
            .lp_lockups
            .iter()
            .find(|i| { i.duration == 23328000u64 })
            .unwrap()
            .total_lp_withdrawed
            .u128(),
        0u128
    );
    assert_eq!(lp_state.are_claims_allowed, true);
    assert_eq!(lp_state.countdown_start_at, suite.get_time());
}

// test distribution of the eclip incentives
#[test]
fn single_sided_incentives_distribution() {
    let mut suite = instantiate();

    suite.update_time(86400u64 * 2);

    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 2592000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 7776000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 23328000)
        .unwrap();

    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 2592000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 7776000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 23328000)
        .unwrap();

    suite.mint_beclip(&suite.admin(), 1_000_000_000).unwrap();
    suite
        .mint_native(suite.admin(), suite.eclip(), 1_000_000_000)
        .unwrap();
    suite
        .fund_beclip(
            &suite.admin(),
            1_000_000u128,
            vec![
                IncentiveRewards {
                    stake_type: StakeType::SingleStaking,
                    amount: Uint128::from(300_000u128),
                },
                IncentiveRewards {
                    stake_type: StakeType::LpStaking,
                    amount: Uint128::from(700_000u128),
                },
            ],
        )
        .unwrap();
    suite
        .fund_eclip(
            &suite.admin(),
            1_000_000u128,
            vec![
                IncentiveRewards {
                    stake_type: StakeType::SingleStaking,
                    amount: Uint128::from(300_000u128),
                },
                IncentiveRewards {
                    stake_type: StakeType::LpStaking,
                    amount: Uint128::from(700_000u128),
                },
            ],
        )
        .unwrap();
    // withdraw window finished
    suite.update_time(86400u64 * 7);
    let cfg = suite.query_lockdrop_config().unwrap();
    assert!(cfg.init_timestamp + cfg.deposit_window + cfg.withdrawal_window < suite.get_time());

    suite
        .update_lockdrop_config(
            &suite.admin(),
            LockdropUpdateConfigMsg {
                single_sided_staking: Some(Addr::unchecked(suite.single_staking_contract())),
                lp_staking: Some(Addr::unchecked(suite.lp_staking_contract())),
                liquidity_pool: Some(Addr::unchecked(suite.eclipastro_xastro_lp_contract())),
                eclipastro_token: Some(Addr::unchecked(suite.eclipastro())),
                converter: Some(Addr::unchecked(suite.converter_contract())),
                dao_treasury_address: Some(Addr::unchecked(suite.treasury())),
            },
        )
        .unwrap();

    // stake assets to single sided vaults and lp vault
    suite.lockdrop_stake_to_vaults(&suite.admin()).unwrap();

    // fund eclip to staking vaults daily reward is 1_000_000_000u128
    suite
        .mint_beclip(&suite.single_staking_contract(), 100_000_000_000u128)
        .unwrap();
    suite
        .mint_native(
            suite.single_staking_contract(),
            suite.eclip(),
            100_000_000_000u128,
        )
        .unwrap();
    suite
        .mint_beclip(&&suite.lp_staking_contract(), 100_000_000_000u128)
        .unwrap();
    suite
        .mint_native(
            suite.lp_staking_contract(),
            suite.eclip(),
            100_000_000_000u128,
        )
        .unwrap();
    // test eclip incentives
    let prev_alice_beclip_balance = suite.query_beclip_balance(ALICE).unwrap();
    let prev_alice_eclip_balance = suite
        .query_balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    suite.single_lockdrop_claim_rewards(ALICE, 0, None).unwrap();
    let alice_beclip_balance = suite.query_beclip_balance(ALICE).unwrap();
    let alice_eclip_balance = suite
        .query_balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    assert_eq!(alice_beclip_balance - prev_alice_beclip_balance, 28547); // 100%
    assert_eq!(alice_eclip_balance - prev_alice_eclip_balance, 28547); // 100%
    let user_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .lockdrop_incentives
            .beclip
            .claimed
            .u128(),
        28547
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .lockdrop_incentives
            .beclip
            .allocated
            .u128(),
        28547
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .lockdrop_incentives
            .eclip
            .claimed
            .u128(),
        28547
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .lockdrop_incentives
            .eclip
            .allocated
            .u128(),
        28547
    );
    suite
        .single_lockdrop_claim_rewards(ALICE, 2592000, None)
        .unwrap();
    let alice_beclip_balance = suite.query_beclip_balance(ALICE).unwrap();
    let alice_eclip_balance = suite
        .query_balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    assert_eq!(alice_beclip_balance - prev_alice_beclip_balance, 85704); // 100%
    assert_eq!(alice_eclip_balance - prev_alice_eclip_balance, 85704); // 100%
    let user_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2592000 })
            .unwrap()
            .lockdrop_incentives
            .beclip
            .claimed
            .u128(),
        57157
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2592000 })
            .unwrap()
            .lockdrop_incentives
            .eclip
            .claimed
            .u128(),
        57157
    );
}

#[test]
fn lp_incentives_distribution() {
    let mut suite = instantiate();

    suite.update_time(86400u64 * 2);

    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, ONE_MONTH)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, THREE_MONTH)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, NINE_MONTH)
        .unwrap();

    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, ONE_MONTH)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, THREE_MONTH)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, NINE_MONTH)
        .unwrap();

    let eclip_incentives_single_staking = 300_000u128;
    let eclip_incentives_lp_staking = 700_000u128;
    let beclip_incentives_single_staking = 300_000u128;
    let beclip_incentives_lp_staking = 700_000u128;

    suite
        .mint_beclip(
            &suite.admin(),
            beclip_incentives_single_staking + beclip_incentives_lp_staking,
        )
        .unwrap();
    suite
        .mint_native(
            suite.admin(),
            suite.eclip(),
            eclip_incentives_single_staking + eclip_incentives_lp_staking,
        )
        .unwrap();
    suite
        .fund_beclip(
            &suite.admin(),
            beclip_incentives_single_staking + beclip_incentives_lp_staking,
            vec![
                IncentiveRewards {
                    stake_type: StakeType::SingleStaking,
                    amount: Uint128::from(beclip_incentives_single_staking),
                },
                IncentiveRewards {
                    stake_type: StakeType::LpStaking,
                    amount: Uint128::from(beclip_incentives_lp_staking),
                },
            ],
        )
        .unwrap();
    suite
        .fund_eclip(
            &suite.admin(),
            eclip_incentives_single_staking + eclip_incentives_lp_staking,
            vec![
                IncentiveRewards {
                    stake_type: StakeType::SingleStaking,
                    amount: Uint128::from(eclip_incentives_single_staking),
                },
                IncentiveRewards {
                    stake_type: StakeType::LpStaking,
                    amount: Uint128::from(eclip_incentives_lp_staking),
                },
            ],
        )
        .unwrap();
    // withdraw window finished
    suite.update_time(86400u64 * 7);
    let cfg = suite.query_lockdrop_config().unwrap();
    assert!(cfg.init_timestamp + cfg.deposit_window + cfg.withdrawal_window < suite.get_time());

    suite
        .update_lockdrop_config(
            &suite.admin(),
            LockdropUpdateConfigMsg {
                single_sided_staking: Some(Addr::unchecked(suite.single_staking_contract())),
                lp_staking: Some(Addr::unchecked(suite.lp_staking_contract())),
                liquidity_pool: Some(Addr::unchecked(suite.eclipastro_xastro_lp_contract())),
                eclipastro_token: Some(Addr::unchecked(suite.eclipastro())),
                converter: Some(Addr::unchecked(suite.converter_contract())),
                dao_treasury_address: Some(Addr::unchecked(suite.treasury())),
            },
        )
        .unwrap();

    // stake assets to single sided vaults and lp vault
    suite.lockdrop_stake_to_vaults(&suite.admin()).unwrap();

    // fund eclip to staking vaults daily reward is 1_000_000_000u128
    suite
        .mint_beclip(&suite.single_staking_contract(), 100_000_000_000u128)
        .unwrap();
    suite
        .mint_native(
            suite.single_staking_contract(),
            suite.eclip(),
            100_000_000_000u128,
        )
        .unwrap();
    suite
        .mint_beclip(&suite.lp_staking_contract(), 100_000_000_000u128)
        .unwrap();
    suite
        .mint_native(
            suite.lp_staking_contract(),
            suite.eclip(),
            100_000_000_000u128,
        )
        .unwrap();
    // test eclip incentives
    let prev_alice_beclip_balance = suite.query_beclip_balance(ALICE).unwrap();
    let user_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .lockdrop_incentives
            .eclip
            .allocated
            .u128(),
        66610
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .lockdrop_incentives
            .beclip
            .allocated
            .u128(),
        66610 // 0.5x
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2_592_000 })
            .unwrap()
            .lockdrop_incentives
            .beclip
            .allocated
            .u128(),
        133368 // 1x
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2_592_000 })
            .unwrap()
            .lockdrop_incentives
            .eclip
            .allocated
            .u128(),
        133368
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 7_776_000 })
            .unwrap()
            .lockdrop_incentives
            .beclip
            .allocated
            .u128(),
        199979 //1.5x
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 7_776_000 })
            .unwrap()
            .lockdrop_incentives
            .eclip
            .allocated
            .u128(),
        199979
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 23_328_000 })
            .unwrap()
            .lockdrop_incentives
            .beclip
            .allocated
            .u128(),
        300041 //2.25x
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 23_328_000 })
            .unwrap()
            .lockdrop_incentives
            .eclip
            .allocated
            .u128(),
        300041
    );
    suite.lp_lockdrop_claim_all_rewards(ALICE, None).unwrap();
    let alice_beclip_balance: u128 = suite.query_beclip_balance(ALICE).unwrap();
    assert_eq!(alice_beclip_balance - prev_alice_beclip_balance, 699998); // 100%
    let user_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .lockdrop_incentives
            .beclip
            .claimed
            .u128(),
        66610
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 0 })
            .unwrap()
            .lockdrop_incentives
            .eclip
            .claimed
            .u128(),
        66610
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2_592_000 })
            .unwrap()
            .lockdrop_incentives
            .beclip
            .claimed
            .u128(),
        133368
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2_592_000 })
            .unwrap()
            .lockdrop_incentives
            .eclip
            .claimed
            .u128(),
        133368
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 7_776_000 })
            .unwrap()
            .lockdrop_incentives
            .beclip
            .claimed
            .u128(),
        199979
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 7_776_000 })
            .unwrap()
            .lockdrop_incentives
            .eclip
            .claimed
            .u128(),
        199979
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 23_328_000 })
            .unwrap()
            .lockdrop_incentives
            .beclip
            .claimed
            .u128(),
        300041
    );
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 23_328_000 })
            .unwrap()
            .lockdrop_incentives
            .eclip
            .claimed
            .u128(),
        300041
    );
}

#[test]
fn restake_and_unlock() {
    let mut suite = instantiate();
    suite.update_time(86400u64 * 2);

    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 2592000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 7776000)
        .unwrap();
    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 23328000)
        .unwrap();

    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 0)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 2592000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 7776000)
        .unwrap();
    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro(), 1_000u128, 23328000)
        .unwrap();

    suite.mint_beclip(&suite.admin(), 1_000_000_000).unwrap();
    suite
        .mint_native(suite.admin(), suite.eclip(), 1_000_000_000)
        .unwrap();
    suite
        .fund_beclip(
            &suite.admin(),
            1_000_000u128,
            vec![
                IncentiveRewards {
                    stake_type: StakeType::SingleStaking,
                    amount: Uint128::from(300_000u128),
                },
                IncentiveRewards {
                    stake_type: StakeType::LpStaking,
                    amount: Uint128::from(700_000u128),
                },
            ],
        )
        .unwrap();
    suite
        .fund_eclip(
            &suite.admin(),
            1_000_000u128,
            vec![
                IncentiveRewards {
                    stake_type: StakeType::SingleStaking,
                    amount: Uint128::from(300_000u128),
                },
                IncentiveRewards {
                    stake_type: StakeType::LpStaking,
                    amount: Uint128::from(700_000u128),
                },
            ],
        )
        .unwrap();
    // withdraw window finished
    suite.update_time(86400u64 * 7);

    let err = suite.single_lockup_relock(ALICE, 0, 2592000).unwrap_err();
    assert_eq!(ContractError::ExtendLockupError {}, err.downcast().unwrap());

    suite
        .update_lockdrop_config(
            &suite.admin(),
            LockdropUpdateConfigMsg {
                single_sided_staking: Some(Addr::unchecked(suite.single_staking_contract())),
                lp_staking: Some(Addr::unchecked(suite.lp_staking_contract())),
                liquidity_pool: Some(Addr::unchecked(suite.eclipastro_xastro_lp_contract())),
                eclipastro_token: Some(Addr::unchecked(suite.eclipastro())),
                converter: Some(Addr::unchecked(suite.converter_contract())),
                dao_treasury_address: Some(Addr::unchecked(suite.treasury())),
            },
        )
        .unwrap();

    // stake assets to single sided vaults and lp vault
    suite.lockdrop_stake_to_vaults(&suite.admin()).unwrap();

    // fund eclip to staking vaults daily reward is 1_000_000_000u128
    suite
        .mint_beclip(&suite.single_staking_contract(), 100_000_000_000u128)
        .unwrap();
    suite
        .mint_native(
            suite.single_staking_contract(),
            suite.eclip(),
            100_000_000_000u128,
        )
        .unwrap();
    suite
        .mint_beclip(&suite.lp_staking_contract(), 100_000_000_000u128)
        .unwrap();
    suite
        .mint_native(
            suite.lp_staking_contract(),
            suite.eclip(),
            100_000_000_000u128,
        )
        .unwrap();

    // restake
    let prev_alice_beclip_balance = suite.query_beclip_balance(ALICE).unwrap();
    suite.single_lockup_relock(ALICE, 0, 2592000).unwrap();
    let alice_beclip_balance = suite.query_beclip_balance(ALICE).unwrap();
    assert_eq!(alice_beclip_balance - prev_alice_beclip_balance, 28547); // flexible incentives
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
    let alice_single_sided_staking = suite.query_single_sided_staking(ALICE).unwrap();
    assert_eq!(
        alice_single_sided_staking
            .iter()
            .find(|s| { s.duration == 2592000 })
            .unwrap()
            .staking[0]
            .amount
            .u128(),
        999u128
    );

    let prev_alice_beclip_balance = suite.query_beclip_balance(ALICE).unwrap();
    let prev_alice_eclip_balance = suite
        .query_balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    suite.single_lockup_relock(ALICE, 2592000, 7776000).unwrap();
    let alice_beclip_balance = suite.query_beclip_balance(ALICE).unwrap();
    assert_eq!(alice_beclip_balance - prev_alice_beclip_balance, 57157);
    let alice_eclip_balance = suite
        .query_balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    assert_eq!(alice_eclip_balance - prev_alice_eclip_balance, 57157);
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
    let alice_single_sided_staking = suite.query_single_sided_staking(ALICE).unwrap();
    assert_eq!(
        alice_single_sided_staking
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
    assert_eq!(ContractError::NotStaked {}, err.downcast().unwrap());

    suite.update_time(86400u64);
    let prev_alice_beclip_balance = suite.query_beclip_balance(ALICE).unwrap();
    suite
        .single_lockdrop_claim_rewards(ALICE, 2592000, None)
        .unwrap();
    // let user_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    // assert_eq!(user_info, vec![]);
    let alice_beclip_balance = suite.query_beclip_balance(ALICE).unwrap();

    let user_eclipastro_balance = suite.query_eclipastro_balance(ALICE).unwrap();
    assert_eq!(user_eclipastro_balance, 0);

    suite
        .single_lockup_unlock(ALICE, 7776000, Some(Uint128::from(100u128)))
        .unwrap();

    let user_eclipastro_balance = suite.query_eclipastro_balance(ALICE).unwrap();
    assert_eq!(user_eclipastro_balance, 50); // 50% penalty

    let prev_user_lp_token_balance = suite.query_lp_token_balance(ALICE).unwrap();
    suite
        .lp_lockdrop_claim_rewards(ALICE, 7776000, None)
        .unwrap();
    suite
        .lp_lockup_unlock(ALICE, 7776000, Some(Uint128::from(400u128)))
        .unwrap();

    let user_lp_token_balance = suite.query_lp_token_balance(ALICE).unwrap();
    assert_eq!(
        user_lp_token_balance.u128() - prev_user_lp_token_balance.u128(),
        200
    ); // 50% penalty

    assert_eq!(alice_beclip_balance - prev_alice_beclip_balance, 0);

    let user_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(
        user_info
            .iter()
            .find(|i| { i.duration == 2592000 })
            .unwrap()
            .eclipastro_staked
            .u128(),
        999
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
        .single_lockup_unlock(ALICE, 2592000, Some(Uint128::from(100u128)))
        .unwrap_err();
    assert_eq!(
        ContractError::WithdrawLimitExceed("0".to_string()),
        err.downcast().unwrap()
    );

    suite.single_lockdrop_claim_rewards(ALICE, 0, None).unwrap();

    let prev_alice_beclip_balance = suite.query_beclip_balance(ALICE).unwrap();
    suite
        .lp_lockdrop_claim_rewards(ALICE, 2592000, None)
        .unwrap();
    let alice_beclip_balance = suite.query_beclip_balance(ALICE).unwrap();
    assert_eq!(alice_beclip_balance - prev_alice_beclip_balance, 133368);

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
        .lp_lockup_unlock(ALICE, 2592000, Some(Uint128::from(476u128)))
        .unwrap();
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
        .lp_lockup_unlock(ALICE, 2592000, Some(Uint128::from(100u128)))
        .unwrap_err();
    assert_eq!(
        ContractError::WithdrawLimitExceed("0".to_string()),
        err.downcast().unwrap()
    );
    suite
        .lp_lockup_unlock(ALICE, 0, Some(Uint128::from(100u128)))
        .unwrap();
    // let prev_eclip_balance = suite.query_balance_native(ALICE.to_string(), suite.eclip()).unwrap();
    // suite.lp_lockdrop_claim_all_rewards(ALICE).unwrap();
    // let eclip_balance = suite.query_balance_native(ALICE.to_string(), suite.eclip()).unwrap();
    // assert_eq!(eclip_balance-prev_eclip_balance, 0u128);
    // let info = suite.query_lockdrop_config().unwrap();
    // assert_eq!(info., vec![])
    // let user_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    // assert_eq!(user_info, vec![]);

    // check claim with duplicated assets
    let err = suite
        .single_lockdrop_claim_rewards(
            ALICE,
            0,
            Some(vec![
                AssetInfo::NativeToken {
                    denom: suite.eclip(),
                },
                AssetInfo::NativeToken {
                    denom: suite.eclip(),
                },
            ]),
        )
        .unwrap_err();
    assert_eq!(ContractError::DuplicatedAssets {}, err.downcast().unwrap());
    let err = suite
        .lp_lockdrop_claim_rewards(
            ALICE,
            0,
            Some(vec![
                AssetInfo::NativeToken {
                    denom: suite.eclip(),
                },
                AssetInfo::NativeToken {
                    denom: suite.eclip(),
                },
            ]),
        )
        .unwrap_err();
    assert_eq!(ContractError::DuplicatedAssets {}, err.downcast().unwrap());
    let err = suite
        .single_lockdrop_claim_all_rewards(
            ALICE,
            Some(vec![
                AssetInfo::NativeToken {
                    denom: suite.eclip(),
                },
                AssetInfo::NativeToken {
                    denom: suite.eclip(),
                },
            ]),
        )
        .unwrap_err();
    assert_eq!(ContractError::DuplicatedAssets {}, err.downcast().unwrap());
    let err = suite
        .lp_lockdrop_claim_all_rewards(
            ALICE,
            Some(vec![
                AssetInfo::NativeToken {
                    denom: suite.eclip(),
                },
                AssetInfo::NativeToken {
                    denom: suite.eclip(),
                },
            ]),
        )
        .unwrap_err();
    assert_eq!(ContractError::DuplicatedAssets {}, err.downcast().unwrap());
}
