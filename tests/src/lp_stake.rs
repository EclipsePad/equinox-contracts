use std::str::FromStr;

use astroport::{
    asset::{Asset, AssetInfo, Decimal256Ext},
    vesting::{VestingAccount, VestingSchedule, VestingSchedulePoint},
};
use cosmwasm_std::{Addr, Decimal256, Uint128};
use equinox_msg::{
    lp_staking::{RewardAmount, RewardWeight},
    utils::UNBONDING_PERIOD_0,
};
use lp_staking::error::ContractError;

use crate::suite::{Suite, SuiteBuilder, ALICE, BOB, CAROL, TREASURY};

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
    // add funds to vault
    suite
        .add_lp_vault_reward(
            &suite.admin(),
            None,
            None,
            12_800_000_000u128,
            8_600_000_000u128,
        )
        .unwrap();
    suite
        .mint_native(BOB.to_string(), suite.astro(), 1_000_000_000)
        .unwrap();
    suite.convert_astro(BOB, 1_000).unwrap();

    let bob_eclipastro_amount = suite.query_eclipastro_balance(BOB).unwrap();

    suite.stake_astro(BOB, 1_000u128).unwrap();
    let bob_xastro_amount = suite
        .query_balance_native(BOB.to_string(), suite.xastro())
        .unwrap();

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

    let bob_lp_token_amount = suite.query_lp_token_balance(BOB).unwrap();
    assert_eq!(bob_lp_token_amount.u128(), 953u128);

    let total_lp_token_staking = suite.query_total_lp_token_staking().unwrap();
    assert_eq!(total_lp_token_staking.u128(), 0);

    suite.stake_lp_token(BOB, 100).unwrap();
    let bob_lp_token_staking = suite.query_user_lp_token_staking(BOB).unwrap();
    assert_eq!(bob_lp_token_staking.staked.u128(), 100);
    let total_lp_token_staking = suite.query_total_lp_token_staking().unwrap();
    assert_eq!(total_lp_token_staking.u128(), 100);
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

    // update time
    suite.update_time(86400);

    let incentive_deposit = suite
        .query_incentive_deposit(
            &suite.eclipastro_xastro_lp_token(),
            &suite.lp_staking_contract(),
        )
        .unwrap();
    assert_eq!(incentive_deposit.u128(), 100);
    let pending_incentives = suite
        .query_incentive_pending_rewards(&suite.lp_staking_contract())
        .unwrap();
    assert_eq!(pending_incentives.len(), 1);
    assert_eq!(pending_incentives[0].amount.u128(), 864000u128);
    let total_lp_token_staking = suite.query_total_lp_token_staking().unwrap();
    assert_eq!(total_lp_token_staking.u128(), 100);
    let reward_weights = suite.query_reward_weights().unwrap();
    let astro_reward_weight = Decimal256::from_ratio(864000u128 * 8_000 / 10_000, 100u128);
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
                reward_weight: Decimal256::from_str("2866666.66").unwrap(),
            },
            RewardWeight {
                info: AssetInfo::NativeToken {
                    denom: suite.eclip()
                },
                reward_weight: Decimal256::from_str("4266666.66").unwrap()
            }
        ]
    );
    let user_rewards = suite.query_user_lp_staking_reward(BOB).unwrap();
    let bob_pending_astro_reward = astro_reward_weight
        .checked_mul(Decimal256::from_ratio(100u128, 1u128))
        .unwrap()
        .to_uint128_with_precision(0u32)
        .unwrap();
    let bob_pending_beclip_reward = Uint128::from(286666666u128);
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
                amount: Uint128::from(426666666u128)
            },
        ]
    ); // 100_000

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

    let reward_weights = suite.query_reward_weights().unwrap();

    suite.stake_lp_token(ALICE, 100u128).unwrap();
    assert_eq!(
        suite.query_user_reward_weights(ALICE.to_string()).unwrap(),
        reward_weights
    );

    // update time againd
    suite.update_time(86400);

    let pending_incentives = suite
        .query_incentive_pending_rewards(&suite.lp_staking_contract())
        .unwrap();
    assert_eq!(pending_incentives.len(), 1);
    assert_eq!(pending_incentives[0].amount.u128(), 864000u128);
    let total_lp_token_staking = suite.query_total_lp_token_staking().unwrap();
    assert_eq!(total_lp_token_staking.u128(), 100 + 100);
    let user_rewards = suite.query_user_lp_staking_reward(ALICE).unwrap();
    assert_eq!(user_rewards[0].amount.u128(), 345600); // 691200

    assert_eq!(user_rewards[2].amount.u128(), 0); // 100_000

    // claim rewards
    let bob_astro_balance = suite
        .query_balance_native(BOB.to_string(), suite.astro())
        .unwrap();
    let bob_beclip_balance = suite.query_beclip_balance(BOB).unwrap();
    let bob_pending_rewards = suite.query_user_lp_staking_reward(BOB).unwrap();
    suite.lp_staking_claim_rewards(BOB).unwrap();
    let new_bob_astro_balance = suite
        .query_balance_native(BOB.to_string(), suite.astro())
        .unwrap();
    let new_bob_beclip_balance = suite.query_beclip_balance(BOB).unwrap();
    assert_eq!(
        new_bob_astro_balance - bob_astro_balance,
        bob_pending_rewards[0].amount.u128()
    );
    assert_eq!(
        new_bob_beclip_balance - bob_beclip_balance,
        bob_pending_rewards[1].amount.u128()
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

    // update time againd
    suite.update_time(86400);

    // // unstake
    // suite.lp_unstake(BOB, 100u128, None).unwrap();
    // let bob_lp_token_staking = suite.query_user_lp_token_staking(BOB).unwrap();
    // assert_eq!(bob_lp_token_staking.staked.u128(), 0u128);
    // let total_lp_token_staking = suite.query_total_lp_token_staking().unwrap();
    // assert_eq!(total_lp_token_staking.u128(), 100u128);
}

