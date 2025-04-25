use std::str::FromStr;

use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_controllers::AdminError;
use eclipse_base::{converters::str_to_dec, voter::msg::AstroStakingRewardResponse};
use equinox_msg::{
    single_sided_staking::{
        TimeLockConfig, UpdateConfigMsg, UserReward, UserStaking, UserStakingByDuration,
    },
    utils::UNBONDING_PERIOD_0,
};
use single_sided_staking::{config::ONE_DAY, error::ContractError};

use crate::suite::{SuiteBuilder, ALICE, ATTACKER, BOB, CAROL, TREASURY};

const ONE_MONTH: u64 = 86400 * 30;
const THREE_MONTH: u64 = 86400 * 30 * 3;
const SIX_MONTH: u64 = 86400 * 30 * 6;
// const NINE_MONTH: u64 = 86400 * 30 * 9;
// const ONE_YEAR: u64 = 86400 * 365;

// #[test]
// fn instantiate() {
//     let mut suite = SuiteBuilder::new().build();
//     suite.update_config();

//     suite
//         .mint_native(ALICE.to_string(), suite.astro(), 10_000_000)
//         .unwrap();
// }

// #[test]
// fn update_config() {
//     let mut suite = SuiteBuilder::new().build();
//     suite.update_config();

//     suite
//         .mint_native(ALICE.to_string(), suite.astro(), 10_000_000)
//         .unwrap();

//     let test_config = UpdateConfigMsg {
//         timelock_config: Some(vec![TimeLockConfig {
//             duration: ONE_MONTH,
//             reward_multiplier: 1,
//         }]),
//         voter: Some("wasm1_voter".to_string()),
//         treasury: Some("wasm1_treasury".to_string()),
//         eclip: None,
//         beclip: None,
//         eclip_staking: None,
//         init_early_unlock_penalty: Some(str_to_dec("0.8")),
//     };

//     // attacker
//     let err = suite
//         .update_single_sided_stake_config(ATTACKER, test_config.clone())
//         .unwrap_err();
//     assert_eq!(
//         ContractError::Admin(AdminError::NotAdmin {}),
//         err.downcast().unwrap()
//     );

//     suite
//         .update_single_sided_stake_config(&suite.admin(), test_config.clone())
//         .unwrap();
//     let new_config = suite.query_single_sided_stake_config().unwrap();
//     assert_eq!(new_config.token, suite.eclipastro());
//     assert_eq!(
//         new_config.timelock_config,
//         vec![TimeLockConfig {
//             duration: ONE_MONTH,
//             reward_multiplier: 1,
//         }]
//     );
//     assert_eq!(new_config.treasury, Addr::unchecked("wasm1_treasury"));
//     assert_eq!(new_config.voter, Addr::unchecked("wasm1_voter"));
//     assert_eq!(
//         new_config.init_early_unlock_penalty,
//         Decimal::from_str("0.8").unwrap()
//     );
// }

// #[test]
// fn stake() {
//     let mut suite = SuiteBuilder::new().build();
//     suite.update_config();

//     // add funds to vault
//     suite
//         .add_single_sided_vault_reward(
//             &suite.admin(),
//             None,
//             None,
//             12_800_000_000u128,
//             8_600_000_000u128,
//         )
//         .unwrap();

//     suite
//         .mint_native(BOB.to_string(), suite.astro(), 10_000)
//         .unwrap();

//     // ready astro_staking_pool
//     suite.stake_astro(&suite.admin(), 1_000_000).unwrap();

//     // bob converts 1_000 astro and get 1_000 eclipAstro
//     suite.convert_astro(BOB, 1_000).unwrap();
//     let err = suite.single_sided_stake(BOB, 1_000, 10, None).unwrap_err();
//     assert_eq!(
//         ContractError::NoLockingPeriodFound(10),
//         err.downcast().unwrap()
//     );
//     suite.single_sided_stake(BOB, 100, ONE_MONTH, None).unwrap();
//     assert_eq!(
//         suite.query_single_sided_staking(BOB).unwrap(),
//         vec![UserStaking {
//             duration: ONE_MONTH,
//             staking: vec![UserStakingByDuration {
//                 amount: Uint128::from(100u128),
//                 locked_at: suite.get_time()
//             }]
//         }]
//     );
//     assert_eq!(suite.query_single_sided_total_staking().unwrap(), 100);

