// use cosmwasm_std::{Addr, Decimal256, Uint128};
// use cw_controllers::AdminError;
// use equinox_msg::{
//     flexible_staking::{Config, UpdateConfigMsg},
//     reward_distributor::FlexibleReward,
//     token_converter::{Reward, RewardResponse},
// };
// use flexible_staking::error::ContractError;

// use crate::suite::{Suite, SuiteBuilder};

// const ONE_MONTH: u64 = 86400 * 30;
// const THREE_MONTH: u64 = 86400 * 30 * 3;
// const SIX_MONTH: u64 = 86400 * 30 * 6;
// const NINE_MONTH: u64 = 86400 * 30 * 9;
// const ONE_YEAR: u64 = 86400 * 365;

// const ALICE: &str = "alice";
// const BOB: &str = "bob";
// const CAROL: &str = "carol";
// const ATTACKER: &str = "attacker";
// // const VICTIM: &str = "victim";

// pub struct Instantiate<'a> {
//     pub astro_staking_initiator: &'a str,
//     pub astro_initial_balances: Vec<(&'a str, u128)>,
//     pub timelock_config: Vec<(u64, u16)>,
//     pub eclip_daily_reward: u128,
//     pub locking_reward_config: Vec<(u64, u64)>,
//     pub suite: Suite,
// }

// pub fn instantiate() -> Instantiate<'static> {
//     let astro_staking_initiator = "astro_staking_initiator";
//     let astro_initial_balances = vec![
//         (astro_staking_initiator, 1_000_000_000),
//         (ALICE, 1_000_000),
//         (BOB, 1_000_000),
//         (CAROL, 1_000_000),
//     ];
//     let timelock_config = vec![
//         (ONE_MONTH, 100),
//         (THREE_MONTH, 100),
//         (SIX_MONTH, 100),
//         (NINE_MONTH, 100),
//         (ONE_YEAR, 100),
//     ];
//     let eclip_daily_reward: u128 = 1_000_000u128;
//     let locking_reward_config = vec![
//         (0, 1),
//         (ONE_MONTH, 2),
//         (THREE_MONTH, 3),
//         (SIX_MONTH, 4),
//         (NINE_MONTH, 5),
//         (ONE_YEAR, 6),
//     ];

//     let mut suite = SuiteBuilder::new()
//         .with_initial_balances(astro_initial_balances.clone())
//         .with_timelock_config(timelock_config.clone())
//         .with_eclip_daily_reward(eclip_daily_reward)
//         .with_locking_reward_config(locking_reward_config.clone())
//         .build();

//     suite.update_config();

//     let config = suite.query_flexible_stake_config().unwrap();
//     assert_eq!(
//         config,
//         Config {
//             token: Addr::unchecked(suite.eclipastro_contract()),
//             reward_contract: Addr::unchecked(suite.reward_distributor_contract()),
//         }
//     );
//     return Instantiate {
//         astro_staking_initiator,
//         astro_initial_balances,
//         timelock_config,
//         eclip_daily_reward,
//         locking_reward_config,
//         suite,
//     };
// }

// #[test]
// fn update_config() {
//     let instantiate_data = instantiate();
//     let mut suite = instantiate_data.suite;

//     let test_config = UpdateConfigMsg {
//         token: Some(Addr::unchecked("test").to_string()),
//         reward_contract: Some(Addr::unchecked("test").to_string()),
//     };

//     // attacker can't change config
//     let err = suite
//         .update_flexible_stake_config(ATTACKER, test_config.clone())
//         .unwrap_err();
//     assert_eq!(
//         ContractError::Admin(AdminError::NotAdmin {}),
//         err.downcast().unwrap()
//     );

//     // admin can change config
//     suite
//         .update_flexible_stake_config(&suite.admin(), test_config)
//         .unwrap();

//     // check update config is successed
//     assert_eq!(
//         suite.query_flexible_stake_config().unwrap(),
//         Config {
//             token: Addr::unchecked("test"),
//             reward_contract: Addr::unchecked("test"),
//         }
//     );
// }

// #[test]
// fn update_owner() {
//     let instantiate_data = instantiate();
//     let mut suite = instantiate_data.suite;

