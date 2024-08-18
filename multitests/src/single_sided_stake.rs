use astroport::asset::AssetInfo;
use cosmwasm_std::{Addr, Uint128};
use cw_controllers::AdminError;
use equinox_msg::{
    single_sided_staking::{
        Config, RewardDetail, RewardDetails, TimeLockConfig, UpdateConfigMsg, UserReward,
        UserRewardByDuration, UserRewardByLockedAt, UserStaking, UserStakingByDuration,
    },
    voter::msg::AstroStakingRewardResponse,
};
use single_sided_staking::error::ContractError;

use crate::suite::{SuiteBuilder, ALICE, ATTACKER, BOB};

const ONE_MONTH: u64 = 86400 * 30;
const THREE_MONTH: u64 = 86400 * 30 * 3;
const SIX_MONTH: u64 = 86400 * 30 * 6;
const NINE_MONTH: u64 = 86400 * 30 * 9;
const ONE_YEAR: u64 = 86400 * 365;

#[test]
fn instantiate() {
    let mut suite = SuiteBuilder::new().build();
    suite.update_config();

    suite
        .mint_native(ALICE.to_string(), suite.astro(), 10_000_000)
        .unwrap();
}

#[test]
fn update_config() {
    let mut suite = SuiteBuilder::new().build();
    suite.update_config();

    suite
        .mint_native(ALICE.to_string(), suite.astro(), 10_000_000)
        .unwrap();

    let test_config = UpdateConfigMsg {
        timelock_config: Some(vec![TimeLockConfig {
            duration: ONE_MONTH,
            early_unlock_penalty_bps: 200,
            reward_multiplier: 1,
        }]),
        voter: Some("wasm1_voter".to_string()),
        treasury: Some("wasm1_treasury".to_string()),
    };

    // attacker
    let err = suite
        .update_single_sided_stake_config(ATTACKER, test_config.clone())
        .unwrap_err();
    assert_eq!(
        ContractError::Admin(AdminError::NotAdmin {}),
        err.downcast().unwrap()
    );

    suite
        .update_single_sided_stake_config(&suite.admin(), test_config.clone())
        .unwrap();
    assert_eq!(
        suite.query_single_sided_stake_config().unwrap(),
        Config {
            token: suite.eclipastro(),
            timelock_config: vec![TimeLockConfig {
                duration: ONE_MONTH,
                early_unlock_penalty_bps: 200,
                reward_multiplier: 1,
            }],
            treasury: Addr::unchecked("wasm1_treasury"),
            voter: Addr::unchecked("wasm1_voter")
        }
    );
}

