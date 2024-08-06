use astroport::{
    asset::{Asset, AssetInfo, Decimal256Ext},
    vesting::{VestingAccount, VestingSchedule, VestingSchedulePoint},
};
use cosmwasm_std::{Addr, Decimal256, Uint128};
use equinox_msg::lp_staking::{RewardAmount, RewardWeight};

use crate::suite::{Suite, SuiteBuilder, ALICE, BOB};

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
                    info: AssetInfo::NativeToken {
                        denom: suite.eclipastro(),
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
fn lp_staking() {
    let mut suite = instantiate();
    suite
        .mint_native(BOB.to_string(), suite.astro(), 1_000_000_000)
        .unwrap();
    suite.convert_astro(BOB, 1_000).unwrap();

    let bob_eclipastro_amount = suite.query_eclipastro_balance(BOB).unwrap();
    assert_eq!(bob_eclipastro_amount, 1_000);
    let (total_deposit, total_sharing) = suite.query_astro_staking_data().unwrap();

    suite.stake_astro(BOB, 1_000u128).unwrap();
    let bob_xastro_amount = suite
        .query_balance_native(BOB.to_string(), suite.xastro())
        .unwrap();
    assert_eq!(
        bob_xastro_amount,
        1_000 * total_sharing.u128() / total_deposit.u128()
    );

    suite
        .provide_liquidity(
            BOB,
            Addr::unchecked(suite.eclipastro_xastro_lp_contract()),
            vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: suite.eclipastro(),
                    },
                    amount: Uint128::from(bob_eclipastro_amount),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: suite.xastro(),
                    },
                    amount: Uint128::from(bob_xastro_amount),
                },
            ],
            None,
            None,
        )
        .unwrap();

    let bob_eclipastro_amount = suite.query_eclipastro_balance(BOB).unwrap();
    assert_eq!(bob_eclipastro_amount, 0);
    let bob_xastro_amount = suite
        .query_balance_native(BOB.to_string(), suite.xastro())
        .unwrap();
    assert_eq!(bob_xastro_amount, 0);
    let bob_lp_token_amount = suite.query_lp_token_balance(BOB).unwrap();
    assert_eq!(bob_lp_token_amount.u128(), 953u128);
    let bob_lp_token_stake_amount = 500u128;

    suite
        .stake_lp_token(BOB, bob_lp_token_stake_amount)
        .unwrap();

    let bob_lp_token_amount = suite.query_lp_token_balance(BOB).unwrap();
    assert_eq!(bob_lp_token_amount.u128(), 453u128);
    let bob_lp_token_staking = suite.query_user_lp_token_staking(BOB).unwrap();
    assert_eq!(
        bob_lp_token_staking.staked.u128(),
        bob_lp_token_stake_amount
    );
    let bob_lp_token_rewards = suite.query_user_lp_token_rewards(BOB).unwrap();
    assert_eq!(
        bob_lp_token_rewards,
        [
            RewardAmount {
                info: AssetInfo::NativeToken {
                    denom: suite.astro()
                },
                amount: Uint128::zero()
            },
            RewardAmount {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked(suite.beclip())
                },
                amount: Uint128::zero()
            },
            RewardAmount {
                info: AssetInfo::NativeToken {
                    denom: suite.eclip()
                },
                amount: Uint128::zero()
            },
        ]
    );
    let pending_incentives = suite
        .query_incentive_pending_rewards(&suite.lp_staking_contract())
        .unwrap();
    assert_eq!(pending_incentives.len(), 1);
    assert_eq!(pending_incentives[0].amount.u128(), 0u128);
    let total_lp_token_staking = suite.query_total_lp_token_staking().unwrap();
    assert_eq!(total_lp_token_staking.u128(), bob_lp_token_stake_amount);

    // update time
    // let start_time = suite.get_time();
    suite.update_time(86400);

    let incentive_deposit = suite
        .query_incentive_deposit(
            &suite.eclipastro_xastro_lp_token(),
            &suite.lp_staking_contract(),
        )
        .unwrap();
    assert_eq!(incentive_deposit.u128(), bob_lp_token_stake_amount);
    let pending_incentives = suite
        .query_incentive_pending_rewards(&suite.lp_staking_contract())
        .unwrap();
    assert_eq!(pending_incentives.len(), 1);
    assert_eq!(pending_incentives[0].amount.u128(), 864000u128);
    let total_lp_token_staking = suite.query_total_lp_token_staking().unwrap();
    assert_eq!(total_lp_token_staking.u128(), bob_lp_token_stake_amount);
    let reward_weights = suite.query_reward_weights().unwrap();
    let astro_reward_weight =
        Decimal256::from_ratio(864000u128 * 8_000 / 10_000, bob_lp_token_stake_amount);
    let reward_config = suite.query_lp_staking_reward_config().unwrap();
    let beclip_reward_weight = Decimal256::from_ratio(
        reward_config.details.beclip.daily_reward,
        bob_lp_token_stake_amount,
    );
    let eclip_reward_weight = Decimal256::from_ratio(
        reward_config.details.eclip.daily_reward,
        bob_lp_token_stake_amount,
    );
    assert_eq!(
        reward_weights,
        [
            RewardWeight {
                info: AssetInfo::NativeToken {
                    denom: suite.astro()
                },
                reward_weight: astro_reward_weight
            },
            RewardWeight {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked(suite.beclip())
                },
                reward_weight: beclip_reward_weight
            },
            RewardWeight {
                info: AssetInfo::NativeToken {
                    denom: suite.eclip()
                },
                reward_weight: eclip_reward_weight
            }
        ]
    );
    let user_rewards = suite.query_user_lp_staking_reward(BOB).unwrap();
    let bob_pending_astro_reward = astro_reward_weight
        .checked_mul(Decimal256::from_ratio(bob_lp_token_stake_amount, 1u128))
        .unwrap()
        .to_uint128_with_precision(0u32)
        .unwrap();
    let bob_pending_beclip_reward = beclip_reward_weight
        .checked_mul(Decimal256::from_ratio(bob_lp_token_stake_amount, 1u128))
        .unwrap()
        .to_uint128_with_precision(0u32)
        .unwrap();
    let bob_pending_eclip_reward = eclip_reward_weight
        .checked_mul(Decimal256::from_ratio(bob_lp_token_stake_amount, 1u128))
        .unwrap()
        .to_uint128_with_precision(0u32)
        .unwrap();
    assert_eq!(
        user_rewards,
        [
            RewardAmount {
                info: AssetInfo::NativeToken {
                    denom: suite.astro()
                },
                amount: bob_pending_astro_reward
            },
            RewardAmount {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked(suite.beclip())
                },
                amount: bob_pending_beclip_reward
            },
            RewardAmount {
                info: AssetInfo::NativeToken {
                    denom: suite.eclip()
                },
                amount: bob_pending_eclip_reward
            },
        ]
    ); // 100_000

    // mint bECLIP to lp staking contract for reward distribution
    suite
        .mint_beclip(&suite.lp_staking_contract(), 5_000_000_000)
        .unwrap();
    suite
        .mint_native(suite.lp_staking_contract(), suite.eclip(), 5_000_000_000)
        .unwrap();

    // claim rewards
    let bob_astro_balance = suite
        .query_balance_native(BOB.to_string(), suite.astro())
        .unwrap();
    let bob_beclip_balance = suite.query_beclip_balance(BOB).unwrap();
    suite.lp_staking_claim_rewards(BOB).unwrap();
    let new_bob_astro_balance = suite
        .query_balance_native(BOB.to_string(), suite.astro())
        .unwrap();
    let new_bob_beclip_balance = suite.query_beclip_balance(BOB).unwrap();
    assert_eq!(
        new_bob_astro_balance - bob_astro_balance,
        bob_pending_astro_reward.u128()
    );
    assert_eq!(
        new_bob_beclip_balance - bob_beclip_balance,
        bob_pending_beclip_reward.u128()
    );
    let user_rewards = suite.query_user_lp_staking_reward(BOB).unwrap();
    assert_eq!(
        user_rewards,
        [
            RewardAmount {
                info: AssetInfo::NativeToken {
                    denom: suite.astro()
                },
                amount: Uint128::zero()
            },
            RewardAmount {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked(suite.beclip())
                },
                amount: Uint128::zero()
            },
            RewardAmount {
                info: AssetInfo::NativeToken {
                    denom: suite.eclip()
                },
                amount: Uint128::zero()
            },
        ]
    );
    suite.update_time(86400 * 30);
    assert_eq!(
        suite.query_user_reward_weights(ALICE.to_string()).unwrap(),
        vec![]
    );
    // assert_eq!(suite.query_reward_weights().unwrap(), vec![]);
    suite
        .send_denom(suite.eclipastro_xastro_lp_token(), BOB, 400u128, ALICE)
        .unwrap();

    suite.stake_lp_token(ALICE, 100u128).unwrap();
    // assert_eq!(
    //     suite.query_user_reward_weights(ALICE.to_string()).unwrap(),
    //     vec![]
    // );

    // // update time againd
    // suite.update_time(86400);

    //     let pending_incentives = suite
    //         .query_incentive_pending_rewards(&suite.lp_staking())
    //         .unwrap();
    //     assert_eq!(pending_incentives.len(), 1);
    //     assert_eq!(pending_incentives[0].amount.u128(), 864000u128);
    //     let total_lp_token_staking = suite.query_total_lp_token_staking().unwrap();
    //     assert_eq!(
    //         total_lp_token_staking.total_staked.u128(),
    //         alice_lp_token_stake_amount
    //     );
    //     assert_eq!(total_lp_token_staking.astroport_reward_weights.len(), 1);
    //     assert_eq!(
    //         total_lp_token_staking.astroport_reward_weights[0].asset,
    //         AssetInfo::Token {
    //             contract_addr: Addr::unchecked(suite.astro_contract())
    //         }
    //     );
    //     let new_astro_reward_weight = astro_reward_weight
    //         + Decimal256::from_ratio(864000u128 * 8_000 / 10_000, alice_lp_token_stake_amount);
    //     let new_eclip_reward_weight = eclip_reward_weight
    //         + Decimal256::from_ratio(1_000_000_000u128, alice_lp_token_stake_amount);
    //     assert_eq!(
    //         total_lp_token_staking.astroport_reward_weights[0].reward_weight,
    //         new_astro_reward_weight
    //     ); // 2764.8
    //     assert_eq!(
    //         total_lp_token_staking.eclip_reward_weight,
    //         new_eclip_reward_weight
    //     ); // 400
    //     let user_rewards = suite.query_user_lp_staking_reward(ALICE).unwrap();
    //     assert_eq!(
    //         user_rewards[0].asset,
    //         AssetInfo::Token {
    //             contract_addr: Addr::unchecked(suite.astro_contract())
    //         }
    //     );
    //     let alice_pending_astro_reward = (new_astro_reward_weight - astro_reward_weight)
    //         .checked_mul(Decimal256::from_ratio(alice_lp_token_stake_amount, 1u128))
    //         .unwrap()
    //         .to_uint128_with_precision(0u32)
    //         .unwrap()
    //         .u128();
    //     assert_eq!(user_rewards[0].amount.u128(), alice_pending_astro_reward); // 691200
    //     assert_eq!(
    //         user_rewards[1].asset,
    //         AssetInfo::NativeToken {
    //             denom: suite.eclip()
    //         }
    //     );
    //     let alice_pending_eclip_reward = (new_eclip_reward_weight - eclip_reward_weight)
    //         .checked_mul(Decimal256::from_ratio(alice_lp_token_stake_amount, 1u128))
    //         .unwrap()
    //         .to_uint128_with_precision(0u32)
    //         .unwrap()
    //         .u128();
    //     assert_eq!(user_rewards[1].amount.u128(), alice_pending_eclip_reward); // 100_000

    //     // stake more
    //     suite
    //         .stake_lp_token(ALICE, alice_lp_token_amount.u128())
    //         .unwrap();
    //     let alice_lp_token_staking = suite.query_user_lp_token_staking(ALICE).unwrap();
    //     assert_eq!(alice_lp_token_staking.staked.u128(), 953u128);
    //     let pending_incentives = suite
    //         .query_incentive_pending_rewards(&suite.lp_staking())
    //         .unwrap();
    //     assert_eq!(pending_incentives.len(), 1);
    //     assert_eq!(pending_incentives[0].amount.u128(), 0u128);
    //     let total_lp_token_staking = suite.query_total_lp_token_staking().unwrap();
    //     assert_eq!(total_lp_token_staking.total_staked.u128(), 953u128);
    //     assert_eq!(total_lp_token_staking.astroport_reward_weights.len(), 1);
    //     assert_eq!(
    //         total_lp_token_staking.astroport_reward_weights[0].reward_weight,
    //         new_astro_reward_weight
    //     );
    //     assert_eq!(
    //         total_lp_token_staking.eclip_reward_weight,
    //         new_eclip_reward_weight
    //     );
    //     let user_rewards = suite.query_user_lp_staking_reward(ALICE).unwrap();
    //     assert_eq!(
    //         user_rewards[0].asset,
    //         AssetInfo::Token {
    //             contract_addr: Addr::unchecked(suite.astro_contract())
    //         }
    //     );
    //     assert_eq!(user_rewards[0].amount.u128(), alice_pending_astro_reward);
    //     assert_eq!(user_rewards[1].amount.u128(), alice_pending_eclip_reward);

    //     // unstake
    //     suite.unstake_lp_token(ALICE, 500u128, None).unwrap();
    //     let alice_lp_token_staking = suite.query_user_lp_token_staking(ALICE).unwrap();
    //     assert_eq!(alice_lp_token_staking.staked.u128(), 453u128);
    //     let total_lp_token_staking = suite.query_total_lp_token_staking().unwrap();
    //     assert_eq!(total_lp_token_staking.total_staked.u128(), 453u128);
    //     assert_eq!(
    //         total_lp_token_staking.astroport_reward_weights[0].reward_weight,
    //         new_astro_reward_weight
    //     );
    //     assert_eq!(
    //         total_lp_token_staking.eclip_reward_weight,
    //         new_eclip_reward_weight
    //     );
    //     let user_rewards = suite.query_user_lp_staking_reward(ALICE).unwrap();
    //     assert_eq!(user_rewards[0].amount.u128(), 0u128);
    //     assert_eq!(user_rewards[1].amount.u128(), 0u128);
    //     assert_eq!(
    //         suite.query_astro_balance(ALICE).unwrap() - new_alice_astro_balance,
    //         alice_pending_astro_reward
    //     );
    //     assert_eq!(
    //         suite
    //             .balance_native(ALICE.to_string(), suite.eclip())
    //             .unwrap()
    //             - new_alice_eclip_balance,
    //         alice_pending_eclip_reward
    //     );
    //     assert_eq!(
    //         suite
    //             .query_astro_balance(&suite.eclipse_treasury())
    //             .unwrap(),
    //         233280u128
    //     );
    //     assert_eq!(
    //         suite
    //             .query_astro_balance(&suite.eclipse_ce_reward_distributor())
    //             .unwrap(),
    //         69120u128
    //     );
    //     assert_eq!(
    //         suite
    //             .query_astro_balance(&suite.eclipse_stability_pool())
    //             .unwrap(),
    //         43200u128
    //     );
}