//     // attacker can't change owner
//     let err = suite
//         .update_flexible_stake_owner(ATTACKER, ATTACKER)
//         .unwrap_err();
//     assert_eq!(
//         ContractError::Admin(AdminError::NotAdmin {}),
//         err.downcast().unwrap()
//     );

//     suite
//         .update_flexible_stake_owner(&suite.admin(), ALICE)
//         .unwrap();
//     assert_eq!(suite.query_flexible_stake_owner().unwrap(), ALICE);
// }

// #[test]
// fn single_stake() {
//     let instantiate_data = instantiate();
//     let mut suite = instantiate_data.suite;
//     let astro_staking_initiator = instantiate_data.astro_staking_initiator;
//     let mut total_astro_deposit = 0;
//     let mut total_xastro_shares = 0;
//     let mut alice_flexible_staking = 0;
//     let mut total_flexible_staking = 0;
//     let mut flexible_stake_reward = FlexibleReward {
//         eclip: Uint128::zero(),
//         eclipastro: Uint128::zero(),
//     };

//     // ready astro_staking_pool
//     let astro_amount = 1_000_000;
//     suite
//         .stake_astro(astro_staking_initiator, astro_amount)
//         .unwrap();
//     total_astro_deposit = total_astro_deposit + astro_amount;
//     total_xastro_shares = total_xastro_shares + astro_amount;
//     assert_eq!(
//         suite.query_flexible_staking(ALICE).unwrap(),
//         alice_flexible_staking
//     );
//     assert_eq!(
//         suite.query_total_flexible_staking().unwrap(),
//         total_flexible_staking
//     );

//     // alice converts 1_000 astro and stakes it
//     let alice_flexible_stake_amount = 1_000;
//     suite
//         .convert_astro(ALICE, alice_flexible_stake_amount)
//         .unwrap();
//     suite
//         .flexible_stake(ALICE, alice_flexible_stake_amount)
//         .unwrap();
//     total_astro_deposit = total_astro_deposit + alice_flexible_stake_amount;
//     total_xastro_shares = total_xastro_shares + alice_flexible_stake_amount;
//     alice_flexible_staking = alice_flexible_staking + alice_flexible_stake_amount;
//     total_flexible_staking = total_flexible_staking + alice_flexible_stake_amount;
//     assert_eq!(
//         suite.query_flexible_staking(ALICE).unwrap(),
//         alice_flexible_staking
//     );
//     assert_eq!(
//         suite.query_total_flexible_staking().unwrap(),
//         total_flexible_staking
//     );
//     assert_eq!(
//         suite.query_flexible_stake_reward(ALICE).unwrap(),
//         flexible_stake_reward
//     );

//     // change ASTRO/xASTRO rate
//     let amount = 100_000;
//     suite
//         .mint_native(
//             suite.reward_distributor_contract(),
//             suite.eclip(),
//             amount,
//         )
//         .unwrap();
//     total_astro_deposit = total_astro_deposit + amount;

//     assert_eq!(
//         suite.query_flexible_stake_reward(ALICE).unwrap(),
//         flexible_stake_reward
//     );
//     assert_eq!(
//         suite.query_converter_reward().unwrap(),
//         RewardResponse {
//             users_reward: Reward {
//                 token: suite.eclipastro_contract(),
//                 amount: Uint128::from(791u128)
//             },
//             ce_holders_reward: Reward {
//                 token: suite.xastro_contract(),
//                 amount: Uint128::from(36u128)
//             },
//             stability_pool_reward: Reward {
//                 token: suite.xastro_contract(),
//                 amount: Uint128::from(22u128)
//             },
//             treasury_reward: Reward {
//                 token: suite.xastro_contract(),
//                 amount: Uint128::from(122u128)
//             },
//         }
//     );
// }

// #[test]
// fn stake() {
//     let instantiate_data = instantiate();
//     let mut suite = instantiate_data.suite;
//     let astro_staking_initiator = instantiate_data.astro_staking_initiator;
//     let mut total_astro_staking = 0;
//     let mut alice_flexible_staking = 0;

//     // ready astro_staking_pool
//     let astro_amount = 1_000_000;
//     suite
//         .stake_astro(astro_staking_initiator, astro_amount)
//         .unwrap();
//     total_astro_staking = total_astro_staking + astro_amount;