//     suite.single_sided_stake(BOB, 100, ONE_MONTH, None).unwrap();
//     assert_eq!(
//         suite.query_single_sided_staking(BOB).unwrap(),
//         vec![UserStaking {
//             duration: ONE_MONTH,
//             staking: vec![UserStakingByDuration {
//                 amount: Uint128::from(200u128),
//                 locked_at: suite.get_time()
//             }]
//         }]
//     );
//     assert_eq!(suite.query_single_sided_total_staking().unwrap(), 200);

//     suite
//         .single_sided_stake(BOB, 100, THREE_MONTH, None)
//         .unwrap();
//     let mut bob_staking = vec![
//         UserStaking {
//             duration: ONE_MONTH,
//             staking: vec![UserStakingByDuration {
//                 amount: Uint128::from(200u128),
//                 locked_at: suite.get_time(),
//             }],
//         },
//         UserStaking {
//             duration: THREE_MONTH,
//             staking: vec![UserStakingByDuration {
//                 amount: Uint128::from(100u128),
//                 locked_at: suite.get_time(),
//             }],
//         },
//     ];
//     assert_eq!(suite.query_single_sided_staking(BOB).unwrap(), bob_staking);
//     assert_eq!(suite.query_single_sided_total_staking().unwrap(), 300);

//     suite.single_sided_stake(BOB, 100, 0, None).unwrap();
//     assert_eq!(
//         suite.query_single_sided_staking(BOB).unwrap(),
//         vec![
//             UserStaking {
//                 duration: 0,
//                 staking: vec![UserStakingByDuration {
//                     amount: Uint128::from(100u128),
//                     locked_at: 0u64,
//                 }],
//             },
//             UserStaking {
//                 duration: ONE_MONTH,
//                 staking: vec![UserStakingByDuration {
//                     amount: Uint128::from(200u128),
//                     locked_at: suite.get_time(),
//                 }],
//             },
//             UserStaking {
//                 duration: THREE_MONTH,
//                 staking: vec![UserStakingByDuration {
//                     amount: Uint128::from(100u128),
//                     locked_at: suite.get_time(),
//                 }],
//             },
//         ]
//     );
//     assert_eq!(suite.query_single_sided_total_staking().unwrap(), 400);

//     suite.update_time(THREE_MONTH);

//     suite
//         .single_sided_stake(BOB, 100, THREE_MONTH, None)
//         .unwrap();
//     bob_staking = vec![
//         UserStaking {
//             duration: 0,
//             staking: vec![UserStakingByDuration {
//                 amount: Uint128::from(100u128),
//                 locked_at: 0u64,
//             }],
//         },
//         UserStaking {
//             duration: ONE_MONTH,
//             staking: vec![UserStakingByDuration {
//                 amount: Uint128::from(200u128),
//                 locked_at: suite.get_time() - THREE_MONTH,
//             }],
//         },
//         UserStaking {
//             duration: THREE_MONTH,
//             staking: vec![
//                 UserStakingByDuration {
//                     amount: Uint128::from(100u128),
//                     locked_at: suite.get_time() - THREE_MONTH,
//                 },
//                 UserStakingByDuration {
//                     amount: Uint128::from(100u128),
//                     locked_at: suite.get_time(),
//                 },
//             ],
//         },
//     ];
//     assert_eq!(suite.query_single_sided_staking(BOB).unwrap(), bob_staking);
//     assert_eq!(suite.query_single_sided_total_staking().unwrap(), 500);
//     let time = suite.get_time();
//     let err = suite
//         .single_sided_unstake(BOB, THREE_MONTH, time - THREE_MONTH, None, None)
//         .unwrap_err();
//     assert_eq!(
//         ContractError::EarlyUnlockDisabled {},
//         err.downcast().unwrap()
//     );
//     bob_staking = vec![
//         UserStaking {
//             duration: 0,
//             staking: vec![UserStakingByDuration {
//                 amount: Uint128::from(100u128),
//                 locked_at: 0u64,
//             }],
//         },
//         UserStaking {
//             duration: ONE_MONTH,
//             staking: vec![UserStakingByDuration {
//                 amount: Uint128::from(200u128),
//                 locked_at: suite.get_time() - THREE_MONTH,
//             }],
//         },
//         UserStaking {
//             duration: THREE_MONTH,
//             staking: vec![
//                 UserStakingByDuration {
//                     amount: Uint128::from(100u128),
//                     locked_at: suite.get_time() - THREE_MONTH,
//                 },
//                 UserStakingByDuration {
//                     amount: Uint128::from(100u128),
//                     locked_at: suite.get_time(),
//                 },
//             ],
//         },
//     ];
//     assert_eq!(suite.query_single_sided_staking(BOB).unwrap(), bob_staking);
//     assert_eq!(suite.query_single_sided_total_staking().unwrap(), 500);
//     assert_eq!(suite.query_eclipastro_balance(BOB).unwrap(), 500);
//     suite
//         .single_sided_restake(BOB, ONE_MONTH, time - THREE_MONTH, SIX_MONTH, None, None)
//         .unwrap();
//     bob_staking = vec![
//         UserStaking {
//             duration: 0,
//             staking: vec![UserStakingByDuration {
//                 amount: Uint128::from(100u128),
//                 locked_at: 0u64,
//             }],
//         },
//         UserStaking {
//             duration: THREE_MONTH,
//             staking: vec![
//                 UserStakingByDuration {
//                     amount: Uint128::from(100u128),
//                     locked_at: suite.get_time() - THREE_MONTH,
//                 },
//                 UserStakingByDuration {
//                     amount: Uint128::from(100u128),
//                     locked_at: suite.get_time(),
//                 },
//             ],
//         },
//         UserStaking {
//             duration: SIX_MONTH,
//             staking: vec![UserStakingByDuration {
//                 amount: Uint128::from(200u128),
//                 locked_at: suite.get_time(),
//             }],
//         },
//     ];
//     assert_eq!(suite.query_single_sided_staking(BOB).unwrap(), bob_staking);
// }