#[test]
fn stake() {
    let mut suite = SuiteBuilder::new().build();
    suite.update_config();

    // mint beclip
    suite
        .mint_beclip(&suite.single_staking_contract(), 1_000_000_000_000_000)
        .unwrap();
    suite
        .mint_native(
            suite.single_staking_contract(),
            suite.eclip(),
            1_000_000_000_000_000,
        )
        .unwrap();

    suite
        .mint_native(BOB.to_string(), suite.astro(), 10_000_000_000)
        .unwrap();

    // ready astro_staking_pool
    suite.stake_astro(&suite.admin(), 1_000_000).unwrap();

    assert_eq!(suite.query_single_sided_staking(BOB).unwrap(), vec![]);
    assert_eq!(suite.query_single_sided_total_staking().unwrap(), 0);
    // bob converts 1_001 astro and get 1_000 eclipAstro due to rounding
    suite.convert_astro(BOB, 1_000).unwrap();
    let err = suite.single_sided_stake(BOB, 1_000, 10, None).unwrap_err();
    assert_eq!(
        ContractError::NoLockingPeriodFound(10),
        err.downcast().unwrap()
    );
    suite
        .single_sided_stake(BOB, 1_000, ONE_MONTH, None)
        .unwrap();
    assert_eq!(
        suite.query_single_sided_staking(BOB).unwrap(),
        vec![UserStaking {
            duration: ONE_MONTH,
            staking: vec![UserStakingByDuration {
                amount: Uint128::from(1_000u128),
                locked_at: suite.get_time()
            }]
        }]
    );
    assert_eq!(suite.query_single_sided_total_staking().unwrap(), 1_000);

    suite.convert_astro(BOB, 1_000).unwrap();
    suite
        .single_sided_stake(BOB, 1_000, ONE_MONTH, None)
        .unwrap();
    assert_eq!(
        suite.query_single_sided_staking(BOB).unwrap(),
        vec![UserStaking {
            duration: ONE_MONTH,
            staking: vec![UserStakingByDuration {
                amount: Uint128::from(2_000u128),
                locked_at: suite.get_time()
            }]
        }]
    );
    assert_eq!(suite.query_single_sided_total_staking().unwrap(), 2_000);

    suite.convert_astro(BOB, 1_000).unwrap();
    suite
        .single_sided_stake(BOB, 1_000, THREE_MONTH, None)
        .unwrap();
    let mut bob_staking = vec![
        UserStaking {
            duration: ONE_MONTH,
            staking: vec![UserStakingByDuration {
                amount: Uint128::from(2_000u128),
                locked_at: suite.get_time(),
            }],
        },
        UserStaking {
            duration: THREE_MONTH,
            staking: vec![UserStakingByDuration {
                amount: Uint128::from(1_000u128),
                locked_at: suite.get_time(),
            }],
        },
    ];
    assert_eq!(suite.query_single_sided_staking(BOB).unwrap(), bob_staking);
    assert_eq!(suite.query_single_sided_total_staking().unwrap(), 3_000);

    suite.convert_astro(BOB, 1_000).unwrap();
    suite.single_sided_stake(BOB, 1_000, 0, None).unwrap();
    assert_eq!(
        suite.query_single_sided_staking(BOB).unwrap(),
        vec![
            UserStaking {
                duration: 0,
                staking: vec![UserStakingByDuration {
                    amount: Uint128::from(1_000u128),
                    locked_at: 0u64,
                }],
            },
            UserStaking {
                duration: ONE_MONTH,
                staking: vec![UserStakingByDuration {
                    amount: Uint128::from(2_000u128),
                    locked_at: suite.get_time(),
                }],
            },
            UserStaking {
                duration: THREE_MONTH,
                staking: vec![UserStakingByDuration {
                    amount: Uint128::from(1_000u128),
                    locked_at: suite.get_time(),
                }],
            },
        ]
    );
    assert_eq!(suite.query_single_sided_total_staking().unwrap(), 4_000);

    suite.update_time(THREE_MONTH);

    suite.convert_astro(BOB, 1_000).unwrap();
    suite
        .single_sided_stake(BOB, 1_000, THREE_MONTH, None)
        .unwrap();
    bob_staking = vec![
        UserStaking {
            duration: 0,
            staking: vec![UserStakingByDuration {
                amount: Uint128::from(1_000u128),
                locked_at: 0u64,
            }],
        },
        UserStaking {
            duration: ONE_MONTH,
            staking: vec![UserStakingByDuration {
                amount: Uint128::from(2_000u128),
                locked_at: suite.get_time() - THREE_MONTH,
            }],
        },
        UserStaking {
            duration: THREE_MONTH,
            staking: vec![
                UserStakingByDuration {
                    amount: Uint128::from(1_000u128),
                    locked_at: suite.get_time() - THREE_MONTH,
                },
                UserStakingByDuration {
                    amount: Uint128::from(1_000u128),
                    locked_at: suite.get_time(),
                },
            ],
        },
    ];
    assert_eq!(suite.query_single_sided_staking(BOB).unwrap(), bob_staking);
    assert_eq!(suite.query_single_sided_total_staking().unwrap(), 5_000);
    let time = suite.get_time();
    suite
        .single_sided_unstake(BOB, THREE_MONTH, time - THREE_MONTH, None, None)
        .unwrap();
    bob_staking = vec![
        UserStaking {
            duration: 0,
            staking: vec![UserStakingByDuration {
                amount: Uint128::from(1_000u128),
                locked_at: 0u64,
            }],
        },
        UserStaking {
            duration: ONE_MONTH,
            staking: vec![UserStakingByDuration {
                amount: Uint128::from(2_000u128),
                locked_at: suite.get_time() - THREE_MONTH,
            }],
        },
        UserStaking {
            duration: THREE_MONTH,
            staking: vec![UserStakingByDuration {
                amount: Uint128::from(1_000u128),
                locked_at: suite.get_time(),
            }],
        },
    ];
    assert_eq!(suite.query_single_sided_staking(BOB).unwrap(), bob_staking);
    assert_eq!(suite.query_single_sided_total_staking().unwrap(), 4_000);
    assert_eq!(suite.query_eclipastro_balance(BOB).unwrap(), 1_000);
    suite
        .single_sided_restake(BOB, ONE_MONTH, time - THREE_MONTH, SIX_MONTH, None, None)
        .unwrap();
    bob_staking = vec![
        UserStaking {
            duration: 0,
            staking: vec![UserStakingByDuration {
                amount: Uint128::from(1_000u128),
                locked_at: 0u64,
            }],
        },
        UserStaking {
            duration: THREE_MONTH,
            staking: vec![UserStakingByDuration {
                amount: Uint128::from(1_000u128),
                locked_at: suite.get_time(),
            }],
        },
        UserStaking {
            duration: SIX_MONTH,
            staking: vec![UserStakingByDuration {
                amount: Uint128::from(2_000u128),
                locked_at: suite.get_time(),
            }],
        },
    ];
    assert_eq!(suite.query_single_sided_staking(BOB).unwrap(), bob_staking);
}

