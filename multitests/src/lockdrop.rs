use astroport::{asset::{Asset, AssetInfo}, vesting::{VestingAccount, VestingSchedule, VestingSchedulePoint}};
use cosmwasm_std::{Addr, Uint128};

use crate::suite::{Suite, SuiteBuilder};

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
        (NINE_MONTH, 5),
        (ONE_YEAR, 6),
    ];

    let mut suite = SuiteBuilder::new()
        .with_initial_balances(astro_initial_balances)
        .with_timelock_config(timelock_config)
        .with_eclip_daily_reward(eclip_daily_reward)
        .with_lp_staking_eclip_daily_reward(eclip_daily_reward)
        .with_locking_reward_config(locking_reward_config)
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