//     assert_eq!(suite.query_flexible_staking(ALICE).unwrap(), 0);
//     assert_eq!(suite.query_total_flexible_staking().unwrap(), 0);
//     // alice converts 1_000 astro and stakes it
//     let alice_flexible_stake_amount = 1_000;
//     suite
//         .convert_astro(ALICE, alice_flexible_stake_amount)
//         .unwrap();
//     suite
//         .flexible_stake(ALICE, alice_flexible_stake_amount)
//         .unwrap();
//     alice_flexible_staking = alice_flexible_staking + alice_flexible_stake_amount;
//     // check alice's staking and total staking
//     assert_eq!(suite.query_flexible_staking(ALICE).unwrap(), 1_000);
//     assert_eq!(suite.query_total_flexible_staking().unwrap(), 1_000);

//     // alice converts 1_000 more astro and stakes it
//     suite.convert_astro(ALICE, 1_000).unwrap();
//     suite.flexible_stake(ALICE, 1_000).unwrap();
//     assert_eq!(suite.query_flexible_staking(ALICE).unwrap(), 2_000);
//     assert_eq!(suite.query_total_flexible_staking().unwrap(), 2_000);

//     // bob converts 1_000 astro and stakes it
//     suite.convert_astro(BOB, 1_000).unwrap();
//     suite.flexible_stake(BOB, 1_000).unwrap();
//     assert_eq!(suite.query_flexible_staking(BOB).unwrap(), 1_000);
//     assert_eq!(suite.query_total_flexible_staking().unwrap(), 3_000);

//     // bob converts 3_000 more astro and stakes it
//     suite.convert_astro(BOB, 3_000).unwrap();
//     suite.flexible_stake(BOB, 3_000).unwrap();
//     assert_eq!(suite.query_flexible_staking(BOB).unwrap(), 4_000);
//     assert_eq!(suite.query_total_flexible_staking().unwrap(), 6_000);

//     // bob converts 3_000 more astro and stakes it
//     suite.convert_astro(BOB, 3_000).unwrap();
//     suite.flexible_stake(BOB, 3_000).unwrap();
//     assert_eq!(suite.query_flexible_staking(BOB).unwrap(), 7_000);
//     assert_eq!(suite.query_total_flexible_staking().unwrap(), 9_000);

//     // bob unstakes 1_000 eclipASTRO
//     suite.flexible_unstake(BOB, 1_000).unwrap();
//     assert_eq!(suite.query_flexible_staking(BOB).unwrap(), 6_000);
//     assert_eq!(suite.query_total_flexible_staking().unwrap(), 8_000);

//     // bob unstakes 2_000 eclipASTRO
//     suite.flexible_unstake(BOB, 2_000).unwrap();
//     assert_eq!(suite.query_flexible_staking(BOB).unwrap(), 4_000);
//     assert_eq!(suite.query_total_flexible_staking().unwrap(), 6_000);

//     // bob stakes 1_000 eclipASTRO
//     suite.flexible_stake(BOB, 1_000).unwrap();
//     assert_eq!(suite.query_flexible_staking(BOB).unwrap(), 5_000);
//     assert_eq!(suite.query_total_flexible_staking().unwrap(), 7_000);
// }

// #[test]
// fn claim() {
//     let astro_staking_initiator = "astro_staking_initiator";
//     let astro_initial_balances = vec![
//         (astro_staking_initiator, 1_000_000_000),
//         (ALICE, 1_000_000),
//         (BOB, 1_000_000),
//         (CAROL, 1_000_000),
//         (ATTACKER, 1_000),
//     ];
//     let timelock_config = vec![
//         (ONE_MONTH, 100),
//         (THREE_MONTH, 100),
//         (SIX_MONTH, 100),
//         (NINE_MONTH, 100),
//         (ONE_YEAR, 100),
//     ];
//     let eclip_daily_reward = 1_000_000;
//     let locking_reward_config = vec![
//         (0, 1),
//         (ONE_MONTH, 2),
//         (THREE_MONTH, 3),
//         (SIX_MONTH, 4),
//         (NINE_MONTH, 5),
//         (ONE_YEAR, 6),
//     ];

//     let mut suite = SuiteBuilder::new()
//         .with_initial_balances(astro_initial_balances)
//         .with_timelock_config(timelock_config)
//         .with_eclip_daily_reward(eclip_daily_reward)
//         .with_locking_reward_config(locking_reward_config)
//         .build();