// #[test]
// fn claim() {
//     let mut suite = SuiteBuilder::new().build();
//     suite.update_config();

//     // add funds to vault
//     suite
//         .add_single_sided_vault_reward(
//             &suite.admin(),
//             None,
//             None,
//             12_800_000_000u128,
//             8_600_000_000u128,
//         )
//         .unwrap();

//     suite
//         .mint_native(BOB.to_string(), suite.astro(), 1_000)
//         .unwrap();

//     suite
//         .mint_native(ALICE.to_string(), suite.astro(), 1_000)
//         .unwrap();

//     // ready astro_staking_pool
//     suite.stake_astro(&suite.admin(), 1_000_000).unwrap();

//     // Bob converts 1_000 astro and stake it
//     suite.convert_astro(BOB, 1_000).unwrap();
//     assert_eq!(suite.query_eclipastro_balance(BOB).unwrap(), 1_000);
//     suite.single_sided_stake(BOB, 100, ONE_MONTH, None).unwrap();

//     suite.single_sided_stake(BOB, 100, 0, None).unwrap();
//     let init_time = suite.get_time();

//     // check initial reward is zero
//     assert_eq!(
//         suite
//             .query_single_sided_staking_reward(BOB, ONE_MONTH, init_time)
//             .unwrap(),
//         UserReward {
//             beclip: Uint128::zero(),
//             eclipastro: Uint128::zero(),
//             eclip: Uint128::zero()
//         }
//     );

//     // change astro/xastro ratio and check balances and rewards
//     suite
//         .mint_native(suite.astro_staking_contract(), suite.astro(), 100_000)
//         .unwrap();
//     assert_eq!(
//         suite
//             .query_balance_native(suite.voter_contract(), suite.xastro())
//             .unwrap(),
//         999
//     );

//     assert_eq!(
//         suite.query_voter_astro_staking_rewards().unwrap(),
//         AstroStakingRewardResponse {
//             users: Uint128::from(76u128),
//             treasury: Uint128::from(19u128)
//         }
//     );

//     // change time and check eclip rewards
//     suite.update_time(43200);

//     assert_eq!(
//         suite
//             .query_single_sided_staking_reward(BOB, ONE_MONTH, init_time)
//             .unwrap(),
//         UserReward {
//             beclip: Uint128::from(95555555u128),
//             eclipastro: Uint128::zero(),
//             eclip: Uint128::from(142222222u128)
//         }
//     );
//     assert_eq!(
//         suite.query_single_sided_staking_reward(BOB, 0, 0).unwrap(),
//         UserReward {
//             beclip: Uint128::from(47777777u128),
//             eclipastro: Uint128::zero(),
//             eclip: Uint128::from(71111111u128)
//         }
//     );

