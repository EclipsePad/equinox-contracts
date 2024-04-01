use astroport::{
    asset::{Asset, AssetInfo},
    vesting::{VestingAccount, VestingSchedule, VestingSchedulePoint},
};
use cosmwasm_std::{Addr, Uint128};
use equinox_msg::lockdrop::{StakeType, UpdateConfigMsg};
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
        (ONE_MONTH, 1),
        (THREE_MONTH, 2),
        (SIX_MONTH, 3),
        (NINE_MONTH, 4),
        (ONE_YEAR, 5),
    ];
    let eclip_daily_reward = 100_000u128;
    let locking_reward_config = vec![
        (0, 1),
        (ONE_MONTH, 2),
        (THREE_MONTH, 3),
        (SIX_MONTH, 4),
        (ONE_YEAR, 5),
    ];

    let mut suite = SuiteBuilder::new()
        .with_initial_balances(astro_initial_balances)
        .with_timelock_config(timelock_config)
        .with_eclip_daily_reward(eclip_daily_reward)
        .with_lp_staking_eclip_daily_reward(eclip_daily_reward)
        .with_locking_reward_config(locking_reward_config.clone())
        .with_lock_configs(locking_reward_config)
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
        .mint_native(suite.admin(), suite.eclip(), 2_000_000_000u128)
        .unwrap();
    suite
        .increase_eclip_incentives_lockdrop(
            &suite.admin(),
            StakeType::SingleStaking,
            1_000_000_000u128,
        )
        .unwrap();
    let single_lockup_state = suite.query_single_lockup_state().unwrap();
    assert_eq!(
        single_lockup_state.total_eclip_incentives.u128(),
        1_000_000_000u128
    );
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info, vec![]);
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info, vec![]);

    suite
        .increase_eclip_incentives_lockdrop(&suite.admin(), StakeType::LpStaking, 1_000_000_000u128)
        .unwrap();
    let lp_lockup_state = suite.query_lp_lockup_state().unwrap();
    assert_eq!(
        lp_lockup_state.total_eclip_incentives.u128(),
        1_000_000_000u128
    );
    let lp_lockup_info = suite.query_lp_lockup_info().unwrap();
    assert_eq!(lp_lockup_info, vec![]);
    let alice_lp_lockup_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(alice_lp_lockup_info, vec![]);

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

    suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, ONE_YEAR)
        .unwrap();
    let lp_lockup_info = suite.query_lp_lockup_info().unwrap();
    assert_eq!(lp_lockup_info[1].duration, ONE_YEAR);
    assert_eq!(
        lp_lockup_info[1].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares / total_deposit
    );
    let alice_lp_lockup_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(alice_lp_lockup_info[1].duration, ONE_YEAR);
    assert_eq!(
        alice_lp_lockup_info[1].xastro_amount_in_lockups.u128(),
        1_000u128 * total_shares / total_deposit
    );

    let err = suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_00u128, NINE_MONTH)
        .unwrap_err();
    assert_eq!(
        ContractError::InvalidDuration(NINE_MONTH),
        err.downcast().unwrap()
    );

    let err = suite
        .lp_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, NINE_MONTH)
        .unwrap_err();
    assert_eq!(
        ContractError::InvalidDuration(NINE_MONTH),
        err.downcast().unwrap()
    );

    // update time and test duration 0 again
    suite.update_time(86400u64);

    suite
        .single_staking_increase_lockdrop(ALICE, suite.astro_contract(), 1_000u128, 0)
        .unwrap();
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info[0].duration, 0u64);
    assert_eq!(
        single_lockup_info[0].xastro_amount_in_lockups.u128(),
        3_000u128 * total_shares / total_deposit
    );
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info[0].duration, 0u64);
    assert_eq!(
        alice_single_lockup_info[0].xastro_amount_in_lockups.u128(),
        3_000u128 * total_shares / total_deposit
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

    // check withdraw during deposit window
    suite
        .single_staking_lockdrop_withdraw(ALICE, Some(Uint128::from(1000u128)), 0u64)
        .unwrap();
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info[0].duration, 0u64);
    assert_eq!(
        single_lockup_info[0].xastro_amount_in_lockups.u128(),
        3_000u128 * total_shares / total_deposit - 1_000u128
    );
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info[0].duration, 0u64);
    assert_eq!(
        alice_single_lockup_info[0].xastro_amount_in_lockups.u128(),
        3_000u128 * total_shares / total_deposit - 1_000u128
    );

    suite
        .lp_staking_lockdrop_withdraw(ALICE, Some(Uint128::from(1_000u128)), 0u64)
        .unwrap();
    let lp_lockup_info = suite.query_lp_lockup_info().unwrap();
    assert_eq!(lp_lockup_info[0].duration, 0u64);
    assert_eq!(
        lp_lockup_info[0].xastro_amount_in_lockups.u128(),
        2_000u128 * total_shares / total_deposit - 1_000u128
    );
    let alice_lp_lockup_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
    assert_eq!(alice_lp_lockup_info[0].duration, 0u64);
    assert_eq!(
        alice_lp_lockup_info[0].xastro_amount_in_lockups.u128(),
        2_000u128 * total_shares / total_deposit - 1_000u128
    );

    let alice_xastro_balance = suite.query_xastro_balance(ALICE).unwrap();
    suite
        .single_staking_lockdrop_withdraw(ALICE, None, 0u64)
        .unwrap();
    let single_lockup_info = suite.query_single_lockup_info().unwrap();
    assert_eq!(single_lockup_info[0].duration, 0u64);
    assert_eq!(single_lockup_info[0].xastro_amount_in_lockups.u128(), 0u128);
    let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
    assert_eq!(alice_single_lockup_info[0].duration, 0u64);
    assert_eq!(
        alice_single_lockup_info[0].xastro_amount_in_lockups.u128(),
        0u128
    );
    let new_alice_xastro_balance = suite.query_xastro_balance(ALICE).unwrap();
    assert_eq!(new_alice_xastro_balance - alice_xastro_balance, 1727u128);

        let alice_xastro_balance = suite.query_xastro_balance(ALICE).unwrap();
        suite
            .single_staking_lockdrop_withdraw(
                ALICE,
                Some(Uint128::from(500u128)),
                ONE_MONTH,
            )
            .unwrap();
        let single_lockup_info = suite.query_single_lockup_info().unwrap();
        assert_eq!(
            single_lockup_info[1].xastro_amount_in_lockups.u128(),
            1_000u128 + 1_000u128 * total_shares / total_deposit - 500u128
        );
        let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
        assert_eq!(
            alice_single_lockup_info[1].xastro_amount_in_lockups.u128(),
            1_000u128 + 1_000u128 * total_shares / total_deposit - 500u128
        );
        assert_eq!(alice_single_lockup_info[1].withdrawal_flag, false);
        let new_alice_xastro_balance = suite.query_xastro_balance(ALICE).unwrap();
        assert_eq!(new_alice_xastro_balance - alice_xastro_balance, 500u128);

        // check invalid amount withdraw
        let err = suite
            .single_staking_lockdrop_withdraw(
                ALICE,
                Some(Uint128::from(2_000u128)),
                ONE_MONTH,
            )
            .unwrap_err();
        assert_eq!(
            ContractError::WithdrawLimitExceed("1409".to_string()),
            err.downcast().unwrap()
        );
        // update time to withdraw window
        // deposit will fail, withdraw will only allow 50% and only once
        suite.update_time(86400u64 * 3 + 43200u64);
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

        let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
        assert_eq!(
            alice_single_lockup_info[1].xastro_amount_in_lockups.u128(),
            1409u128
        );

        let err = suite
            .single_staking_lockdrop_withdraw(
                ALICE,
                Some(Uint128::from(1_000u128)),
                ONE_MONTH,
            )
            .unwrap_err();
        assert_eq!(
            ContractError::WithdrawLimitExceed("704".to_string()),
            err.downcast().unwrap()
        );

        suite
            .single_staking_lockdrop_withdraw(ALICE, None, ONE_MONTH)
            .unwrap();
        let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
        assert_eq!(
            alice_single_lockup_info[1].xastro_amount_in_lockups.u128(),
            705u128
        );
        assert_eq!(alice_single_lockup_info[1].withdrawal_flag, true);

        let err = suite
            .single_staking_lockdrop_withdraw(ALICE, None, ONE_MONTH)
            .unwrap_err();
        assert_eq!(ContractError::AlreadyWithdrawed {}, err.downcast().unwrap());

        suite
            .lp_staking_lockdrop_withdraw(ALICE, None, 0u64)
            .unwrap();
        let alice_lp_lockup_info = suite.query_user_lp_lockup_info(ALICE).unwrap();
        assert_eq!(
            alice_lp_lockup_info[0].xastro_amount_in_lockups.u128(),
            409u128
        );
        assert_eq!(alice_lp_lockup_info[0].withdrawal_flag, true);

        let err = suite
            .lp_staking_lockdrop_withdraw(ALICE, None, 0u64)
            .unwrap_err();
        assert_eq!(ContractError::AlreadyWithdrawed {}, err.downcast().unwrap());

        suite
            .single_staking_lockdrop_withdraw(
                ALICE,
                Some(Uint128::from(300u128)),
                THREE_MONTH,
            )
            .unwrap();
        let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
        assert_eq!(
            alice_single_lockup_info[2].xastro_amount_in_lockups.u128(),
            609u128
        );
        assert_eq!(alice_single_lockup_info[2].withdrawal_flag, true);

        // update time to second half of withdrawal window
        suite.update_time(86400u64);
        let err = suite
            .single_staking_lockdrop_withdraw(
                ALICE,
                Some(Uint128::from(300u128)),
                SIX_MONTH,
            )
            .unwrap_err();
        assert_eq!(
            ContractError::WithdrawLimitExceed("227".to_string()),
            err.downcast().unwrap()
        );
        suite
            .single_staking_lockdrop_withdraw(
                ALICE,
                Some(Uint128::from(200u128)),
                SIX_MONTH,
            )
            .unwrap();
        let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
        assert_eq!(
            alice_single_lockup_info[3].xastro_amount_in_lockups.u128(),
            709u128
        );
        assert_eq!(alice_single_lockup_info[3].withdrawal_flag, true);
        suite
            .single_staking_lockdrop_withdraw(ALICE, None, ONE_YEAR)
            .unwrap();
        let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
        assert_eq!(
            alice_single_lockup_info[4].xastro_amount_in_lockups.u128(),
            682u128
        );
        assert_eq!(alice_single_lockup_info[4].withdrawal_flag, true);

        // stake all funds to single staking vault
        suite.update_time(86400u64);
        let err = suite.lockdrop_stake_single_vault(ALICE).unwrap_err();
        assert_eq!(
            ContractError::Admin(cw_controllers::AdminError::NotAdmin {}),
            err.downcast().unwrap()
        );

        let new_config = UpdateConfigMsg {
            flexible_staking: Some(suite.flexible_staking_contract()),
            timelock_staking: Some(suite.timelock_staking_contract()),
            lp_staking: Some(suite.lp_staking()),
            reward_distributor: Some(suite.reward_distributor_contract()),
        };
        suite
            .update_lockdrop_config(&suite.admin(), new_config)
            .unwrap();

        suite.lockdrop_stake_single_vault(&suite.admin()).unwrap();
        let single_staking_state = suite.query_single_lockup_state().unwrap();
        assert_eq!(single_staking_state.is_staked, true);

        assert_eq!(suite.query_voter_convert_ratio().unwrap(), (Uint128::from(2224000u128), Uint128::from(2021815u128),));

        suite.lockdrop_stake_lp_vault(&suite.admin()).unwrap();
        let lp_staking_state = suite.query_lp_lockup_state().unwrap();
        assert_eq!(lp_staking_state.is_staked, true);

        suite.lockdrop_enable_claimes(&suite.admin()).unwrap();
        let single_staking_state = suite.query_single_lockup_state().unwrap();
        assert_eq!(single_staking_state.are_claims_allowed, true);
        let lp_staking_state = suite.query_lp_lockup_state().unwrap();
        assert_eq!(lp_staking_state.are_claims_allowed, true);

        // test claim rewards and optionally withdraw
        suite.update_time(86400u64);
        // lp_staking_total_incentives 1000000000
        // single_staking_total_incentives 1000000000
        assert_eq!(
            suite
                .balance_native(suite.lockdrop(), suite.eclip())
                .unwrap(),
            2_000_000_000u128
        );
        // lp lockdrop
        // duration 0, 238, 3_657_386_363
        // duration 1 month, 523, 3_325_000_000
        // duration 1 year, 476, 1_828_409_090
        // single lockdrop
        // duration 1 month, 775, 407_894_736
        // duration 3 months, 700, 184_210_526
        // duration 6 months, 800, 210_526_315
        // duration 1 year, 750, 197_368_421

        // let alice_single_lockup_info = suite.query_user_single_lockup_info(ALICE).unwrap();
        // assert_eq!(alice_single_lockup_info, vec![]);
        suite
            .mint_native(
                suite.reward_distributor_contract(),
                suite.eclip(),
                1_000_000_000,
            )
            .unwrap();
        suite
        .mint_native(suite.lp_staking(), suite.eclip(), 1_000_000_000)
        .unwrap();
        // assert_eq!(suite.query_single_lockup_info().unwrap(), vec![]);
        // assert_eq!(suite.query_lockdrop_config().unwrap().lock_configs, vec![]);
        
        suite
            .single_lockdrop_claim_rewards_and_optionally_unlock(ALICE, 0u64, None)
            .unwrap();
        suite
            .lp_lockdrop_claim_rewards_and_optionally_unlock(ALICE, 0u64, None)
            .unwrap();
}