//     suite.update_config();

//     // ready astro_staking_pool
//     suite
//         .stake_astro(astro_staking_initiator, 1_000_000)
//         .unwrap();

//     // mint eclip
//     suite
//         .mint_native(
//             suite.reward_distributor_contract(),
//             suite.eclip(),
//             1_000_000_000,
//         )
//         .unwrap();

//     // alice converts 3_000 astro and stake it
//     suite.convert_astro(ALICE, 3_000).unwrap();
//     suite.flexible_stake(ALICE, 3_000).unwrap();

//     // bob converts 7000 astro and stake it
//     suite.convert_astro(BOB, 7_000).unwrap();
//     suite.flexible_stake(BOB, 7_000).unwrap();

//     assert_eq!(
//         suite.query_reward_distributor_pending_rewards().unwrap(),
//         vec![]
//     );

//     // check initial reward is zero
//     assert_eq!(
//         suite.query_flexible_stake_reward(ALICE).unwrap(),
//         FlexibleReward {
//             eclip: Uint128::zero(),
//             eclipastro: Uint128::zero(),
//         }
//     );

//     // change astro/xastro ratio and check balances and rewards
//     suite
//         .send_astro(
//             astro_staking_initiator,
//             &suite.astro_staking_contract(),
//             100_000,
//         )
//         .unwrap();

//     assert_eq!(
//         suite.query_astro_staking_total_deposit().unwrap(),
//         1_110_000
//     );
//     assert_eq!(suite.query_astro_staking_total_shares().unwrap(), 1_010_000);
//     assert_eq!(
//         suite
//             .query_xastro_balance(suite.voter_contract().as_str())
//             .unwrap(),
//         10_000
//     );

//     // xASTRO rewards = (staked_total_xastro * total_deposit / total_shares - staked_astro) * total_shares / total_deposit - claimed_xASTRO
//     // (10_000 * 1_110_000 / 1_010_000 - 10_000) * 1_010_000 / 1_110_000 - 0 = 900 xASTRO
//     // user's reward = 900 * 0.8 * 1_110_000 / 1_010_000 = 720 * ~ = 791 eclipASTRO
//     // ce_holders_reward = (900 - 720) * 0.2 = 36
//     // stability_pool_reward = (900 - 720) * 0.125 = 22
//     // treasury_reward = 900 - 720 - 36 - 22 = 122
//     assert_eq!(
//         suite.query_converter_reward().unwrap(),
//         RewardResponse {
//             users_reward: Reward {
//                 token: suite.eclipastro_contract(),
//                 amount: Uint128::from(791u128)
//             },
//             ce_holders_reward: Reward {
//                 token: suite.xastro_contract(),
//                 amount: Uint128::from(36u128)
//             },
//             stability_pool_reward: Reward {
//                 token: suite.xastro_contract(),
//                 amount: Uint128::from(22u128)
//             },
//             treasury_reward: Reward {
//                 token: suite.xastro_contract(),
//                 amount: Uint128::from(122u128)
//             },
//         }
//     );
//     // ALICE and BOB staked tokens but time not passed so current eclipastro reward is zero
//     assert_eq!(
//         suite.query_flexible_stake_reward(ALICE).unwrap(),
//         FlexibleReward {
//             eclip: Uint128::zero(),
//             eclipastro: Uint128::from(0u128),
//             // eclipastro: Uint128::from(237u128),
//         },
//     );

//     assert_eq!(
//         suite.query_flexible_stake_reward(BOB).unwrap(),
//         FlexibleReward {
//             eclip: Uint128::zero(),
//             eclipastro: Uint128::from(0u128),
//             // eclipastro: Uint128::from(553u128),
//         },
//     );

//     // change time and check eclip rewards
//     // time changed but xASTRO rewards are not claimed so there is no eclipASTRO rewards
//     suite.update_time(43200); // 12 hours

//     assert_eq!(
//         suite.query_flexible_stake_reward(ALICE).unwrap(),
//         FlexibleReward {
//             eclip: Uint128::from(150_000u128),
//             eclipastro: Uint128::from(0u128),
//             // eclipastro: Uint128::from(237u128),
//         },
//     );