//     suite.single_stake_claim(BOB, 0, 0, None).unwrap();
//     assert_eq!(
//         suite
//             .query_balance_native(BOB.to_string(), suite.eclip())
//             .unwrap(),
//         71111111
//     );
//     assert_eq!(suite.query_beclip_balance(BOB).unwrap(), 47777777);
//     suite.single_stake_claim(BOB, 0, 0, None).unwrap();
//     assert_eq!(
//         suite
//             .query_balance_native(BOB.to_string(), suite.eclip())
//             .unwrap(),
//         71111111
//     );
//     assert_eq!(
//         suite.query_single_sided_staking_reward(BOB, 0, 0).unwrap(),
//         UserReward {
//             beclip: Uint128::from(0u128),
//             eclipastro: Uint128::zero(),
//             eclip: Uint128::from(0u128)
//         }
//     );

//     // staking will withdraw current rewards
//     // convert users xastro reward to eclipastro, and sends it to reward_distributor contract
//     // send stability pool reward and ce_holders reward as xastro
//     // total xastro amount will be reduced and pending rewards are zero. treasury rewards are not claimed.
//     suite.update_time(THREE_MONTH);
//     suite.single_stake_claim_all(BOB, true, None).unwrap();
//     assert_eq!(
//         suite
//             .query_balance_native(BOB.to_string(), suite.eclip())
//             .unwrap(),
//         12799999978
//     );
// }

// #[test]
// fn blacklist() {
//     let mut suite = SuiteBuilder::new().build();
//     suite.update_config();

//     // add funds to vault
//     suite
//         .add_single_sided_vault_reward(
//             &suite.admin(),
//             None,
//             None,
//             12_800_000_000u128,
//             8_600_000_000u128,
//         )
//         .unwrap();

//     suite
//         .mint_native(CAROL.to_string(), suite.astro(), 1_000)
//         .unwrap();

//     // ready astro_staking_pool
//     suite.stake_astro(&suite.admin(), 1_000_000).unwrap();

//     // Bob converts 1_000 astro and stake it
//     suite.convert_astro(CAROL, 1_000).unwrap();
//     assert_eq!(suite.query_eclipastro_balance(CAROL).unwrap(), 1_000);
//     suite
//         .single_sided_stake(CAROL, 100, ONE_MONTH, None)
//         .unwrap();

//     suite.single_sided_stake(CAROL, 100, 0, None).unwrap();
//     let init_time = suite.get_time();

//     // check initial reward is zero
//     assert_eq!(
//         suite
//             .query_single_sided_staking_reward(CAROL, ONE_MONTH, init_time)
//             .unwrap(),
//         UserReward {
//             beclip: Uint128::zero(),
//             eclipastro: Uint128::zero(),
//             eclip: Uint128::zero()
//         }
//     );

//     // change astro/xastro ratio and check balances and rewards
//     suite
//         .mint_native(suite.astro_staking_contract(), suite.astro(), 100_000)
//         .unwrap();
//     assert_eq!(
//         suite
//             .query_balance_native(suite.voter_contract(), suite.xastro())
//             .unwrap(),
//         999
//     );

//     assert_eq!(
//         suite.query_voter_astro_staking_rewards().unwrap(),
//         AstroStakingRewardResponse {
//             users: Uint128::from(76u128),
//             treasury: Uint128::from(19u128)
//         }
//     );

//     // change time and check eclip rewards
//     suite.update_time(43200);

//     assert_eq!(
//         suite
//             .query_single_sided_staking_reward(CAROL, ONE_MONTH, init_time)
//             .unwrap(),
//         UserReward {
//             beclip: Uint128::zero(),
//             eclipastro: Uint128::zero(),
//             eclip: Uint128::zero()
//         }
//     );
//     assert_eq!(
//         suite
//             .query_single_sided_staking_reward(CAROL, 0, 0)
//             .unwrap(),
//         UserReward {
//             beclip: Uint128::zero(),
//             eclipastro: Uint128::zero(),
//             eclip: Uint128::zero()
//         }
//     );