#[test]
fn blacklist() {
    let mut suite = instantiate();
    // add funds to vault
    suite
        .add_lp_vault_reward(
            &suite.admin(),
            None,
            None,
            12_800_000_000u128,
            8_600_000_000u128,
        )
        .unwrap();
    suite
        .mint_native(CAROL.to_string(), suite.astro(), 1_000_000_000)
        .unwrap();
    suite.convert_astro(CAROL, 1_000).unwrap();

    let carol_eclipastro_amount = suite.query_eclipastro_balance(CAROL).unwrap();

    suite.stake_astro(CAROL, 1_000u128).unwrap();
    let carol_xastro_amount = suite
        .query_balance_native(CAROL.to_string(), suite.xastro())
        .unwrap();

    suite
        .provide_liquidity(
            CAROL,
            Addr::unchecked(suite.eclipastro_xastro_lp_contract()),
            vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: suite.eclipastro(),
                    },
                    amount: Uint128::from(carol_eclipastro_amount),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: suite.xastro(),
                    },
                    amount: Uint128::from(carol_xastro_amount),
                },
            ],
            None,
            None,
        )
        .unwrap();

    let carol_lp_token_amount = suite.query_lp_token_balance(CAROL).unwrap();
    assert_eq!(carol_lp_token_amount.u128(), 953u128);

    let total_lp_token_staking = suite.query_total_lp_token_staking().unwrap();
    assert_eq!(total_lp_token_staking.u128(), 0);

    suite.stake_lp_token(CAROL, 100).unwrap();
    let carol_lp_token_rewards = suite.query_user_lp_token_rewards(CAROL).unwrap();
    assert_eq!(carol_lp_token_rewards, []);
    let pending_incentives = suite
        .query_incentive_pending_rewards(&suite.lp_staking_contract())
        .unwrap();
    assert_eq!(pending_incentives.len(), 1);
    assert_eq!(pending_incentives[0].amount.u128(), 0u128);

    // update time
    suite.update_time(86400);

    let incentive_deposit = suite
        .query_incentive_deposit(
            &suite.eclipastro_xastro_lp_token(),
            &suite.lp_staking_contract(),
        )
        .unwrap();
    assert_eq!(incentive_deposit.u128(), 100);
    let pending_incentives = suite
        .query_incentive_pending_rewards(&suite.lp_staking_contract())
        .unwrap();
    assert_eq!(pending_incentives.len(), 1);
    assert_eq!(pending_incentives[0].amount.u128(), 864000u128);
    let total_lp_token_staking = suite.query_total_lp_token_staking().unwrap();
    assert_eq!(total_lp_token_staking.u128(), 100);
    let reward_weights = suite.query_reward_weights().unwrap();
    let astro_reward_weight = Decimal256::from_ratio(864000u128 * 8_000 / 10_000, 100u128);
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
                reward_weight: Decimal256::from_str("2866666.66").unwrap(),
            },
            RewardWeight {
                info: AssetInfo::NativeToken {
                    denom: suite.eclip()
                },
                reward_weight: Decimal256::from_str("4266666.66").unwrap()
            }
        ]
    );
    let user_rewards = suite.query_user_lp_staking_reward(CAROL).unwrap();
    assert_eq!(user_rewards, []); // 100_000
    let err = suite.lp_staking_claim_rewards(CAROL).unwrap_err();
    assert_eq!(ContractError::Blacklisted {}, err.downcast().unwrap());
    assert_eq!(
        suite.query_lp_blacklisted_reward().unwrap(),
        [
            RewardAmount {
                info: AssetInfo::NativeToken {
                    denom: suite.astro()
                },
                amount: Uint128::from(691200u128)
            },
            RewardAmount {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked(suite.beclip())
                },
                amount: Uint128::from(286666666u128)
            },
            RewardAmount {
                info: AssetInfo::NativeToken {
                    denom: suite.eclip()
                },
                amount: Uint128::from(426666666u128)
            },
        ]
    );

    suite.lp_blacklist_claim().unwrap();
    assert_eq!(
        suite
            .query_balance_native(TREASURY.to_string(), suite.eclip())
            .unwrap(),
        713333332u128
    );
}