//     assert_eq!(
//         suite.query_flexible_stake_reward(BOB).unwrap(),
//         FlexibleReward {
//             eclip: Uint128::from(350_000u128),
//             eclipastro: Uint128::from(0u128),
//             // eclipastro: Uint128::from(553u128),
//         },
//     );

//     // check converting
//     suite.convert_astro(ALICE, 4_000).unwrap();
//     assert_eq!(
//         suite.query_astro_staking_total_deposit().unwrap(),
//         1_114_000
//     );
//     assert_eq!(suite.query_astro_staking_total_shares().unwrap(), 1_013_639);
//     assert_eq!(
//         suite
//             .query_xastro_balance(suite.voter_contract().as_str())
//             .unwrap(),
//         13_639
//     );

//     assert_eq!(
//         suite.query_converter_reward().unwrap(),
//         RewardResponse {
//             users_reward: Reward {
//                 token: suite.eclipastro_contract(),
//                 amount: Uint128::from(790u128) // total 899, users_reward 899 * 0.8 = 719
//             },
//             ce_holders_reward: Reward {
//                 token: suite.xastro_contract(),
//                 amount: Uint128::from(36u128)
//             },
//             stability_pool_reward: Reward {
//                 token: suite.xastro_contract(),
//                 amount: Uint128::from(22u128)
//             },
//             treasury_reward: Reward {
//                 token: suite.xastro_contract(),
//                 amount: Uint128::from(122u128)
//             },
//         }
//     );

//     // staking will withdraw current rewards
//     // convert users xastro reward to eclipastro, and sends it to reward_distributor contract
//     // send stability pool reward and ce_holders reward as xastro
//     // total xastro amount will be reduced and pending rewards are zero. treasury rewards are not claimed.
//     suite.flexible_stake(ALICE, 4_000).unwrap();
//     assert_eq!(
//         suite.query_reward_distributor_pending_rewards().unwrap(),
//         vec![(suite.get_time(), Uint128::from(790u128))]
//     );
//     // xASTRO rewards claimed. with time pass, you can see eclipASTRO rewards
//     assert_eq!(
//         suite
//             .query_reward_distributor_total_staking()
//             .unwrap()
//             .reward_weight_eclipastro,
//         Decimal256::zero()
//     );

//     assert_eq!(
//         suite
//             .query_xastro_balance(suite.voter_contract().as_str())
//             .unwrap(),
//         13_581
//     );
//     assert_eq!(
//         suite
//             .query_xastro_balance(suite.eclipse_treasury().as_str())
//             .unwrap(),
//         0
//     );
//     assert_eq!(
//         suite
//             .query_xastro_balance(suite.eclipse_stability_pool().as_str())
//             .unwrap(),
//         22
//     );
//     assert_eq!(
//         suite
//             .query_xastro_balance(suite.eclipse_ce_reward_distributor().as_str())
//             .unwrap(),
//         36
//     );
//     assert_eq!(
//         suite
//             .query_eclipastro_balance(suite.reward_distributor_contract().as_str())
//             .unwrap(),
//         790
//     );

//     assert_eq!(
//         suite.query_flexible_stake_reward(ALICE).unwrap(),
//         FlexibleReward {
//             eclip: Uint128::from(150_000u128),
//             eclipastro: Uint128::from(0u128),
//         },
//     );

//     assert_eq!(
//         suite.query_flexible_stake_reward(BOB).unwrap(),
//         FlexibleReward {
//             eclip: Uint128::from(350_000u128),
//             eclipastro: Uint128::from(0u128),
//         },
//     );

//     // change astro/xastro ratio again and check balances and rewards
//     suite
//         .send_astro(
//             astro_staking_initiator,
//             &suite.astro_staking_contract(),
//             100_000,
//         )
//         .unwrap();

//     assert_eq!(
//         suite.query_astro_staking_total_deposit().unwrap(),
//         1_214_000
//     );
//     assert_eq!(suite.query_astro_staking_total_shares().unwrap(), 1_013_639);
//     assert_eq!(
//         suite
//             .query_xastro_balance(suite.voter_contract().as_str())
//             .unwrap(),
//         13581
//     );

