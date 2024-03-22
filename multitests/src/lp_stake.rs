use astroport::{
    asset::{Asset, AssetInfo, Decimal256Ext},
    vesting::{VestingAccount, VestingSchedule, VestingSchedulePoint},
};
use cosmwasm_std::{Addr, Decimal256, Uint128};
use equinox_msg::lp_staking::UpdateConfigMsg;

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

#[test]
fn update_config() {
    let mut suite = instantiate();
    let config = suite.query_lp_staking_config().unwrap();
    assert_eq!(config.astro.to_string(), suite.astro_contract());
    assert_eq!(
        config.astroport_generator.to_string(),
        suite.astroport_generator_contract()
    );
    assert_eq!(
        config.ce_reward_distributor.unwrap().to_string(),
        suite.eclipse_ce_reward_distributor()
    );
    assert_eq!(config.eclip.clone(), suite.eclip());
    assert_eq!(config.eclip_daily_reward.u128(), 100_000u128);
    assert_eq!(
        config.lp_token.to_string(),
        suite.eclipastro_xastro_lp_token_contract()
    );
    assert_eq!(
        config.stability_pool.to_string(),
        suite.eclipse_stability_pool()
    );
    assert_eq!(config.treasury.to_string(), suite.eclipse_treasury());
    let new_config = UpdateConfigMsg {
        lp_token: Some(Addr::unchecked("new_lp_token".to_string()).to_string()),
        eclip: Some(Addr::unchecked("new_eclip".to_string()).to_string()),
        eclip_daily_reward: Some(Uint128::from(100_000u128)),
        astroport_generator: Some(
            Addr::unchecked("new_astroport_generator".to_string()).to_string(),
        ),
        treasury: Some(Addr::unchecked("new_treasury".to_string()).to_string()),
        stability_pool: Some(Addr::unchecked("new_stability_pool".to_string()).to_string()),
        ce_reward_distributor: Some(
            Addr::unchecked("new_ce_reward_distributor".to_string()).to_string(),
        ),
    };

    suite
        .lp_staking_update_config(&suite.admin(), new_config)
        .unwrap();

    let config = suite.query_lp_staking_config().unwrap();
    assert_eq!(
        config.astroport_generator.to_string(),
        "new_astroport_generator".to_string()
    );
    assert_eq!(
        config.ce_reward_distributor.unwrap().to_string(),
        "new_ce_reward_distributor".to_string()
    );
    assert_eq!(config.eclip.clone(), "new_eclip".to_string());
    assert_eq!(config.eclip_daily_reward.u128(), 100_000u128);
    assert_eq!(config.lp_token.to_string(), "new_lp_token".to_string());
    assert_eq!(
        config.stability_pool.to_string(),
        "new_stability_pool".to_string()
    );
    assert_eq!(config.treasury.to_string(), "new_treasury".to_string());
}