#[test]
fn unbond_half_period() {
    let mut suite = instantiate();
    // add funds to vault
    suite
        .add_lp_vault_reward(
            &suite.admin(),
            None,
            None,
            12_800_000_000u128,
            8_600_000_000u128,
        )
        .unwrap();
    suite
        .mint_native(BOB.to_string(), suite.astro(), 1_000_000_000)
        .unwrap();
    suite.convert_astro(BOB, 1_000).unwrap();

    let bob_eclipastro_amount = suite.query_eclipastro_balance(BOB).unwrap();

    suite.stake_astro(BOB, 1_000).unwrap();
    let bob_xastro_amount = suite
        .query_balance_native(BOB.to_string(), suite.xastro())
        .unwrap();

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

    suite.stake_lp_token(BOB, 100).unwrap();
    // let bob_lp_token_rewards = suite.query_user_lp_token_rewards(BOB).unwrap();
    // assert_eq!(
    //     bob_lp_token_rewards,
    //     [
    //         RewardAmount {
    //             info: AssetInfo::NativeToken {
    //                 denom: suite.astro()
    //             },
    //             amount: Uint128::zero()
    //         },
    //         RewardAmount {
    //             info: AssetInfo::Token {
    //                 contract_addr: Addr::unchecked(suite.beclip())
    //             },
    //             amount: Uint128::zero()
    //         },
    //         RewardAmount {
    //             info: AssetInfo::NativeToken {
    //                 denom: suite.eclip()
    //             },
    //             amount: Uint128::zero()
    //         },
    //     ]
    // );
    assert_eq!(
        suite.query_balance_native(&BOB, suite.astro()).unwrap(),
        999_998_000
    );

    suite.unbond_lp_token(BOB, 100, UNBONDING_PERIOD_0).unwrap();
    assert_eq!(
        suite.query_balance_native(&BOB, suite.astro()).unwrap(),
        999_998_000
    );

    suite.update_time(UNBONDING_PERIOD_0);
    suite.withdraw_lp_token(BOB, None).unwrap();
    assert_eq!(
        suite.query_balance_native(&BOB, suite.astro()).unwrap(),
        999_998_197
    );
}