#[test]
fn claim() {
    let mut suite = SuiteBuilder::new().build();
    suite.update_config();
    suite
        .update_single_sided_stake_reward_config(
            &suite.admin(),
            RewardDetails {
                eclip: RewardDetail {
                    info: AssetInfo::NativeToken {
                        denom: suite.eclip(),
                    },
                    daily_reward: Uint128::from(1_000_000u128),
                },
                beclip: RewardDetail {
                    info: AssetInfo::Token {
                        contract_addr: Addr::unchecked(suite.beclip()),
                    },
                    daily_reward: Uint128::from(2_000_000u128),
                },
            },
            None,
        )
        .unwrap();

    // mint beclip
    suite
        .mint_beclip(&suite.single_staking_contract(), 1_000_000_000_000)
        .unwrap();
    suite
        .mint_native(
            suite.single_staking_contract(),
            suite.eclip(),
            1_000_000_000_000,
        )
        .unwrap();

    suite
        .mint_native(BOB.to_string(), suite.astro(), 10_000_000_000)
        .unwrap();

    suite
        .mint_native(ALICE.to_string(), suite.astro(), 10_000_000_000)
        .unwrap();

    // ready astro_staking_pool
    suite.stake_astro(&suite.admin(), 1_000_000).unwrap();

    // Bob converts 3_000 astro and stake it
    suite.convert_astro(BOB, 3_000).unwrap();
    assert_eq!(suite.query_eclipastro_balance(BOB).unwrap(), 3_000);
    suite
        .single_sided_stake(BOB, 3_000, ONE_MONTH, None)
        .unwrap();

    suite.convert_astro(BOB, 1_000).unwrap();
    suite.single_sided_stake(BOB, 1_000, 0, None).unwrap();

    // check initial reward is zero
    assert_eq!(
        suite.query_single_sided_staking_reward(BOB).unwrap(),
        vec![
            UserRewardByDuration {
                duration: 0,
                rewards: vec![UserRewardByLockedAt {
                    locked_at: 0,
                    rewards: UserReward {
                        beclip: Uint128::zero(),
                        eclipastro: Uint128::zero(),
                        eclip: Uint128::zero()
                    }
                }]
            },
            UserRewardByDuration {
                duration: ONE_MONTH,
                rewards: vec![UserRewardByLockedAt {
                    locked_at: suite.get_time(),
                    rewards: UserReward {
                        beclip: Uint128::zero(),
                        eclipastro: Uint128::zero(),
                        eclip: Uint128::zero()
                    }
                }]
            },
            UserRewardByDuration {
                duration: THREE_MONTH,
                rewards: vec![]
            },
            UserRewardByDuration {
                duration: SIX_MONTH,
                rewards: vec![]
            },
            UserRewardByDuration {
                duration: NINE_MONTH,
                rewards: vec![]
            },
            UserRewardByDuration {
                duration: ONE_YEAR,
                rewards: vec![]
            }
        ]
    );

    // change astro/xastro ratio and check balances and rewards
    suite
        .mint_native(suite.astro_staking_contract(), suite.astro(), 100_000)
        .unwrap();
    let (total_deposit, total_shares) = suite.query_astro_staking_data().unwrap();
    assert_eq!(total_deposit.u128(), 1114001);
    assert_eq!(total_shares.u128(), 1013898);
    assert_eq!(
        suite
            .query_balance_native(suite.voter_contract(), suite.xastro())
            .unwrap(),
        3998
    );

    assert_eq!(
        suite.query_voter_astro_staking_rewards().unwrap(),
        AstroStakingRewardResponse {
            users: Uint128::from(312u128), // (3998 * 1104001 / 1003998 - 4000) * 0.8,
            treasury: Uint128::from(79u128)
        }
    );

    assert_eq!(
        suite.query_single_sided_staking_reward(BOB).unwrap(),
        vec![
            UserRewardByDuration {
                duration: 0,
                rewards: vec![UserRewardByLockedAt {
                    locked_at: 0,
                    rewards: UserReward {
                        beclip: Uint128::zero(),
                        eclipastro: Uint128::zero(),
                        eclip: Uint128::zero()
                    }
                }]
            },
            UserRewardByDuration {
                duration: ONE_MONTH,
                rewards: vec![UserRewardByLockedAt {
                    locked_at: suite.get_time(),
                    rewards: UserReward {
                        beclip: Uint128::zero(),
                        eclipastro: Uint128::zero(),
                        eclip: Uint128::zero()
                    }
                }]
            },
            UserRewardByDuration {
                duration: THREE_MONTH,
                rewards: vec![]
            },
            UserRewardByDuration {
                duration: SIX_MONTH,
                rewards: vec![]
            },
            UserRewardByDuration {
                duration: NINE_MONTH,
                rewards: vec![]
            },
            UserRewardByDuration {
                duration: ONE_YEAR,
                rewards: vec![]
            }
        ]
    );

    // change time and check eclip rewards
    suite.update_time(43200);

    assert_eq!(
        suite.query_single_sided_staking_reward(BOB).unwrap(),
        vec![
            UserRewardByDuration {
                duration: 0,
                rewards: vec![UserRewardByLockedAt {
                    locked_at: 0,
                    rewards: UserReward {
                        beclip: Uint128::from(142857u128),
                        eclipastro: Uint128::zero(),
                        eclip: Uint128::from(71428u128)
                    }
                }]
            },
            UserRewardByDuration {
                duration: ONE_MONTH,
                rewards: vec![UserRewardByLockedAt {
                    locked_at: suite.get_time() - 43200,
                    rewards: UserReward {
                        beclip: Uint128::from(857142u128), // 6 times than duration 0
                        eclipastro: Uint128::zero(),
                        eclip: Uint128::from(428571u128)
                    }
                }]
            },
            UserRewardByDuration {
                duration: THREE_MONTH,
                rewards: vec![]
            },
            UserRewardByDuration {
                duration: SIX_MONTH,
                rewards: vec![]
            },
            UserRewardByDuration {
                duration: NINE_MONTH,
                rewards: vec![]
            },
            UserRewardByDuration {
                duration: ONE_YEAR,
                rewards: vec![]
            }
        ]
    );

    // check converting
    suite.convert_astro(BOB, 4_000).unwrap();

    suite.single_stake_claim(BOB, 0, 0, None).unwrap();

    // staking will withdraw current rewards
    // convert users xastro reward to eclipastro, and sends it to reward_distributor contract
    // send stability pool reward and ce_holders reward as xastro
    // total xastro amount will be reduced and pending rewards are zero. treasury rewards are not claimed.
    suite
        .single_sided_stake(BOB, 4_000, THREE_MONTH, None)
        .unwrap();

    assert_eq!(
        suite
            .query_balance_native(suite.treasury(), suite.xastro())
            .unwrap(),
        0
    );
    assert_eq!(
        suite
            .query_eclipastro_balance(&suite.single_staking_contract())
            .unwrap(),
        8312
    );

    assert_eq!(suite.query_beclip_balance(BOB).unwrap(), 142857u128);

    assert_eq!(
        suite.query_voter_astro_staking_rewards().unwrap(),
        AstroStakingRewardResponse {
            users: Uint128::from(0u128), // (3998 * 1104001 / 1003998 - 4000) * 0.8,
            treasury: Uint128::from(79u128)
        }
    );

    assert_eq!(
        suite.query_single_sided_staking_reward(BOB).unwrap(),
        vec![
            UserRewardByDuration {
                duration: 0,
                rewards: vec![UserRewardByLockedAt {
                    locked_at: 0,
                    rewards: UserReward {
                        beclip: Uint128::zero(),
                        eclipastro: Uint128::zero(),
                        eclip: Uint128::zero()
                    }
                }]
            },
            UserRewardByDuration {
                duration: ONE_MONTH,
                rewards: vec![UserRewardByLockedAt {
                    locked_at: suite.get_time() - 43200,
                    rewards: UserReward {
                        beclip: Uint128::from(857142u128), // 6 times than duration 0
                        eclipastro: Uint128::zero(),
                        eclip: Uint128::from(428571u128)
                    }
                }]
            },
            UserRewardByDuration {
                duration: THREE_MONTH,
                rewards: vec![UserRewardByLockedAt {
                    locked_at: suite.get_time(),
                    rewards: UserReward {
                        beclip: Uint128::zero(), // 6 times than duration 0
                        eclipastro: Uint128::zero(),
                        eclip: Uint128::zero()
                    }
                }]
            },
            UserRewardByDuration {
                duration: SIX_MONTH,
                rewards: vec![]
            },
            UserRewardByDuration {
                duration: NINE_MONTH,
                rewards: vec![]
            },
            UserRewardByDuration {
                duration: ONE_YEAR,
                rewards: vec![]
            }
        ]
    );

    // change astro/xastro ratio again and check balances and rewards
    suite
        .mint_native(suite.astro_staking_contract(), suite.astro(), 100_000)
        .unwrap();

    let time = suite.get_time();

    let penalty = suite
        .calculate_penalty(3_000, ONE_MONTH, time - 43_200)
        .unwrap();
    assert_eq!(penalty, 1_500);

    suite
        .single_sided_unstake(BOB, ONE_MONTH, time - 43200, None, None)
        .unwrap();

    assert_eq!(suite.query_eclipastro_balance(BOB).unwrap(), 1_500);
    suite.update_time(43200);
    assert_eq!(
        suite.query_single_sided_staking_reward(BOB).unwrap(),
        vec![
            UserRewardByDuration {
                duration: 0,
                rewards: vec![UserRewardByLockedAt {
                    locked_at: 0,
                    rewards: UserReward {
                        beclip: Uint128::from(40000u128),
                        eclipastro: Uint128::from(7u128),
                        eclip: Uint128::from(20000u128)
                    }
                }]
            },
            UserRewardByDuration {
                duration: ONE_MONTH,
                rewards: vec![]
            },
            UserRewardByDuration {
                duration: THREE_MONTH,
                rewards: vec![UserRewardByLockedAt {
                    locked_at: suite.get_time() - 43200,
                    rewards: UserReward {
                        beclip: Uint128::from(960000u128),
                        eclipastro: Uint128::from(28u128),
                        eclip: Uint128::from(480000u128)
                    }
                }]
            },
            UserRewardByDuration {
                duration: SIX_MONTH,
                rewards: vec![]
            },
            UserRewardByDuration {
                duration: NINE_MONTH,
                rewards: vec![]
            },
            UserRewardByDuration {
                duration: ONE_YEAR,
                rewards: vec![]
            }
        ]
    );

    suite
        .mint_native(suite.astro_staking_contract(), suite.astro(), 10_000u128)
        .unwrap();
    suite.single_stake_claim_all(BOB, true, None).unwrap();
    assert_eq!(suite.query_eclipastro_balance(BOB).unwrap(), 1_535);
    assert_eq!(
        suite
            .query_eclipastro_balance(&suite.single_staking_contract())
            .unwrap(),
        5_900
    );
    assert_eq!(
        suite
            .query_single_sided_staking_eclipastro_rewards()
            .unwrap(),
        vec![
            (1696853200, Uint128::from(572u128)),
            (1696896400, Uint128::from(51u128))
        ]
    );

    suite.convert_astro(ALICE, 3_000).unwrap();
    suite
        .single_sided_stake(ALICE, 3_000, ONE_MONTH, None)
        .unwrap();

    suite.convert_astro(ALICE, 1_000).unwrap();
    suite.single_sided_stake(ALICE, 1_000, 0, None).unwrap();

    // check initial reward is zero
    assert_eq!(
        suite.query_single_sided_staking_reward(ALICE).unwrap(),
        vec![
            UserRewardByDuration {
                duration: 0,
                rewards: vec![UserRewardByLockedAt {
                    locked_at: 0,
                    rewards: UserReward {
                        beclip: Uint128::zero(),
                        eclipastro: Uint128::zero(),
                        eclip: Uint128::zero()
                    }
                }]
            },
            UserRewardByDuration {
                duration: ONE_MONTH,
                rewards: vec![UserRewardByLockedAt {
                    locked_at: suite.get_time(),
                    rewards: UserReward {
                        beclip: Uint128::zero(),
                        eclipastro: Uint128::zero(),
                        eclip: Uint128::zero()
                    }
                }]
            },
            UserRewardByDuration {
                duration: THREE_MONTH,
                rewards: vec![]
            },
            UserRewardByDuration {
                duration: SIX_MONTH,
                rewards: vec![]
            },
            UserRewardByDuration {
                duration: NINE_MONTH,
                rewards: vec![]
            },
            UserRewardByDuration {
                duration: ONE_YEAR,
                rewards: vec![]
            }
        ]
    );

    suite.update_time(86400 * 7);

    // check claim rewards with selected assets
    let err = suite
        .single_stake_claim(
            ALICE,
            0,
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

    suite
        .single_stake_claim(
            ALICE,
            0,
            0,
            Some(vec![AssetInfo::NativeToken {
                denom: suite.eclip(),
            }]),
        )
        .unwrap();

    let alice_beclip_balance = suite.query_beclip_balance(ALICE).unwrap();
    assert_eq!(alice_beclip_balance, 0);
}