#[test]
fn lp_staking() {
    let mut suite = instantiate();

    // stake lp_token(convert astro to eclipastro, stake astro to get xastro, provide liquidity to get lp token, stake lp token)
    let total_deposit = suite.query_astro_staking_total_deposit().unwrap();
    let total_sharing = suite.query_astro_staking_total_shares().unwrap();
    assert_eq!(total_deposit, 2_210_000u128);
    assert_eq!(total_sharing, 2_009_089u128);

    suite.convert_astro(ALICE, 1_000).unwrap();

    let alice_eclipastro_amount = suite.query_eclipastro_balance(ALICE).unwrap();
    assert_eq!(alice_eclipastro_amount, 1_000);
    let total_deposit = suite.query_astro_staking_total_deposit().unwrap();
    let total_sharing = suite.query_astro_staking_total_shares().unwrap();
    assert_eq!(total_deposit, 2_210_000u128 + 1_000u128);
    assert_eq!(
        total_sharing,
        2_009_089u128 + 1_000u128 * 2_009_089u128 / 2_210_000u128
    );

    suite.stake_astro(ALICE, 1_000u128).unwrap();
    let alice_xastro_amount = suite.query_xastro_balance(ALICE).unwrap();
    assert_eq!(alice_xastro_amount, 1_000 * total_sharing / total_deposit);

    suite
        .provide_liquidity(
            ALICE,
            Addr::unchecked(suite.eclipastro_xastro_lp_contract()),
            vec![
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: Addr::unchecked(suite.eclipastro_contract()),
                    },
                    amount: Uint128::from(alice_eclipastro_amount),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: Addr::unchecked(suite.xastro_contract()),
                    },
                    amount: Uint128::from(alice_xastro_amount),
                },
            ],
            None,
            None,
        )
        .unwrap();

    let alice_eclipastro_amount = suite.query_eclipastro_balance(ALICE).unwrap();
    assert_eq!(alice_eclipastro_amount, 0);
    let alice_xastro_amount = suite.query_xastro_balance(ALICE).unwrap();
    assert_eq!(alice_xastro_amount, 0);
    let alice_lp_token_amount = suite.query_lp_token_balance(ALICE).unwrap();
    assert_eq!(alice_lp_token_amount.u128(), 953u128);
    let alice_lp_token_stake_amount = 500u128;

    suite
        .stake_lp_token(ALICE, alice_lp_token_stake_amount)
        .unwrap();

    let alice_lp_token_amount = suite.query_lp_token_balance(ALICE).unwrap();
    assert_eq!(alice_lp_token_amount.u128(), 453u128);
    let alice_lp_token_staking = suite.query_user_lp_token_staking(ALICE).unwrap();
    assert_eq!(
        alice_lp_token_staking.staked.u128(),
        alice_lp_token_stake_amount
    );
    assert_eq!(alice_lp_token_staking.astroport_rewards, vec![]);
    assert_eq!(alice_lp_token_staking.pending_eclip_rewards.u128(), 0);
    assert_eq!(
        alice_lp_token_staking.eclip_reward_weight,
        Decimal256::zero()
    );
    let pending_incentives = suite
        .query_incentive_pending_rewards(&suite.lp_staking())
        .unwrap();
    assert_eq!(pending_incentives.len(), 1);
    assert_eq!(pending_incentives[0].amount.u128(), 0u128);
    let total_lp_token_staking = suite.query_total_lp_token_staking().unwrap();
    assert_eq!(
        total_lp_token_staking.total_staked.u128(),
        alice_lp_token_stake_amount
    );
    assert_eq!(total_lp_token_staking.astroport_reward_weights.len(), 1);
    assert_eq!(
        total_lp_token_staking.astroport_reward_weights[0].reward_weight,
        Decimal256::zero()
    );
    assert_eq!(
        total_lp_token_staking.eclip_reward_weight,
        Decimal256::zero()
    );
    let user_rewards = suite.query_user_lp_staking_reward(ALICE).unwrap();
    assert_eq!(
        user_rewards[0].asset,
        AssetInfo::Token {
            contract_addr: Addr::unchecked(suite.astro_contract())
        }
    );
    assert_eq!(user_rewards[0].amount.u128(), 0u128);
    assert_eq!(
        user_rewards[1].asset,
        AssetInfo::NativeToken {
            denom: suite.eclip()
        }
    );
    assert_eq!(user_rewards[1].amount.u128(), 0u128);

    // update time
    // let start_time = suite.get_time();
    suite.update_time(86400);

    let incentive_deposit = suite
        .query_incentive_deposit(
            &suite.eclipastro_xastro_lp_token_contract(),
            &suite.lp_staking(),
        )
        .unwrap();
    assert_eq!(incentive_deposit.u128(), alice_lp_token_stake_amount);
    let pending_incentives = suite
        .query_incentive_pending_rewards(&suite.lp_staking())
        .unwrap();
    assert_eq!(pending_incentives.len(), 1);
    assert_eq!(pending_incentives[0].amount.u128(), 864000u128);
    let total_lp_token_staking = suite.query_total_lp_token_staking().unwrap();
    assert_eq!(
        total_lp_token_staking.total_staked.u128(),
        alice_lp_token_stake_amount
    );
    assert_eq!(total_lp_token_staking.astroport_reward_weights.len(), 1);
    assert_eq!(
        total_lp_token_staking.astroport_reward_weights[0].asset,
        AssetInfo::Token {
            contract_addr: Addr::unchecked(suite.astro_contract())
        }
    );
    let astro_reward_weight =
        Decimal256::from_ratio(864000u128 * 8_000 / 10_000, alice_lp_token_stake_amount);
    let eclip_reward_weight = Decimal256::from_ratio(100_000u128, alice_lp_token_stake_amount);
    assert_eq!(
        total_lp_token_staking.astroport_reward_weights[0].reward_weight,
        astro_reward_weight
    ); // 1382.4
    assert_eq!(
        total_lp_token_staking.eclip_reward_weight,
        eclip_reward_weight
    ); // 200
    let user_rewards = suite.query_user_lp_staking_reward(ALICE).unwrap();
    assert_eq!(
        user_rewards[0].asset,
        AssetInfo::Token {
            contract_addr: Addr::unchecked(suite.astro_contract())
        }
    );
    let alice_pending_astro_reward = astro_reward_weight
        .checked_mul(Decimal256::from_ratio(alice_lp_token_stake_amount, 1u128))
        .unwrap()
        .to_uint128_with_precision(0u32)
        .unwrap()
        .u128();
    assert_eq!(user_rewards[0].amount.u128(), alice_pending_astro_reward); // 691200
    assert_eq!(
        user_rewards[1].asset,
        AssetInfo::NativeToken {
            denom: suite.eclip()
        }
    );
    let alice_pending_eclip_reward = eclip_reward_weight
        .checked_mul(Decimal256::from_ratio(alice_lp_token_stake_amount, 1u128))
        .unwrap()
        .to_uint128_with_precision(0u32)
        .unwrap()
        .u128();
    assert_eq!(user_rewards[1].amount.u128(), alice_pending_eclip_reward); // 100_000

    // mint ECLIP to lp staking contract for reward distribution
    suite
        .mint_native(suite.lp_staking(), suite.eclip(), 1_000_000_000)
        .unwrap();

    // claim rewards
    let alice_astro_balance = suite.query_astro_balance(ALICE).unwrap();
    let alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    suite.lp_staking_claim_rewards(ALICE).unwrap();
    let new_alice_astro_balance = suite.query_astro_balance(ALICE).unwrap();
    let new_alice_eclip_balance = suite
        .balance_native(ALICE.to_string(), suite.eclip())
        .unwrap();
    assert_eq!(
        new_alice_astro_balance - alice_astro_balance,
        alice_pending_astro_reward
    );
    assert_eq!(
        new_alice_eclip_balance - alice_eclip_balance,
        alice_pending_eclip_reward
    );
    let user_rewards = suite.query_user_lp_staking_reward(ALICE).unwrap();
    assert_eq!(
        user_rewards[0].asset,
        AssetInfo::Token {
            contract_addr: Addr::unchecked(suite.astro_contract())
        }
    );
    assert_eq!(user_rewards[0].amount.u128(), 0u128);
    assert_eq!(
        user_rewards[1].asset,
        AssetInfo::NativeToken {
            denom: suite.eclip()
        }
    );
    assert_eq!(user_rewards[1].amount.u128(), 0u128);
    let pending_incentives = suite
        .query_incentive_pending_rewards(&suite.lp_staking())
        .unwrap();
    assert_eq!(pending_incentives.len(), 1);
    assert_eq!(pending_incentives[0].amount.u128(), 0u128);

    // update time again
    suite.update_time(86400);

    let pending_incentives = suite
        .query_incentive_pending_rewards(&suite.lp_staking())
        .unwrap();
    assert_eq!(pending_incentives.len(), 1);
    assert_eq!(pending_incentives[0].amount.u128(), 864000u128);
    let total_lp_token_staking = suite.query_total_lp_token_staking().unwrap();
    assert_eq!(
        total_lp_token_staking.total_staked.u128(),
        alice_lp_token_stake_amount
    );
    assert_eq!(total_lp_token_staking.astroport_reward_weights.len(), 1);
    assert_eq!(
        total_lp_token_staking.astroport_reward_weights[0].asset,
        AssetInfo::Token {
            contract_addr: Addr::unchecked(suite.astro_contract())
        }
    );
    let new_astro_reward_weight = astro_reward_weight
        + Decimal256::from_ratio(864000u128 * 8_000 / 10_000, alice_lp_token_stake_amount);
    let new_eclip_reward_weight =
        eclip_reward_weight + Decimal256::from_ratio(100_000u128, alice_lp_token_stake_amount);
    assert_eq!(
        total_lp_token_staking.astroport_reward_weights[0].reward_weight,
        new_astro_reward_weight
    ); // 2764.8
    assert_eq!(
        total_lp_token_staking.eclip_reward_weight,
        new_eclip_reward_weight
    ); // 400
    let user_rewards = suite.query_user_lp_staking_reward(ALICE).unwrap();
    assert_eq!(
        user_rewards[0].asset,
        AssetInfo::Token {
            contract_addr: Addr::unchecked(suite.astro_contract())
        }
    );
    let alice_pending_astro_reward = (new_astro_reward_weight - astro_reward_weight)
        .checked_mul(Decimal256::from_ratio(alice_lp_token_stake_amount, 1u128))
        .unwrap()
        .to_uint128_with_precision(0u32)
        .unwrap()
        .u128();
    assert_eq!(user_rewards[0].amount.u128(), alice_pending_astro_reward); // 691200
    assert_eq!(
        user_rewards[1].asset,
        AssetInfo::NativeToken {
            denom: suite.eclip()
        }
    );
    let alice_pending_eclip_reward = (new_eclip_reward_weight - eclip_reward_weight)
        .checked_mul(Decimal256::from_ratio(alice_lp_token_stake_amount, 1u128))
        .unwrap()
        .to_uint128_with_precision(0u32)
        .unwrap()
        .u128();
    assert_eq!(user_rewards[1].amount.u128(), alice_pending_eclip_reward); // 100_000

    // stake more
    suite
        .stake_lp_token(ALICE, alice_lp_token_amount.u128())
        .unwrap();
    let alice_lp_token_staking = suite.query_user_lp_token_staking(ALICE).unwrap();
    assert_eq!(alice_lp_token_staking.staked.u128(), 953u128);
    let pending_incentives = suite
        .query_incentive_pending_rewards(&suite.lp_staking())
        .unwrap();
    assert_eq!(pending_incentives.len(), 1);
    assert_eq!(pending_incentives[0].amount.u128(), 0u128);
    let total_lp_token_staking = suite.query_total_lp_token_staking().unwrap();
    assert_eq!(total_lp_token_staking.total_staked.u128(), 953u128);
    assert_eq!(total_lp_token_staking.astroport_reward_weights.len(), 1);
    assert_eq!(
        total_lp_token_staking.astroport_reward_weights[0].reward_weight,
        new_astro_reward_weight
    );
    assert_eq!(
        total_lp_token_staking.eclip_reward_weight,
        new_eclip_reward_weight
    );
    let user_rewards = suite.query_user_lp_staking_reward(ALICE).unwrap();
    assert_eq!(
        user_rewards[0].asset,
        AssetInfo::Token {
            contract_addr: Addr::unchecked(suite.astro_contract())
        }
    );
    assert_eq!(user_rewards[0].amount.u128(), alice_pending_astro_reward);
    assert_eq!(user_rewards[1].amount.u128(), alice_pending_eclip_reward);

    // unstake
    suite.unstake_lp_token(ALICE, 500u128).unwrap();
    let alice_lp_token_staking = suite.query_user_lp_token_staking(ALICE).unwrap();
    assert_eq!(alice_lp_token_staking.staked.u128(), 453u128);
    let total_lp_token_staking = suite.query_total_lp_token_staking().unwrap();
    assert_eq!(total_lp_token_staking.total_staked.u128(), 453u128);
    assert_eq!(
        total_lp_token_staking.astroport_reward_weights[0].reward_weight,
        new_astro_reward_weight
    );
    assert_eq!(
        total_lp_token_staking.eclip_reward_weight,
        new_eclip_reward_weight
    );
    let user_rewards = suite.query_user_lp_staking_reward(ALICE).unwrap();
    assert_eq!(user_rewards[0].amount.u128(), 0u128);
    assert_eq!(user_rewards[1].amount.u128(), 0u128);
    assert_eq!(
        suite.query_astro_balance(ALICE).unwrap() - new_alice_astro_balance,
        alice_pending_astro_reward
    );
    assert_eq!(
        suite
            .balance_native(ALICE.to_string(), suite.eclip())
            .unwrap()
            - new_alice_eclip_balance,
        alice_pending_eclip_reward
    );
}