//     let err = suite.single_stake_claim(CAROL, 0, 0, None).unwrap_err();
//     assert_eq!(ContractError::Blacklisted {}, err.downcast().unwrap());
//     assert_eq!(
//         suite.query_single_sided_blacklisted_reward().unwrap(),
//         UserReward {
//             beclip: Uint128::from(143333332u128),
//             eclipastro: Uint128::zero(),
//             eclip: Uint128::from(213333333u128)
//         }
//     );

//     suite.single_blacklist_claim().unwrap();
//     assert_eq!(
//         suite
//             .query_balance_native(TREASURY.to_string(), suite.eclip())
//             .unwrap(),
//         356666665u128
//     );
// }

#[test]
fn stake_default() {
    let mut suite = SuiteBuilder::new().build();
    suite.update_config();

    // add funds to vault
    suite
        .add_single_sided_vault_reward(
            &suite.admin(),
            None,
            None,
            12_800_000_000u128,
            8_600_000_000u128,
        )
        .unwrap();

    suite
        .mint_native(BOB.to_string(), suite.astro(), 10_000)
        .unwrap();

    // ready astro_staking_pool
    suite.stake_astro(&suite.admin(), 1_000_000).unwrap();

    // bob converts 1_000 astro and get 1_000 eclipAstro
    suite.convert_astro(BOB, 1_000).unwrap();

    println!("++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++");

    // pub fn calculate_lock_end_time(duration: u64, locked_at: u64) -> u64 {
    //     (duration + locked_at) / ONE_DAY * ONE_DAY + ONE_DAY
    // }

    // sync date
    // let block_time = suite.get_time();
    // let offset = ONE_DAY * (1 + block_time / ONE_DAY) - block_time;
    // suite.update_time(offset);
    let block_time = suite.get_time();

    suite.single_sided_stake(BOB, 100, ONE_MONTH, None).unwrap();
    assert_eq!(
        suite.query_single_sided_staking(BOB).unwrap(),
        vec![UserStaking {
            duration: ONE_MONTH,
            staking: vec![UserStakingByDuration {
                amount: Uint128::new(100),
                locked_at: block_time
            }]
        }]
    );

    suite.update_time(ONE_MONTH + ONE_DAY);
    suite
        .single_sided_unbond(BOB, ONE_MONTH, block_time, UNBONDING_PERIOD_0)
        .unwrap();

    suite.update_time(UNBONDING_PERIOD_0);
    suite.single_sided_withdraw(BOB, None).unwrap();

    // -----------------------------------------------------------------------------------------

    // suite.single_sided_stake(BOB, 100, ONE_MONTH, None).unwrap();
    // assert_eq!(
    //     suite.query_single_sided_staking(BOB).unwrap(),
    //     vec![UserStaking {
    //         duration: ONE_MONTH,
    //         staking: vec![UserStakingByDuration {
    //             amount: Uint128::from(200u128),
    //             locked_at: suite.get_time()
    //         }]
    //     }]
    // );
    // assert_eq!(suite.query_single_sided_total_staking().unwrap(), 200);

    // suite
    //     .single_sided_stake(BOB, 100, THREE_MONTH, None)
    //     .unwrap();
    // let mut bob_staking = vec![
    //     UserStaking {
    //         duration: ONE_MONTH,
    //         staking: vec![UserStakingByDuration {
    //             amount: Uint128::from(200u128),
    //             locked_at: suite.get_time(),
    //         }],
    //     },
    //     UserStaking {
    //         duration: THREE_MONTH,
    //         staking: vec![UserStakingByDuration {
    //             amount: Uint128::from(100u128),
    //             locked_at: suite.get_time(),
    //         }],
    //     },
    // ];
    // assert_eq!(suite.query_single_sided_staking(BOB).unwrap(), bob_staking);
    // assert_eq!(suite.query_single_sided_total_staking().unwrap(), 300);

    // suite.single_sided_stake(BOB, 100, 0, None).unwrap();
    // assert_eq!(
    //     suite.query_single_sided_staking(BOB).unwrap(),
    //     vec![
    //         UserStaking {
    //             duration: 0,
    //             staking: vec![UserStakingByDuration {
    //                 amount: Uint128::from(100u128),
    //                 locked_at: 0u64,
    //             }],
    //         },
    //         UserStaking {
    //             duration: ONE_MONTH,
    //             staking: vec![UserStakingByDuration {
    //                 amount: Uint128::from(200u128),
    //                 locked_at: suite.get_time(),
    //             }],
    //         },
    //         UserStaking {
    //             duration: THREE_MONTH,
    //             staking: vec![UserStakingByDuration {
    //                 amount: Uint128::from(100u128),
    //                 locked_at: suite.get_time(),
    //             }],
    //         },
    //     ]
    // );
    // assert_eq!(suite.query_single_sided_total_staking().unwrap(), 400);

    // suite.update_time(THREE_MONTH);

    // suite
    //     .single_sided_stake(BOB, 100, THREE_MONTH, None)
    //     .unwrap();
    // bob_staking = vec![
    //     UserStaking {
    //         duration: 0,
    //         staking: vec![UserStakingByDuration {
    //             amount: Uint128::from(100u128),
    //             locked_at: 0u64,
    //         }],
    //     },
    //     UserStaking {
    //         duration: ONE_MONTH,
    //         staking: vec![UserStakingByDuration {
    //             amount: Uint128::from(200u128),
    //             locked_at: suite.get_time() - THREE_MONTH,
    //         }],
    //     },
    //     UserStaking {
    //         duration: THREE_MONTH,
    //         staking: vec![
    //             UserStakingByDuration {
    //                 amount: Uint128::from(100u128),
    //                 locked_at: suite.get_time() - THREE_MONTH,
    //             },
    //             UserStakingByDuration {
    //                 amount: Uint128::from(100u128),
    //                 locked_at: suite.get_time(),
    //             },
    //         ],
    //     },
    // ];
    // assert_eq!(suite.query_single_sided_staking(BOB).unwrap(), bob_staking);
    // assert_eq!(suite.query_single_sided_total_staking().unwrap(), 500);
    // let time = suite.get_time();
    // let err = suite
    //     .single_sided_unstake(BOB, THREE_MONTH, time - THREE_MONTH, None, None)
    //     .unwrap_err();
    // assert_eq!(
    //     ContractError::EarlyUnlockDisabled {},
    //     err.downcast().unwrap()
    // );
    // bob_staking = vec![
    //     UserStaking {
    //         duration: 0,
    //         staking: vec![UserStakingByDuration {
    //             amount: Uint128::from(100u128),
    //             locked_at: 0u64,
    //         }],
    //     },
    //     UserStaking {
    //         duration: ONE_MONTH,
    //         staking: vec![UserStakingByDuration {
    //             amount: Uint128::from(200u128),
    //             locked_at: suite.get_time() - THREE_MONTH,
    //         }],
    //     },
    //     UserStaking {
    //         duration: THREE_MONTH,
    //         staking: vec![
    //             UserStakingByDuration {
    //                 amount: Uint128::from(100u128),
    //                 locked_at: suite.get_time() - THREE_MONTH,
    //             },
    //             UserStakingByDuration {
    //                 amount: Uint128::from(100u128),
    //                 locked_at: suite.get_time(),
    //             },
    //         ],
    //     },
    // ];
    // assert_eq!(suite.query_single_sided_staking(BOB).unwrap(), bob_staking);
    // assert_eq!(suite.query_single_sided_total_staking().unwrap(), 500);
    // assert_eq!(suite.query_eclipastro_balance(BOB).unwrap(), 500);
    // suite
    //     .single_sided_restake(BOB, ONE_MONTH, time - THREE_MONTH, SIX_MONTH, None, None)
    //     .unwrap();
    // bob_staking = vec![
    //     UserStaking {
    //         duration: 0,
    //         staking: vec![UserStakingByDuration {
    //             amount: Uint128::from(100u128),
    //             locked_at: 0u64,
    //         }],
    //     },
    //     UserStaking {
    //         duration: THREE_MONTH,
    //         staking: vec![
    //             UserStakingByDuration {
    //                 amount: Uint128::from(100u128),
    //                 locked_at: suite.get_time() - THREE_MONTH,
    //             },
    //             UserStakingByDuration {
    //                 amount: Uint128::from(100u128),
    //                 locked_at: suite.get_time(),
    //             },
    //         ],
    //     },
    //     UserStaking {
    //         duration: SIX_MONTH,
    //         staking: vec![UserStakingByDuration {
    //             amount: Uint128::from(200u128),
    //             locked_at: suite.get_time(),
    //         }],
    //     },
    // ];
    // assert_eq!(suite.query_single_sided_staking(BOB).unwrap(), bob_staking);
}