//     assert_eq!(
//         suite.query_converter_reward().unwrap(),
//         RewardResponse {
//             users_reward: Reward {
//                 token: suite.eclipastro_contract(),
//                 amount: Uint128::from(1004u128) // (13639 * 1214000 / 1013639 - 14000) * 1013639 / 1214000 - 899 = 1049 * 0.8
//             },
//             ce_holders_reward: Reward {
//                 token: suite.xastro_contract(),
//                 amount: Uint128::from(42u128)
//             },
//             stability_pool_reward: Reward {
//                 token: suite.xastro_contract(),
//                 amount: Uint128::from(26u128)
//             },
//             treasury_reward: Reward {
//                 token: suite.xastro_contract(),
//                 amount: Uint128::from(264u128)
//             },
//         }
//     );

//     suite.update_time(43200);
//     assert_eq!(
//         suite.query_reward_distributor_pending_rewards().unwrap(),
//         vec![(suite.get_time() - 43200, Uint128::from(790u128))]
//     );

//     assert_eq!(
//         suite.query_flexible_stake_reward(ALICE).unwrap(),
//         FlexibleReward {
//             eclip: Uint128::from(399_999u128),
//             eclipastro: Uint128::from(24u128), // total rewards 790 / 8 / 2 = 49 user rewards = 49 / 2 = 24
//         },
//     );

//     assert_eq!(
//         suite.query_flexible_stake_reward(BOB).unwrap(),
//         FlexibleReward {
//             eclip: Uint128::from(599999u128),
//             eclipastro: Uint128::from(24u128),
//         },
//     );

//     // claim alice rewards and check balances and rewards
//     suite.flexible_claim(ALICE).unwrap();

//     assert_eq!(
//         suite.query_converter_reward().unwrap(),
//         RewardResponse {
//             users_reward: Reward {
//                 token: suite.eclipastro_contract(),
//                 amount: Uint128::from(0u128) // (13639 * 1214000 / 1013639 - 14000) * 1013639 / 1214000 - 899 = 1049 * 0.8
//             },
//             ce_holders_reward: Reward {
//                 token: suite.xastro_contract(),
//                 amount: Uint128::from(0u128)
//             },
//             stability_pool_reward: Reward {
//                 token: suite.xastro_contract(),
//                 amount: Uint128::from(0u128)
//             },
//             treasury_reward: Reward {
//                 token: suite.xastro_contract(),
//                 amount: Uint128::from(264u128)
//             },
//         }
//     );

//     assert_eq!(
//         suite
//             .query_xastro_balance(suite.eclipse_treasury().as_str())
//             .unwrap(),
//         0
//     );
//     assert_eq!(
//         suite
//             .query_xastro_balance(suite.eclipse_stability_pool().as_str())
//             .unwrap(),
//         48
//     );
//     assert_eq!(
//         suite
//             .query_xastro_balance(suite.eclipse_ce_reward_distributor().as_str())
//             .unwrap(),
//         78
//     );
//     assert_eq!(
//         suite
//             .query_eclipastro_balance(suite.reward_distributor_contract().as_str())
//             .unwrap(),
//         1770
//     );

//     assert_eq!(
//         suite.query_flexible_stake_reward(ALICE).unwrap(),
//         FlexibleReward {
//             eclip: Uint128::from(0u128),
//             eclipastro: Uint128::from(0u128),
//         },
//     );

//     assert_eq!(
//         suite.query_flexible_stake_reward(BOB).unwrap(),
//         FlexibleReward {
//             eclip: Uint128::from(599_999u128),
//             eclipastro: Uint128::from(24u128),
//         },
//     );

//     assert_eq!(suite.query_eclipastro_balance(ALICE).unwrap(), 24);
//     assert_eq!(
//         suite
//             .balance_native(ALICE.to_string(), suite.eclip())
//             .unwrap(),
//         399_999u128
//     );

//     assert_eq!(suite.query_eclipastro_balance(BOB).unwrap(), 0);
//     assert_eq!(
//         suite
//             .balance_native(BOB.to_string(), suite.eclip())
//             .unwrap(),
//         0
//     );

//     assert_eq!(
//         suite
//             .query_xastro_balance(suite.voter_contract().as_str())
//             .unwrap(),
//         13513
//     );

//     suite.flexible_unstake(ALICE, 1_000).unwrap();

//     assert_eq!(suite.query_eclipastro_balance(ALICE).unwrap(), 1024);
// }