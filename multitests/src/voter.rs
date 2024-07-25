use astroport::{asset::AssetInfo, router::SwapOperation};
use astroport_governance::emissions_controller::hub::{
    AstroPoolConfig, OutpostInfo, OutpostParams, VotedPoolInfo,
};

use cosmwasm_std::{coins, Addr, Decimal, StdResult, Uint128};
use cw_multi_test::Executor;

use speculoos::assert_that;
use strum::IntoEnumIterator;

use eclipse_base::{
    assets::{Currency, Token},
    converters::str_to_dec,
    error::parse_err,
};

use equinox_msg::voter::{
    msg::{DaoResponse, UserResponse, VoterInfoResponse},
    state::{EPOCH_LENGTH, GENESIS_EPOCH_START_DATE, VOTE_DELAY},
    types::{
        BribesAllocationItem, EssenceAllocationItem, EssenceInfo, PoolInfoItem, RewardsInfo,
        RouteItem, RouteListItem, UserType, VoteResults, WeightAllocationItem,
    },
};

use voter::{
    error::ContractError,
    math::{
        calc_essence_allocation, calc_pool_info_list_with_rewards, calc_scaled_essence_allocation,
        calc_updated_essence_allocation, calc_weights_from_essence_allocation,
    },
};

use crate::suite_astro::{
    extensions::{
        astroport_router::AstroportRouterExtension, eclipsepad_staking::EclipsepadStakingExtension,
        minter::MinterExtension, tribute_market_mocks::TributeMarketExtension,
        voter::VoterExtension,
    },
    helper::{assert_error, Acc, ControllerHelper, Denom, Pool},
};

const INITIAL_LIQUIDITY: u128 = 1_000_000;

fn prepare_helper() -> ControllerHelper {
    let mut h = ControllerHelper::new();
    let astro = &h.astro.clone();
    let owner = &h.acc(Acc::Owner);

    h.astroport_router_prepare_contract();
    h.tribute_market_prepare_contract(&h.vxastro.clone(), &h.emission_controller.clone());
    h.minter_prepare_contract();
    h.eclipsepad_staking_prepare_contract(
        None,
        None,
        Some(&Denom::Eclip.to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    h.voter_prepare_contract(
        Some(vec![&owner.to_string()]),
        &h.acc(Acc::Dao),
        None,
        &h.minter_contract_address(),
        &h.eclipsepad_staking_contract_address(),
        None,
        &h.staking.clone(),
        &h.assembly.clone(),
        &h.vxastro.clone(),
        &h.emission_controller.clone(),
        &h.astroport_router_contract_address(),
        Some(h.tribute_market_contract_address().to_string()),
        &Denom::Eclip.to_string(),
        &h.astro.clone(),
        &h.xastro.clone(),
        &Denom::EclipAstro.to_string(),
        GENESIS_EPOCH_START_DATE,
        EPOCH_LENGTH,
        VOTE_DELAY,
    );

    h.eclipsepad_staking_try_update_config(
        owner,
        None,
        Some(h.voter_contract_address()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();

    for token in [Denom::Eclip, Denom::Astro] {
        h.mint_tokens(owner, &coins(1_000 * INITIAL_LIQUIDITY, token.to_string()))
            .unwrap();
    }

    for user in Acc::iter() {
        for token in [
            &Denom::Eclip.to_string(),
            &Denom::Astro.to_string(),
            &h.xastro.clone(),
        ] {
            h.app
                .send_tokens(
                    owner.to_owned(),
                    h.acc(user),
                    &coins(INITIAL_LIQUIDITY / 10, token),
                )
                .unwrap();
        }
    }

    h.mint_tokens(
        &h.minter_contract_address(),
        &coins(1_000 * INITIAL_LIQUIDITY, Denom::EclipAstro.to_string()),
    )
    .unwrap();

    h.minter_try_register_currency(
        owner,
        &Currency::new(&Token::new_native(&Denom::EclipAstro.to_string()), 6),
        &h.voter_contract_address(),
    )
    .unwrap();

    // whitelist pools
    let prefix = "neutron";
    let astro_pool = "neutron1f37v0rdvrred27tlqqcpkrqpzfv6ddr2dxqan2";
    let astro_ibc_denom = "ibc/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    let channel = "channel-1";

    h.add_outpost(
        prefix,
        OutpostInfo {
            astro_denom: astro_ibc_denom.to_string(),
            astro_pool_config: Some(AstroPoolConfig {
                astro_pool: astro_pool.to_string(),
                constant_emissions: Uint128::one(),
            }),
            params: Some(OutpostParams {
                emissions_controller: h.emission_controller.to_string(),
                ics20_channel: channel.to_string(),
                voting_channel: channel.to_string(),
            }),
        },
    )
    .unwrap();

    for pool in [Pool::EclipAtom, Pool::NtrnAtom, Pool::AstroAtom] {
        const POOL_LIQUIDITY: u128 = 100_000_000_000_000_000_000;

        // create pair
        let (denom_a, denom_b) = &pool.get_pair();
        let pair_info = h.create_pair(denom_a, denom_b);
        let pair = &Addr::unchecked(pair_info.liquidity_token);
        // add pair in pool_list
        h.pool_list.push((pool, pair.to_owned()));
        // add in wl
        h.whitelist(owner, pair, &coins(1_000_000, astro)).unwrap();

        // provide liquidity
        h.mint_tokens(owner, &coins(POOL_LIQUIDITY, denom_a))
            .unwrap();
        h.mint_tokens(owner, &coins(POOL_LIQUIDITY, denom_b))
            .unwrap();

        h.provide_liquidity(
            owner,
            pair_info.contract_addr,
            denom_a,
            denom_b,
            POOL_LIQUIDITY,
        )
        .unwrap();
    }

    // initial voter's voting power is 100_000_000
    h.voter_try_swap_to_eclip_astro(owner, 100_000_000, astro)
        .unwrap();

    // add routes: [atom-eclip], [ntrn-atom, atom-eclip], [astro-atom, atom-eclip]
    h.voter_try_update_route_list(
        owner,
        &[
            RouteListItem::new(Denom::Atom, &[RouteItem::new(Denom::Atom, Denom::Eclip)]),
            RouteListItem::new(
                Denom::Ntrn,
                &[
                    RouteItem::new(Denom::Ntrn, Denom::Atom),
                    RouteItem::new(Denom::Atom, Denom::Eclip),
                ],
            ),
            RouteListItem::new(
                Denom::Astro,
                &[
                    RouteItem::new(Denom::Astro, Denom::Atom),
                    RouteItem::new(Denom::Atom, Denom::Eclip),
                ],
            ),
        ],
    )
    .unwrap();

    // add bribes in tribute market
    let bribes_allocation: Vec<BribesAllocationItem> = vec![
        BribesAllocationItem::new(
            h.pool(Pool::EclipAtom),
            &vec![
                (100 * INITIAL_LIQUIDITY, Denom::Eclip),
                (100 * INITIAL_LIQUIDITY, Denom::Atom),
            ],
        ),
        BribesAllocationItem::new(
            h.pool(Pool::NtrnAtom),
            &vec![
                (200 * INITIAL_LIQUIDITY, Denom::Ntrn),
                (120 * INITIAL_LIQUIDITY, Denom::Atom),
            ],
        ),
        BribesAllocationItem::new(
            h.pool(Pool::AstroAtom),
            &vec![(100 * INITIAL_LIQUIDITY, Denom::Astro)],
        ),
    ];

    h.tribute_market_try_set_bribes_allocation(owner, &bribes_allocation)
        .unwrap();

    h
}

#[test]
fn calc_voter_bribe_allocation_math() -> StdResult<()> {
    let tribute_market_bribe_allocation = &[
        BribesAllocationItem::new(Pool::EclipAtom, &[(100, Denom::Atom), (100, Denom::Eclip)]),
        BribesAllocationItem::new(Pool::NtrnAtom, &[(200, Denom::Ntrn), (120, Denom::Atom)]),
        BribesAllocationItem::new(Pool::AstroAtom, &[(100, Denom::Ntrn)]),
    ];

    let voter_to_tribute_voting_power_ratio_allocation: Vec<(String, Decimal)> = vec![
        (Pool::EclipAtom.to_string(), str_to_dec("0.5")),
        (Pool::NtrnAtom.to_string(), str_to_dec("0.25")),
    ];

    let voter_rewards_bribe_allocation = &calc_pool_info_list_with_rewards(
        &[
            PoolInfoItem::new(Pool::EclipAtom, "0", &[]),
            PoolInfoItem::new(Pool::NtrnAtom, "0", &[]),
        ],
        tribute_market_bribe_allocation,
        &voter_to_tribute_voting_power_ratio_allocation,
    );

    assert_that(&voter_rewards_bribe_allocation).is_equal_to(&vec![
        PoolInfoItem::new(Pool::EclipAtom, "0", &[(50, "atom"), (50, "eclip")]),
        PoolInfoItem::new(Pool::NtrnAtom, "0", &[(50, "ntrn"), (30, "atom")]),
    ]);

    Ok(())
}

#[test]
fn full_cycle() -> StdResult<()> {
    let mut h = prepare_helper();
    let ControllerHelper { xastro, .. } = &ControllerHelper::new();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let owner = &h.acc(Acc::Owner);
    let dao = &h.acc(Acc::Dao);
    // electors
    let alice = &h.acc(Acc::Alice);
    let ruby = &h.acc(Acc::Ruby);
    // delegators
    let bob = &h.acc(Acc::Bob);
    let vlad = &h.acc(Acc::Vlad);
    // slackers
    let john = &h.acc(Acc::John);
    // individual voters
    let kate = &h.acc(Acc::Kate);

    let weights_alice = &vec![
        WeightAllocationItem::new(eclip_atom, "0.4"),
        WeightAllocationItem::new(astro_atom, "0.6"),
    ];
    let weights_ruby = &vec![
        WeightAllocationItem::new(ntrn_atom, "0.6"),
        WeightAllocationItem::new(astro_atom, "0.4"),
    ];
    let weights_electors_expected = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(astro_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
    ];
    let weights_dao = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // stake and lock
    for (user, amount) in [
        (alice, 500),
        (ruby, 500),
        (bob, 500),
        (vlad, 1_500),
        (john, 3_000),
    ] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_place_vote(alice, weights_alice)?;
    h.voter_try_place_vote(ruby, weights_ruby)?;
    h.voter_try_delegate(bob)?;
    h.voter_try_delegate(vlad)?;
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // final voting
    let date_config = h.voter_query_date_config()?;
    let epoch_length = date_config.epoch_length;
    let vote_delay = date_config.vote_delay;
    h.wait(vote_delay);
    // vote
    h.voter_try_push()?;

    // check voting power and votes allocation
    let tribute_voting_power = h.total_vp(None)?;
    let voted_pools = h.query_voted_pools(None)?;

    let block_time = 1716163200;
    assert_that(&tribute_voting_power.u128()).is_equal_to(100_000_000);
    assert_that(&voted_pools).is_equal_to(vec![
        (
            eclip_atom.to_string(),
            VotedPoolInfo {
                init_ts: block_time,
                voting_power: Uint128::new(33_000_000),
            },
        ),
        (
            ntrn_atom.to_string(),
            VotedPoolInfo {
                init_ts: block_time,
                voting_power: Uint128::new(30_000_000),
            },
        ),
        (
            astro_atom.to_string(),
            VotedPoolInfo {
                init_ts: block_time,
                voting_power: Uint128::new(37_000_000),
            },
        ),
    ]);

    // vote interacting with emissions_controller directly
    h.mint_tokens(kate, &coins(100 * INITIAL_LIQUIDITY, xastro))
        .map_err(parse_err)?;
    h.lock(kate, 100 * INITIAL_LIQUIDITY).map_err(parse_err)?;
    h.vote(
        kate,
        &[
            (h.pool(Pool::EclipAtom).to_string(), str_to_dec("0.5")),
            (h.pool(Pool::AstroAtom).to_string(), str_to_dec("0.5")),
        ],
    )
    .map_err(parse_err)?;

    // check voting power and votes allocation
    let kate_voting_power = h.user_vp(kate, None)?;
    let tribute_voting_power = h.total_vp(None)?;
    let voted_pools = h.query_voted_pools(None)?;

    let block_time = 1716163200;
    assert_that(&kate_voting_power.u128()).is_equal_to(199_009_900);
    assert_that(&tribute_voting_power.u128()).is_equal_to(299_009_900);
    assert_that(&voted_pools).is_equal_to(vec![
        (
            eclip_atom.to_string(),
            VotedPoolInfo {
                init_ts: block_time,
                voting_power: Uint128::new(132_504_950),
            },
        ),
        (
            ntrn_atom.to_string(),
            VotedPoolInfo {
                init_ts: block_time,
                voting_power: Uint128::new(30_000_000),
            },
        ),
        (
            astro_atom.to_string(),
            VotedPoolInfo {
                init_ts: block_time,
                voting_power: Uint128::new(136_504_950),
            },
        ),
    ]);

    // allocate rewards
    h.wait(epoch_length - vote_delay + 1);
    h.tribute_market_try_allocate_rewards(owner, &[&h.voter_contract_address(), kate])?;

    // check rewards allocation
    let voter_rewards = h.tribute_market_query_rewards(h.voter_contract_address())?;
    let kate_rewards = h.tribute_market_query_rewards(kate)?;

    // tribute bribe allocation
    // eclip-atom: 100 atom, 100 eclip
    // ntrn-atom: 200 ntrn, 120 atom
    // astro-atom: 100 astro
    //
    // voter voting power: 100_000_000
    // kate voting power: 199_009_900
    //
    // voter votes allocation
    // eclip-atom: 0.33
    // ntrn-atom: 0.3
    // astro-atom: 0.37
    //
    // kate votes allocation
    // eclip-atom: 0.5
    // astro-atom: 0.5
    //
    // user_rewards_per_denom = sum_over_pools(tribute_rewards * (user_vp * user_votes) / sum_over_users(user_vp * user_votes))
    //
    // voter rewards
    // astro = 100 * (100_000_000 * 0.37) / ((100_000_000 * 0.37) + (199_009_900 * 0.5)) = 27.105244
    // atom = 100 * (100_000_000 * 0.33) / ((100_000_000 * 0.33) + (199_009_900 * 0.5)) +
    //        120 * (100_000_000 * 0.3) / ((100_000_000 * 0.3) + 0) = 144.904729
    // eclip = 100 * (100_000_000 * 0.33) / ((100_000_000 * 0.33) + (199_009_900 * 0.5)) = 24.904729
    // ntrn = 200 * (100_000_000 * 0.3) / ((100_000_000 * 0.3) + 0) = 200
    //
    // kate rewards
    // astro = 100 * (199_009_900 * 0.5) / ((100_000_000 * 0.37) + (199_009_900 * 0.5)) = 72.894755
    // atom = 100 * (199_009_900 * 0.5) / ((100_000_000 * 0.33) + (199_009_900 * 0.5)) = 75.095270
    // eclip = 100 * (199_009_900 * 0.5) / ((100_000_000 * 0.33) + (199_009_900 * 0.5)) = 75.095270
    assert_that(&voter_rewards).is_equal_to(vec![
        (Uint128::new(27_105_244), Denom::Astro.to_string()),
        (Uint128::new(144_904_729), Denom::Atom.to_string()),
        (Uint128::new(24_904_729), Denom::Eclip.to_string()),
        (Uint128::new(200_000_000), Denom::Ntrn.to_string()),
    ]);
    assert_that(&kate_rewards).is_equal_to(vec![
        (Uint128::new(72_894_755), Denom::Astro.to_string()),
        (Uint128::new(75_095_270), Denom::Atom.to_string()),
        (Uint128::new(75_095_270), Denom::Eclip.to_string()),
    ]);

    let voter_rewards_from_voter = h.voter_query_rewards()?;
    assert_that(&voter_rewards).is_equal_to(voter_rewards_from_voter);

    // claim rewards
    let treasury_balance_before = h.query_balance(dao, Denom::Eclip);
    let voter_astro_before = h.query_balance(&h.voter_contract_address(), Denom::Astro);
    let voter_atom_before = h.query_balance(&h.voter_contract_address(), Denom::Atom);
    let voter_eclip_before = h.query_balance(&h.voter_contract_address(), Denom::Eclip);
    let voter_ntrn_before = h.query_balance(&h.voter_contract_address(), Denom::Ntrn);

    // claim
    h.voter_try_push()?;

    let voter_astro_after = h.query_balance(&h.voter_contract_address(), Denom::Astro);
    let voter_atom_after = h.query_balance(&h.voter_contract_address(), Denom::Atom);
    let voter_eclip_after = h.query_balance(&h.voter_contract_address(), Denom::Eclip);
    let voter_ntrn_after = h.query_balance(&h.voter_contract_address(), Denom::Ntrn);

    assert_that(&(voter_astro_after - voter_astro_before)).is_equal_to(27_105_244);
    assert_that(&(voter_atom_after - voter_atom_before)).is_equal_to(144_904_729);
    assert_that(&(voter_eclip_after - voter_eclip_before)).is_equal_to(24_904_729);
    assert_that(&(voter_ntrn_after - voter_ntrn_before)).is_equal_to(200_000_000);

    // check vote results
    let block_time = h.get_block_time();
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 4_000),
        total_votes: vec![],
        vote_results: vec![VoteResults {
            epoch_id: 1,
            end_date: 1717372800,
            elector_essence: Uint128::new(3_400),
            dao_essence: Uint128::new(2_600),
            slacker_essence: Uint128::new(3_000),
            elector_weights: weights_electors_expected.to_owned(),
            dao_weights: weights_dao.to_owned(),
            dao_treasury_eclip_rewards: Uint128::new(0),
            dao_delegators_eclip_rewards: Uint128::new(0),
            pool_info_list: vec![
                PoolInfoItem::new(
                    eclip_atom,
                    "0.33",
                    &[
                        (24_904_729, &Denom::Eclip.to_string()),
                        (24_904_729, &Denom::Atom.to_string()),
                    ],
                ),
                PoolInfoItem::new(
                    astro_atom,
                    "0.37",
                    &[(27_105_244, &Denom::Astro.to_string())],
                ),
                PoolInfoItem::new(
                    ntrn_atom,
                    "0.3",
                    &[
                        (200_000_000, &Denom::Ntrn.to_string()),
                        (120_000_000, &Denom::Atom.to_string()),
                    ],
                ),
            ],
        }],
    });

    // swap dao rewards to eclip
    h.voter_try_push()?;

    // check vote results
    let block_time = h.get_block_time();
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 4_000),
        total_votes: vec![],
        vote_results: vec![VoteResults {
            epoch_id: 1,
            end_date: 1717372800,
            elector_essence: Uint128::new(3_400),
            dao_essence: Uint128::new(2_600),
            slacker_essence: Uint128::new(3_000),
            elector_weights: weights_electors_expected.to_owned(),
            dao_weights: weights_dao.to_owned(),
            // dao_eclip_rewards = sum_over_denoms(dao_rewards_per_denom)
            // dao_rewards_per_denom = sum_over_pools(voter_rewards * (dao_essence * dao_weight) / (voter_essence * voter_weight))
            //
            // dao_eclip_rewards = 16_351_589 + 16_351_589 + 86_666_666 + 52_000_000 + 6_348_976 = 177_718_820
            // dao_treasury_eclip_rewards = 0.8 * 177_718_820 = 35_543_763
            // dao_delegators_eclip_rewards = 177_718_820 - 35_543_763 = 142_175_051
            dao_treasury_eclip_rewards: Uint128::new(35_543_763),
            dao_delegators_eclip_rewards: Uint128::new(142_175_051),
            // dao_rewards_per_denom = sum_over_pools(voter_rewards * (dao_essence * dao_weight) / (voter_essence * voter_weight))
            // elector_rewards_per_denom = rewards_per_denom - dao_rewards_per_denom
            //
            // eclip_in_eclip_atom = 24_904_729 - 24_904_729 * (2_600 * 0.5) / (6_000 * 0.33) = 24_904_729 - 16_351_589 = 8_553_140
            // atom_in_eclip_atom = 24_904_729 - 24_904_729 * (2_600 * 0.5) / (6_000 * 0.33) = 24_904_729 - 16_351_589 = 8_553_140
            //
            // ntrn_in_ntrn_atom = 200_000_000 - 200_000_000 * (2_600 * 0.3) / (6_000 * 0.3) = 200_000_000 - 86_666_666 = 113_333_334
            // atom_in_ntrn_atom = 120_000_000 - 120_000_000 * (2_600 * 0.3) / (6_000 * 0.3) = 120_000_000 - 52_000_000 = 68_000_000
            //
            // astro_in_astro_atom = 27_105_244 - 27_105_244 * (2_600 * 0.2) / (6_000 * 0.37) = 27_105_244 - 6_348_976 = 20_756_268
            pool_info_list: vec![
                PoolInfoItem::new(
                    eclip_atom,
                    "0.33",
                    &[
                        (8_553_140, &Denom::Eclip.to_string()),
                        (8_553_140, &Denom::Atom.to_string()),
                    ],
                ),
                PoolInfoItem::new(
                    astro_atom,
                    "0.37",
                    &[(20_756_268, &Denom::Astro.to_string())],
                ),
                PoolInfoItem::new(
                    ntrn_atom,
                    "0.3",
                    &[
                        (113_333_334, &Denom::Ntrn.to_string()),
                        (68_000_001, &Denom::Atom.to_string()),
                    ],
                ),
            ],
        }],
    });

    // check dao treasury balance
    let treasury_balance_after = h.query_balance(dao, Denom::Eclip);
    assert_that(&(treasury_balance_after - treasury_balance_before)).is_equal_to(35_543_763);

    // query user rewards
    let alice_rewards = h.voter_query_user(alice, None)?.rewards.value;
    let ruby_rewards = h.voter_query_user(ruby, None)?.rewards.value;
    let bob_rewards = h.voter_query_user(bob, None)?.rewards.value;
    let vlad_rewards = h.voter_query_user(vlad, None)?.rewards.value;
    let john_rewards = h.voter_query_user(john, None)?.rewards.value;

    // elector_personal_rewards = sum_over_pools(elector_personal_rewards_per_pool)
    // elector_personal_rewards_per_pool = elector_rewards * (personal_elector_essence * personal_weight) /
    //     ((elector_essence - 0.8 * slacker_essence) * elector_weight)
    //
    // alice_astro_in_astro_atom = 20_756_268 * (500 * 0.6) / ((3_400 - 0.8 * 3_000) * 0.5) = 12_453_760
    // alice_atom_in_eclip_atom = 8_553_140 * (500 * 0.4) / ((3_400 - 0.8 * 3_000) * 0.2) = 8_553_140
    // alice_eclip_in_eclip_atom = 8_553_140 * (500 * 0.4) / ((3_400 - 0.8 * 3_000) * 0.2) = 8_553_140
    assert_that(&alice_rewards).is_equal_to(vec![
        (Uint128::new(12_453_760), Denom::Astro.to_string()),
        (Uint128::new(8_553_140), Denom::Atom.to_string()),
        (Uint128::new(8_553_140), Denom::Eclip.to_string()),
    ]);
    // ruby_astro_in_astro_atom = 20_756_268 * (500 * 0.4) / ((3_400 - 0.8 * 3_000) * 0.5) = 8_302_507
    // ruby_atom_in_ntrn_atom = 68_000_001 * (500 * 0.6) / ((3_400 - 0.8 * 3_000) * 0.3) = 68_000_001
    // ruby_ntrn_in_ntrn_atom = 113_333_334 * (500 * 0.6) / ((3_400 - 0.8 * 3_000) * 0.3) = 113_333_334
    assert_that(&ruby_rewards).is_equal_to(vec![
        (Uint128::new(8_302_507), Denom::Astro.to_string()),
        (Uint128::new(68_000_001), Denom::Atom.to_string()),
        (Uint128::new(113_333_334), Denom::Ntrn.to_string()),
    ]);
    // delegator_rewards = dao_delegators_eclip_rewards * delegator_essence / (dao_essence - 0.2 * slacker_essence)
    //
    // bob_rewards = 142_175_051 * 500 / (2_600 - 0.2 * 3_000) = 35_543_762
    assert_that(&bob_rewards)
        .is_equal_to(vec![(Uint128::new(35_543_762), Denom::Eclip.to_string())]);
    // vlad_rewards = 142_175_051 * 1_500 / (2_600 - 0.2 * 3_000) = 106_631_288
    assert_that(&vlad_rewards)
        .is_equal_to(vec![(Uint128::new(106_631_288), Denom::Eclip.to_string())]);
    // slacker_rewards = 0
    assert_that(&john_rewards).is_equal_to(vec![]);

    // claim user rewards
    let alice_astro_balance_before = h.query_balance(alice, Denom::Astro);
    let alice_atom_balance_before = h.query_balance(alice, Denom::Atom);
    let alice_eclip_balance_before = h.query_balance(alice, Denom::Eclip);
    let ruby_astro_balance_before = h.query_balance(ruby, Denom::Astro);
    let ruby_atom_balance_before = h.query_balance(ruby, Denom::Atom);
    let ruby_ntrn_balance_before = h.query_balance(ruby, Denom::Ntrn);
    let bob_eclip_balance_before = h.query_balance(bob, Denom::Eclip);
    let vlad_eclip_balance_before = h.query_balance(vlad, Denom::Eclip);

    for user in [alice, ruby, bob, vlad] {
        h.voter_try_claim_rewards(user)?;
    }

    let alice_astro_balance_after = h.query_balance(alice, Denom::Astro);
    let alice_atom_balance_after = h.query_balance(alice, Denom::Atom);
    let alice_eclip_balance_after = h.query_balance(alice, Denom::Eclip);
    let ruby_astro_balance_after = h.query_balance(ruby, Denom::Astro);
    let ruby_atom_balance_after = h.query_balance(ruby, Denom::Atom);
    let ruby_ntrn_balance_after = h.query_balance(ruby, Denom::Ntrn);
    let bob_eclip_balance_after = h.query_balance(bob, Denom::Eclip);
    let vlad_eclip_balance_after = h.query_balance(vlad, Denom::Eclip);

    assert_that(&(alice_astro_balance_after - alice_astro_balance_before)).is_equal_to(12_453_760);
    assert_that(&(alice_atom_balance_after - alice_atom_balance_before)).is_equal_to(8_553_140);
    assert_that(&(alice_eclip_balance_after - alice_eclip_balance_before)).is_equal_to(8_553_140);
    assert_that(&(ruby_astro_balance_after - ruby_astro_balance_before)).is_equal_to(8_302_507);
    assert_that(&(ruby_atom_balance_after - ruby_atom_balance_before)).is_equal_to(68_000_001);
    assert_that(&(ruby_ntrn_balance_after - ruby_ntrn_balance_before)).is_equal_to(113_333_334);
    assert_that(&(bob_eclip_balance_after - bob_eclip_balance_before)).is_equal_to(35_543_762);
    assert_that(&(vlad_eclip_balance_after - vlad_eclip_balance_before)).is_equal_to(106_631_288);

    Ok(())
}

#[test]
fn router_default() -> StdResult<()> {
    let mut h = prepare_helper();
    let alice = &h.acc(Acc::Alice);

    let res =
        h.astroport_router_query_simulate_swap_operations(1_000, Denom::Eclip, Denom::Atom)?;

    let alice_atom_balance_before = h.query_balance(alice, Denom::Atom);
    h.astroport_router_try_execute_swap_operations(
        alice,
        Denom::Eclip,
        1_000,
        &vec![SwapOperation::AstroSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: Denom::Eclip.to_string(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: Denom::Atom.to_string(),
            },
        }],
    )?;

    let alice_atom_balance_after = h.query_balance(alice, Denom::Atom);
    assert_that(&(alice_atom_balance_after - alice_atom_balance_before))
        .is_equal_to(res.amount.u128());

    Ok(())
}

#[test]
fn router_batch_swap() -> StdResult<()> {
    let mut h = prepare_helper();
    let alice = &h.acc(Acc::Alice);

    // mint tokens
    for denom in [Denom::Ntrn, Denom::Atom] {
        h.mint_tokens(alice, &coins(1_000, denom.to_string()))
            .unwrap();
    }

    // check balances
    let alice_ntrn_balance_before = h.query_balance(alice, Denom::Ntrn);
    let alice_atom_balance_before = h.query_balance(alice, Denom::Atom);
    let alice_eclip_balance_before = h.query_balance(alice, Denom::Eclip);

    assert_that(&alice_ntrn_balance_before).is_equal_to(1_000);
    assert_that(&alice_atom_balance_before).is_equal_to(1_000);
    assert_that(&alice_eclip_balance_before).is_equal_to(100_000);

    // try to swap [600 NTRN, 400 ATOM] -> 1_000 ECLIP using [NTRN-ATOM, ATOM-ECLIP] route
    // 600 NTRN -> 600 ATOM
    // (400 + 600) ATOM -> 1_000 ECLIP
    h.astroport_router_try_execute_batch_swap(
        alice,
        &vec![
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: Denom::Ntrn.to_string(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: Denom::Atom.to_string(),
                },
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: Denom::Atom.to_string(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: Denom::Eclip.to_string(),
                },
            },
        ],
        &vec![(600, Denom::Ntrn), (400, Denom::Atom)],
    )?;

    // check balances
    let alice_ntrn_balance_after = h.query_balance(alice, Denom::Ntrn);
    let alice_atom_balance_after = h.query_balance(alice, Denom::Atom);
    let alice_eclip_balance_after = h.query_balance(alice, Denom::Eclip);

    assert_that(&alice_ntrn_balance_after).is_equal_to(400);
    assert_that(&alice_atom_balance_after).is_equal_to(600);
    assert_that(&alice_eclip_balance_after).is_equal_to(100_998);

    Ok(())
}

#[test]
fn swap_to_eclip_astro_default() -> StdResult<()> {
    let mut h = prepare_helper();
    let ControllerHelper { astro, xastro, .. } = &ControllerHelper::new();
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);

    let alice_astro = h.query_balance(alice, astro);
    let alice_xastro = h.query_balance(alice, xastro);
    let alice_eclip_astro = h.query_balance(alice, Denom::EclipAstro);
    assert_that(&alice_astro).is_equal_to(100_000);
    assert_that(&alice_xastro).is_equal_to(100_000);
    assert_that(&alice_eclip_astro).is_equal_to(0);

    let bob_astro = h.query_balance(bob, astro);
    let bob_xastro = h.query_balance(bob, xastro);
    let bob_eclip_astro = h.query_balance(bob, Denom::EclipAstro);
    assert_that(&bob_astro).is_equal_to(100_000);
    assert_that(&bob_xastro).is_equal_to(100_000);
    assert_that(&bob_eclip_astro).is_equal_to(0);

    h.voter_try_swap_to_eclip_astro(alice, 1_000, astro)?;
    h.voter_try_swap_to_eclip_astro(bob, 1_000, xastro)?;

    let alice_astro = h.query_balance(alice, astro);
    let alice_xastro = h.query_balance(alice, xastro);
    let alice_eclip_astro = h.query_balance(alice, Denom::EclipAstro);
    assert_that(&alice_astro).is_equal_to(99_000);
    assert_that(&alice_xastro).is_equal_to(100_000);
    assert_that(&alice_eclip_astro).is_equal_to(1_000);

    let bob_astro = h.query_balance(bob, astro);
    let bob_xastro = h.query_balance(bob, xastro);
    let bob_eclip_astro = h.query_balance(bob, Denom::EclipAstro);
    assert_that(&bob_astro).is_equal_to(100_000);
    assert_that(&bob_xastro).is_equal_to(99_000);
    assert_that(&bob_eclip_astro).is_equal_to(1_000);

    Ok(())
}

#[test]
fn essence_info_math() -> StdResult<()> {
    let essence_info_1 = EssenceInfo::new::<u128>(20, 500_000_000, 40);
    let essence_info_2 = EssenceInfo::new::<u128>(10, 250_000_000, 20);

    assert_that(&essence_info_2.add(&essence_info_2)).is_equal_to(&essence_info_1);
    assert_that(&essence_info_1.sub(&essence_info_2)).is_equal_to(&essence_info_2);
    assert_that(&essence_info_1.sub(&essence_info_1).is_zero()).is_equal_to(true);
    // (20 * 50_000_000 - 500_000_000) / 31_536_000 + 40 = 55
    assert_that(&essence_info_1.capture(50_000_000).u128()).is_equal_to(55);
    assert_that(&essence_info_1.scale(str_to_dec("0.5"))).is_equal_to(essence_info_2);

    Ok(())
}

#[test]
fn essence_allocation_math() -> StdResult<()> {
    let h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let essence_info = &EssenceInfo::new::<u128>(20, 500_000_000, 40);
    let weights = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.5"),
    ];
    let expected_essence_allocation = vec![
        EssenceAllocationItem::new(eclip_atom, &EssenceInfo::new::<u128>(4, 100_000_000, 8)),
        EssenceAllocationItem::new(ntrn_atom, &EssenceInfo::new::<u128>(6, 150_000_000, 12)),
        EssenceAllocationItem::new(astro_atom, &EssenceInfo::new::<u128>(10, 250_000_000, 20)),
    ];

    let essence_allocation = calc_essence_allocation(essence_info, weights);
    assert_that(&essence_allocation).is_equal_to(expected_essence_allocation);

    Ok(())
}

#[test]
fn updated_essence_allocation_math() -> StdResult<()> {
    let h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let essence_info = &EssenceInfo::new::<u128>(20, 500_000_000, 40);
    let alice_weights = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.5"),
    ];
    let bob_weights_initial = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.5"),
    ];
    let bob_weights_equal = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];
    let bob_weights_small = &vec![
        WeightAllocationItem::new(eclip_atom, "0.7"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
    ];
    let bob_weights_large = &vec![
        WeightAllocationItem::new(eclip_atom, "0.1"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.3"),
        WeightAllocationItem::new(Pool::UsdcAtom, "0.3"),
    ];

    let essence_allocation_alice = &calc_essence_allocation(essence_info, alice_weights);
    let essence_allocation_bob_initial =
        &calc_essence_allocation(essence_info, bob_weights_initial);
    let essence_allocation_bob_equal = &calc_essence_allocation(essence_info, bob_weights_equal);
    let essence_allocation_bob_small = &calc_essence_allocation(essence_info, bob_weights_small);
    let essence_allocation_bob_large = &calc_essence_allocation(essence_info, bob_weights_large);

    // get initial total essence allocation
    let essence_allocation = calc_updated_essence_allocation(
        essence_allocation_alice,
        essence_allocation_bob_initial,
        &vec![],
    );
    assert_that(&essence_allocation).is_equal_to(vec![
        EssenceAllocationItem::new(eclip_atom, &EssenceInfo::new::<u128>(8, 200_000_000, 16)),
        EssenceAllocationItem::new(ntrn_atom, &EssenceInfo::new::<u128>(12, 300_000_000, 24)),
        EssenceAllocationItem::new(astro_atom, &EssenceInfo::new::<u128>(20, 500_000_000, 40)),
    ]);

    // update bob allocation - equal amount of pools
    // (20, 500_000_000, 40) * (0.2, 0.3, 0.5) + (20, 500_000_000, 40) * (0.5, 0.3, 0.2) =
    // (20, 500_000_000, 40) * (0.7, 0.6, 0.7) =
    // [(14, 350_000_000, 28), (12, 300_000_000, 24), (14, 350_000_000, 28)]
    assert_that(&calc_updated_essence_allocation(
        &essence_allocation,
        essence_allocation_bob_equal,
        essence_allocation_bob_initial,
    ))
    .is_equal_to(vec![
        EssenceAllocationItem::new(eclip_atom, &EssenceInfo::new::<u128>(14, 350_000_000, 28)),
        EssenceAllocationItem::new(ntrn_atom, &EssenceInfo::new::<u128>(12, 300_000_000, 24)),
        EssenceAllocationItem::new(astro_atom, &EssenceInfo::new::<u128>(14, 350_000_000, 28)),
    ]);

    // update bob allocation - less amount of pools
    // [(4, 100_000_000, 8), (6, 150_000_000, 12), (10, 250_000_000, 20)] +
    // [(14, 350_000_000, 28), (6, 150_000_000, 12), (0, 0, 0)] =
    // [(18, 450_000_000, 36), (12, 300_000_000, 24), (10, 250_000_000, 20)]
    assert_that(&calc_updated_essence_allocation(
        &essence_allocation,
        essence_allocation_bob_small,
        essence_allocation_bob_initial,
    ))
    .is_equal_to(vec![
        EssenceAllocationItem::new(eclip_atom, &EssenceInfo::new::<u128>(18, 450_000_000, 36)),
        EssenceAllocationItem::new(ntrn_atom, &EssenceInfo::new::<u128>(12, 300_000_000, 24)),
        EssenceAllocationItem::new(astro_atom, &EssenceInfo::new::<u128>(10, 250_000_000, 20)),
    ]);

    // update bob allocation - greater amount of pools
    // [(4, 100_000_000, 8), (6, 150_000_000, 12), (10, 250_000_000, 20)] +
    // [(2, 50_000_000, 4), (6, 150_000_000, 12), (6, 150_000_000, 12), (6, 150_000_000, 12)] =
    // [(6, 150_000_000, 12), (12, 300_000_000, 24), (16, 400_000_000, 32), (6, 150_000_000, 12)]
    assert_that(&calc_updated_essence_allocation(
        &essence_allocation,
        essence_allocation_bob_large,
        essence_allocation_bob_initial,
    ))
    .is_equal_to(vec![
        EssenceAllocationItem::new(eclip_atom, &EssenceInfo::new::<u128>(6, 150_000_000, 12)),
        EssenceAllocationItem::new(ntrn_atom, &EssenceInfo::new::<u128>(12, 300_000_000, 24)),
        EssenceAllocationItem::new(astro_atom, &EssenceInfo::new::<u128>(16, 400_000_000, 32)),
        EssenceAllocationItem::new(
            Pool::UsdcAtom,
            &EssenceInfo::new::<u128>(6, 150_000_000, 12),
        ),
    ]);

    Ok(())
}

#[test]
fn scaled_essence_allocation_math() -> StdResult<()> {
    let h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let essence_info = &EssenceInfo::new::<u128>(20, 500_000_000, 40);
    let weights = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.5"),
    ];

    let additional_essence = &EssenceInfo::new::<u128>(20, 500_000_000, 40);
    let additional_essence_fraction = str_to_dec("0.5");
    let scaled_essence_allocation = calc_scaled_essence_allocation(
        essence_info,
        weights,
        additional_essence,
        additional_essence_fraction,
    );
    // (20, 500_000_000, 40) * (0.2, 0.3, 0.5) * 1.5 =
    // [(6, 150_000_000, 12), (9, 225_000_000, 18), (15, 375_000_000, 30)]
    assert_that(&scaled_essence_allocation).is_equal_to(vec![
        EssenceAllocationItem::new(eclip_atom, &EssenceInfo::new::<u128>(6, 150_000_000, 12)),
        EssenceAllocationItem::new(ntrn_atom, &EssenceInfo::new::<u128>(9, 225_000_000, 18)),
        EssenceAllocationItem::new(astro_atom, &EssenceInfo::new::<u128>(15, 375_000_000, 30)),
    ]);

    Ok(())
}

#[test]
fn auto_updating_essence() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.5"),
    ];

    // stake
    for user in [alice, bob, john] {
        h.eclipsepad_staking_try_stake(user, 1_000, Denom::Eclip)?;
    }

    let essence_info_alice = h.voter_query_user(alice, None)?;
    let essence_info_bob = h.voter_query_user(bob, None)?;
    let essence_info_john = h.voter_query_user(john, None)?;

    assert_that(&essence_info_alice).is_equal_to(UserResponse {
        user_type: UserType::Slacker,
        essence_info: EssenceInfo::new::<u128>(1000, 1716163200000, 0),
        essence_value: Uint128::zero(),
        weights: vec![],
        rewards: RewardsInfo::default(),
    });
    assert_that(&essence_info_bob).is_equal_to(UserResponse {
        user_type: UserType::Slacker,
        essence_info: EssenceInfo::new::<u128>(1000, 1716163200000, 0),
        essence_value: Uint128::zero(),
        weights: vec![],
        rewards: RewardsInfo::default(),
    });
    assert_that(&essence_info_john).is_equal_to(UserResponse {
        user_type: UserType::Slacker,
        essence_info: EssenceInfo::new::<u128>(1000, 1716163200000, 0),
        essence_value: Uint128::zero(),
        weights: vec![],
        rewards: RewardsInfo::default(),
    });

    // take roles: alice - elector, bob - delegator, john - slacker
    h.voter_try_place_vote(alice, weights)?;
    h.voter_try_delegate(bob)?;

    let essence_info_alice = h.voter_query_user(alice, None)?;
    let essence_info_bob = h.voter_query_user(bob, None)?;
    let essence_info_john = h.voter_query_user(john, None)?;

    assert_that(&essence_info_alice).is_equal_to(UserResponse {
        user_type: UserType::Elector,
        essence_info: EssenceInfo::new::<u128>(1000, 1716163200000, 0),
        essence_value: Uint128::zero(),
        weights: weights.to_owned(),
        rewards: RewardsInfo::default(),
    });
    assert_that(&essence_info_bob).is_equal_to(UserResponse {
        user_type: UserType::Delegator,
        essence_info: EssenceInfo::new::<u128>(1000, 1716163200000, 0),
        essence_value: Uint128::zero(),
        weights: vec![],
        rewards: RewardsInfo::default(),
    });
    assert_that(&essence_info_john).is_equal_to(UserResponse {
        user_type: UserType::Slacker,
        essence_info: EssenceInfo::new::<u128>(1000, 1716163200000, 0),
        essence_value: Uint128::zero(),
        weights: vec![],
        rewards: RewardsInfo::default(),
    });

    // change essence for slacker, elector, delegator
    // lock
    for user in [alice, bob, john] {
        h.eclipsepad_staking_try_lock(user, 1_000, 4)?;
    }

    let essence_info_alice = h.voter_query_user(alice, None)?;
    let essence_info_bob = h.voter_query_user(bob, None)?;
    let essence_info_john = h.voter_query_user(john, None)?;

    assert_that(&essence_info_alice).is_equal_to(UserResponse {
        user_type: UserType::Elector,
        essence_info: EssenceInfo::new::<u128>(0, 0, 1000),
        essence_value: Uint128::new(1000),
        weights: weights.to_owned(),
        rewards: RewardsInfo::default(),
    });
    assert_that(&essence_info_bob).is_equal_to(UserResponse {
        user_type: UserType::Delegator,
        essence_info: EssenceInfo::new::<u128>(0, 0, 1000),
        essence_value: Uint128::new(1000),
        weights: vec![],
        rewards: RewardsInfo::default(),
    });
    assert_that(&essence_info_john).is_equal_to(UserResponse {
        user_type: UserType::Slacker,
        essence_info: EssenceInfo::new::<u128>(0, 0, 1000),
        essence_value: Uint128::new(1000),
        weights: vec![],
        rewards: RewardsInfo::default(),
    });

    Ok(())
}

#[test]
fn changing_weights_by_essence() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let dao = &h.acc(Acc::Dao);
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights_alice = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.5"),
    ];
    let weights_dao = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // stake and lock
    for user in [alice, bob, john] {
        h.eclipsepad_staking_try_stake(user, 1_000, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, 1_000, 4)?;
    }

    // place votes
    h.voter_try_place_vote(alice, weights_alice)?;
    h.voter_try_delegate(bob)?;
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    let essence_info_alice = h.voter_query_user(alice, None)?;
    let essence_info_dao = h.voter_query_dao_info(None)?;
    let voter_info = h.voter_query_voter_info(None)?;
    let block_time = h.get_block_time();

    assert_that(&essence_info_alice).is_equal_to(UserResponse {
        user_type: UserType::Elector,
        essence_info: EssenceInfo::new::<u128>(0, 0, 1_000),
        essence_value: Uint128::new(1_000),
        weights: weights_alice.to_owned(),
        rewards: RewardsInfo::default(),
    });
    assert_that(&essence_info_dao).is_equal_to(DaoResponse {
        essence_info: EssenceInfo::new::<u128>(0, 0, 1_000),
        essence_value: Uint128::new(1_000),
        weights: weights_dao.to_owned(),
    });
    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time: 1716163200,
        elector_votes: vec![
            EssenceAllocationItem::new(eclip_atom, &EssenceInfo::new::<u128>(0, 0, 200)),
            EssenceAllocationItem::new(ntrn_atom, &EssenceInfo::new::<u128>(0, 0, 300)),
            EssenceAllocationItem::new(astro_atom, &EssenceInfo::new::<u128>(0, 0, 500)),
        ],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 1_000),
        // (0.2, 0.3, 0.5) * (1_000 + 0.8 * 1_000) + (0.5, 0.3, 0.2) * (1_000 + 0.2 * 1_000) =
        // (360, 540, 900) + (600, 360, 240) = (960, 900, 1_140)
        total_votes: vec![
            EssenceAllocationItem::new(eclip_atom, &EssenceInfo::new::<u128>(0, 0, 960)),
            EssenceAllocationItem::new(ntrn_atom, &EssenceInfo::new::<u128>(0, 0, 900)),
            EssenceAllocationItem::new(astro_atom, &EssenceInfo::new::<u128>(0, 0, 1_140)),
        ],
        vote_results: vec![],
    });

    assert_that(&calc_weights_from_essence_allocation(&voter_info.elector_votes, block_time).1)
        .is_equal_to(weights_alice);
    assert_that(&calc_weights_from_essence_allocation(&voter_info.total_votes, block_time).1)
        .is_equal_to(vec![
            WeightAllocationItem::new(eclip_atom, "0.32"),
            WeightAllocationItem::new(ntrn_atom, "0.3"),
            WeightAllocationItem::new(astro_atom, "0.38"),
        ]);

    // change alice essence
    h.wait(1);
    h.eclipsepad_staking_try_stake(alice, 1_000, Denom::Eclip)?;
    h.eclipsepad_staking_try_lock(alice, 1_000, 4)?;

    let essence_info_alice = h.voter_query_user(alice, None)?;
    let essence_info_dao = h.voter_query_dao_info(None)?;
    let voter_info = h.voter_query_voter_info(None)?;
    let block_time = h.get_block_time();

    assert_that(&essence_info_alice).is_equal_to(UserResponse {
        user_type: UserType::Elector,
        essence_info: EssenceInfo::new::<u128>(0, 0, 2_000),
        essence_value: Uint128::new(2_000),
        weights: weights_alice.to_owned(),
        rewards: RewardsInfo::default(),
    });
    assert_that(&essence_info_dao).is_equal_to(DaoResponse {
        essence_info: EssenceInfo::new::<u128>(0, 0, 1_000),
        essence_value: Uint128::new(1_000),
        weights: weights_dao.to_owned(),
    });
    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![
            EssenceAllocationItem::new(eclip_atom, &EssenceInfo::new::<u128>(0, 0, 400)),
            EssenceAllocationItem::new(ntrn_atom, &EssenceInfo::new::<u128>(0, 0, 600)),
            EssenceAllocationItem::new(astro_atom, &EssenceInfo::new::<u128>(0, 0, 1_000)),
        ],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 1_000),
        // (0.2, 0.3, 0.5) * (2_000 + 0.8 * 1_000) + (0.5, 0.3, 0.2) * (1_000 + 0.2 * 1_000) =
        // (560, 840, 1_400) + (600, 360, 240) = (1_160, 1_200, 1_640)
        total_votes: vec![
            EssenceAllocationItem::new(eclip_atom, &EssenceInfo::new::<u128>(0, 0, 1_160)),
            EssenceAllocationItem::new(ntrn_atom, &EssenceInfo::new::<u128>(0, 0, 1_200)),
            EssenceAllocationItem::new(astro_atom, &EssenceInfo::new::<u128>(0, 0, 1_640)),
        ],
        vote_results: vec![],
    });

    assert_that(&calc_weights_from_essence_allocation(&voter_info.elector_votes, block_time).1)
        .is_equal_to(weights_alice);
    // (1_160, 1_200, 1_640) / 4_000 = (0.3, 0.3, 0.4)
    assert_that(&calc_weights_from_essence_allocation(&voter_info.total_votes, block_time).1)
        .is_equal_to(vec![
            WeightAllocationItem::new(eclip_atom, "0.29"),
            WeightAllocationItem::new(ntrn_atom, "0.3"),
            WeightAllocationItem::new(astro_atom, "0.41"),
        ]);

    Ok(())
}

#[test]
fn electors_delegators_slackers_dao_voting() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let dao = &h.acc(Acc::Dao);
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);
    let kate = &h.acc(Acc::Kate);
    let ruby = &h.acc(Acc::Ruby);
    let vlad = &h.acc(Acc::Vlad);

    let weights_alice = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.5"),
    ];
    let weights_bob = &vec![
        WeightAllocationItem::new(eclip_atom, "0.1"),
        WeightAllocationItem::new(ntrn_atom, "0.7"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];
    let weights_dao = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // stake and lock
    for (user, amount) in [
        (alice, 1_000),
        (bob, 2_000),
        (john, 3_000),
        (kate, 4_000),
        (ruby, 5_000),
        (vlad, 6_000),
    ] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_place_vote(alice, weights_alice)?;
    h.voter_try_place_vote(bob, weights_bob)?;
    h.voter_try_delegate(john)?;
    h.voter_try_delegate(kate)?;
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    // vote
    h.voter_try_push()?;

    let block_time = h.get_block_time();
    let voter_info = h.voter_query_voter_info(None)?;
    let voted_pools = h.query_voted_pools(None)?;

    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 14_000),
        total_votes: vec![],
        vote_results: vec![VoteResults {
            epoch_id: 1,
            end_date: 1717372800,
            // 3_000 + 0.8 * 11_000
            elector_essence: Uint128::new(11_800),
            // 7_000 + 0.2 * 11_000
            dao_essence: Uint128::new(9_200),
            slacker_essence: Uint128::new(11_000),
            elector_weights: vec![
                WeightAllocationItem::new(eclip_atom, "0.133333333333333333"),
                WeightAllocationItem::new(ntrn_atom, "0.566666666666666666"),
                WeightAllocationItem::new(astro_atom, "0.3"),
            ],
            dao_weights: weights_dao.to_owned(),
            dao_treasury_eclip_rewards: Uint128::new(0),
            dao_delegators_eclip_rewards: Uint128::new(0),
            // ((0.2, 0.3, 0.5) * 1_000 + (0.1, 0.7, 0.2) * 2_000) * ((3_000 + 0.8 * 11_000) / 3_000) +
            // (0.5, 0.3, 0.2) * (7_000 + 0.2 * 11_000) =
            // ((200, 300, 500) + (200, 1_400, 400)) * (11_800 / 3_000) + (0.5, 0.3, 0.2) * 9_200 =
            // (400, 1_700, 900) * 3.93 + (4_600 + 2_760 + 1_840) =
            // (6_173, 9_446, 5_379) = (0.294, 0.45, 0.256)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.293966379351397685", &[]),
                PoolInfoItem::new(ntrn_atom, "0.449830944330682413", &[]),
                PoolInfoItem::new(astro_atom, "0.2562026763179199", &[]),
            ],
        }],
    });

    let expected_voted_pools = vec![
        (
            eclip_atom.to_string(),
            VotedPoolInfo {
                init_ts: 1716163200,
                voting_power: Uint128::new(29396637),
            },
        ),
        (
            ntrn_atom.to_string(),
            VotedPoolInfo {
                init_ts: 1716163200,
                voting_power: Uint128::new(44983094),
            },
        ),
        (
            astro_atom.to_string(),
            VotedPoolInfo {
                init_ts: 1716163200,
                voting_power: Uint128::new(25620267),
            },
        ),
    ];

    assert_that(&voted_pools).matches(|x| x.iter().all(|y| expected_voted_pools.contains(y)));

    Ok(())
}

#[test]
fn electors_slackers_dao_voting() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let dao = &h.acc(Acc::Dao);
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights_alice = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.5"),
    ];
    let weights_dao = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_place_vote(alice, weights_alice)?;
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    // vote
    h.voter_try_push()?;

    let block_time = h.get_block_time();
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 6_000),
        total_votes: vec![],
        vote_results: vec![VoteResults {
            epoch_id: 1,
            end_date: 1717372800,
            // 1_000 + 0.8 * 5_000
            elector_essence: Uint128::new(5_000),
            // 0.2 * 5_000
            dao_essence: Uint128::new(1_000),
            slacker_essence: Uint128::new(5_000),
            elector_weights: weights_alice.to_owned(),
            dao_weights: weights_dao.to_owned(),
            dao_treasury_eclip_rewards: Uint128::new(0),
            dao_delegators_eclip_rewards: Uint128::new(0),
            // (0.2, 0.3, 0.5) * (1_000 + 0.8 * 5_000) + (0.5, 0.3, 0.2) * (0.2 * 5_000) =
            // (1_000, 1_500, 2_500) + (500, 300, 200) = (1_500, 1_800, 2_700) = (0.25, 0.3, 0.45)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.25", &[]),
                PoolInfoItem::new(ntrn_atom, "0.3", &[]),
                PoolInfoItem::new(astro_atom, "0.45", &[]),
            ],
        }],
    });

    Ok(())
}

#[test]
fn delegators_slackers_dao_voting() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let dao = &h.acc(Acc::Dao);
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights_dao = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_delegate(bob)?;
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    // vote
    h.voter_try_push()?;

    let block_time = h.get_block_time();
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 4_000),
        total_votes: vec![],
        vote_results: vec![VoteResults {
            epoch_id: 1,
            end_date: 1717372800,
            elector_essence: Uint128::new(3_200),
            // 2_000 + 0.2 * 4_000 = 2_800
            dao_essence: Uint128::new(2_800),
            slacker_essence: Uint128::new(4_000),
            elector_weights: vec![],
            dao_weights: weights_dao.to_owned(),
            dao_treasury_eclip_rewards: Uint128::new(0),
            dao_delegators_eclip_rewards: Uint128::new(0),
            // (0.5, 0.3, 0.2) * (2_000 + 0.2 * 4_000) =
            // (1_400, 740, 560) = (0.5, 0.3, 0.2)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.5", &[]),
                PoolInfoItem::new(ntrn_atom, "0.3", &[]),
                PoolInfoItem::new(astro_atom, "0.2", &[]),
            ],
        }],
    });

    Ok(())
}

#[test]
fn electors_delegators_dao_voting() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let dao = &h.acc(Acc::Dao);
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights_alice = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.5"),
    ];
    let weights_dao = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_place_vote(alice, weights_alice)?;
    h.voter_try_delegate(bob)?;
    h.voter_try_delegate(john)?;
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    // vote
    h.voter_try_push()?;

    let block_time = h.get_block_time();
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 1_000),
        total_votes: vec![],
        vote_results: vec![VoteResults {
            epoch_id: 1,
            end_date: 1717372800,
            elector_essence: Uint128::new(1_000),
            dao_essence: Uint128::new(5_000),
            slacker_essence: Uint128::new(0),
            elector_weights: weights_alice.to_owned(),
            dao_weights: weights_dao.to_owned(),
            dao_treasury_eclip_rewards: Uint128::new(0),
            dao_delegators_eclip_rewards: Uint128::new(0),
            // (0.2, 0.3, 0.5) * 1_000 + (0.5, 0.3, 0.2) * 5_000 =
            // (200, 300, 500) + (2_500, 1_500, 1_000) = (2_700, 1_800, 1_500) = (0.45, 0.3, 0.25)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.45", &[]),
                PoolInfoItem::new(ntrn_atom, "0.3", &[]),
                PoolInfoItem::new(astro_atom, "0.25", &[]),
            ],
        }],
    });

    Ok(())
}

#[test]
fn slackers_dao_voting() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let dao = &h.acc(Acc::Dao);
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights_dao = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    // vote
    h.voter_try_push()?;

    let block_time = h.get_block_time();
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 6_000),
        total_votes: vec![],
        vote_results: vec![VoteResults {
            epoch_id: 1,
            end_date: 1717372800,
            elector_essence: Uint128::new(4_800),
            // 0.2 * 6_000 = 1_200
            dao_essence: Uint128::new(1_200),
            slacker_essence: Uint128::new(6_000),
            elector_weights: vec![],
            dao_weights: weights_dao.to_owned(),
            dao_treasury_eclip_rewards: Uint128::new(0),
            dao_delegators_eclip_rewards: Uint128::new(0),
            // (0.5, 0.3, 0.2) * 1_200 = (600, 360, 240) = (0.5, 0.3, 0.2)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.5", &[]),
                PoolInfoItem::new(ntrn_atom, "0.3", &[]),
                PoolInfoItem::new(astro_atom, "0.2", &[]),
            ],
        }],
    });

    Ok(())
}

#[test]
fn electors_dao_voting() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let dao = &h.acc(Acc::Dao);
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights_alice = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.5"),
    ];
    let weights_dao = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
        h.voter_try_place_vote(user, weights_alice)?;
    }

    // place votes
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    // vote
    h.voter_try_push()?;

    let block_time = h.get_block_time();
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 6_000),
        total_votes: vec![],
        vote_results: vec![VoteResults {
            epoch_id: 1,
            end_date: 1717372800,
            elector_essence: Uint128::new(6_000),
            dao_essence: Uint128::new(0),
            slacker_essence: Uint128::new(0),
            elector_weights: weights_alice.to_owned(),
            dao_weights: weights_dao.to_owned(),
            dao_treasury_eclip_rewards: Uint128::new(0),
            dao_delegators_eclip_rewards: Uint128::new(0),
            // (0.2, 0.3, 0.5) * 6_000 = (0.2, 0.3, 0.5)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.2", &[]),
                PoolInfoItem::new(ntrn_atom, "0.3", &[]),
                PoolInfoItem::new(astro_atom, "0.5", &[]),
            ],
        }],
    });

    Ok(())
}

#[test]
fn delegators_dao_voting() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let dao = &h.acc(Acc::Dao);
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights_dao = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
        h.voter_try_delegate(user)?;
    }

    // place votes
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    // vote
    h.voter_try_push()?;

    let block_time = h.get_block_time();
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 0),
        total_votes: vec![],
        vote_results: vec![VoteResults {
            epoch_id: 1,
            end_date: 1717372800,
            elector_essence: Uint128::new(0),
            dao_essence: Uint128::new(6_000),
            slacker_essence: Uint128::new(0),
            elector_weights: vec![],
            dao_weights: weights_dao.to_owned(),
            dao_treasury_eclip_rewards: Uint128::new(0),
            dao_delegators_eclip_rewards: Uint128::new(0),
            // (0.5, 0.3, 0.2) * 6_000 = (0.5, 0.3, 0.2)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.5", &[]),
                PoolInfoItem::new(ntrn_atom, "0.3", &[]),
                PoolInfoItem::new(astro_atom, "0.2", &[]),
            ],
        }],
    });

    Ok(())
}

#[test]
fn electors_voting() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights_alice = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.5"),
    ];

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
        // place votes
        h.voter_try_place_vote(user, weights_alice)?;
    }

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    // vote
    h.voter_try_push()?;

    let block_time = h.get_block_time();
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 6_000),
        total_votes: vec![],
        vote_results: vec![VoteResults {
            epoch_id: 1,
            end_date: 1717372800,
            elector_essence: Uint128::new(6_000),
            dao_essence: Uint128::new(0),
            slacker_essence: Uint128::new(0),
            elector_weights: weights_alice.to_owned(),
            dao_weights: vec![],
            dao_treasury_eclip_rewards: Uint128::new(0),
            dao_delegators_eclip_rewards: Uint128::new(0),
            // (0.2, 0.3, 0.5) * 6_000 = (0.2, 0.3, 0.5)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.2", &[]),
                PoolInfoItem::new(ntrn_atom, "0.3", &[]),
                PoolInfoItem::new(astro_atom, "0.5", &[]),
            ],
        }],
    });

    Ok(())
}

#[test]
fn delegators_voting() -> StdResult<()> {
    let mut h = prepare_helper();

    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
        h.voter_try_delegate(user)?;
    }

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    // vote
    h.voter_try_push()?;

    let block_time = h.get_block_time();
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 0),
        total_votes: vec![],
        vote_results: vec![VoteResults {
            epoch_id: 1,
            end_date: 1717372800,
            elector_essence: Uint128::new(0),
            dao_essence: Uint128::new(6_000),
            slacker_essence: Uint128::new(0),
            elector_weights: vec![],
            dao_weights: vec![],
            dao_treasury_eclip_rewards: Uint128::new(0),
            dao_delegators_eclip_rewards: Uint128::new(0),
            pool_info_list: vec![],
        }],
    });

    Ok(())
}

#[test]
fn slackers_voting() -> StdResult<()> {
    let mut h = prepare_helper();

    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    // vote
    h.voter_try_push()?;

    let block_time = h.get_block_time();
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 6_000),
        total_votes: vec![],
        vote_results: vec![VoteResults {
            epoch_id: 1,
            end_date: 1717372800,
            elector_essence: Uint128::new(4_800),
            dao_essence: Uint128::new(1_200),
            slacker_essence: Uint128::new(6_000),
            elector_weights: vec![],
            dao_weights: vec![],
            dao_treasury_eclip_rewards: Uint128::new(0),
            dao_delegators_eclip_rewards: Uint128::new(0),
            pool_info_list: vec![],
        }],
    });

    Ok(())
}

#[test]
fn change_vote_after_dao_voting() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let dao = &h.acc(Acc::Dao);
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights_alice_before = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];
    let weights_alice_after = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.5"),
    ];
    let weights_dao = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_place_vote(alice, &weights_alice_before)?;
    h.voter_try_delegate(bob)?;
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // revote
    h.voter_try_place_vote(alice, &weights_alice_after)?;

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    // vote
    h.voter_try_push()?;

    let block_time = h.get_block_time();
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 4_000),
        total_votes: vec![],
        vote_results: vec![VoteResults {
            epoch_id: 1,
            end_date: 1717372800,
            // 1_000 + 0.8 * 3_000
            elector_essence: Uint128::new(3_400),
            // 2_000 + 0.2 * 3_000
            dao_essence: Uint128::new(2_600),
            slacker_essence: Uint128::new(3_000),
            elector_weights: weights_alice_after.to_owned(),
            dao_weights: weights_dao.to_owned(),
            dao_treasury_eclip_rewards: Uint128::new(0),
            dao_delegators_eclip_rewards: Uint128::new(0),
            // (0.2, 0.3, 0.5) * (1_000 + 0.8 * 3_000) + (0.5, 0.3, 0.2) * (2_000 + 0.2 * 3_000) =
            // (680, 1_020, 1_700) + (1_300, 780, 520) = (1_980, 1_800, 2_220) = (0.33, 0.3, 0.37)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.33", &[]),
                PoolInfoItem::new(ntrn_atom, "0.3", &[]),
                PoolInfoItem::new(astro_atom, "0.37", &[]),
            ],
        }],
    });

    Ok(())
}

#[test]
fn user_roles_default() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let dao = &h.acc(Acc::Dao);
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights_alice = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.5"),
    ];
    let weights_dao = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // check roles
    let res = h.voter_query_user(alice, None).unwrap_err();
    assert_error(&res, ContractError::UserIsNotFound);

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_place_vote(alice, weights_alice)?;
    h.voter_try_delegate(bob)?;
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // check roles
    let alice_info = h.voter_query_user(alice, None)?;
    let bob_info = h.voter_query_user(bob, None)?;
    let john_info = h.voter_query_user(john, None)?;

    assert_that(&alice_info.user_type).is_equal_to(UserType::Elector);
    assert_that(&bob_info.user_type).is_equal_to(UserType::Delegator);
    assert_that(&john_info.user_type).is_equal_to(UserType::Slacker);

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    // vote
    h.voter_try_push()?;

    // check roles
    let alice_info = h.voter_query_user(alice, None)?;
    let bob_info = h.voter_query_user(bob, None)?;
    let john_info = h.voter_query_user(john, None)?;

    assert_that(&alice_info.user_type).is_equal_to(UserType::Slacker);
    assert_that(&bob_info.user_type).is_equal_to(UserType::Delegator);
    assert_that(&john_info.user_type).is_equal_to(UserType::Slacker);

    Ok(())
}

#[test]
fn electors_and_slackers_can_delegate() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let dao = &h.acc(Acc::Dao);
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights_alice = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.5"),
    ];
    let weights_dao = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_place_vote(alice, weights_alice)?;
    h.voter_try_delegate(bob)?;

    // check roles
    let essence_info_alice = h.voter_query_user(alice, None)?;
    let essence_info_john = h.voter_query_user(john, None)?;

    assert_that(&essence_info_alice).is_equal_to(UserResponse {
        user_type: UserType::Elector,
        essence_info: EssenceInfo::new::<u128>(0, 0, 1_000),
        essence_value: Uint128::new(1_000),
        weights: weights_alice.to_owned(),
        rewards: RewardsInfo::default(),
    });
    assert_that(&essence_info_john).is_equal_to(UserResponse {
        user_type: UserType::Slacker,
        essence_info: EssenceInfo::new::<u128>(0, 0, 3_000),
        essence_value: Uint128::new(3_000),
        weights: vec![],
        rewards: RewardsInfo::default(),
    });

    // delegate
    h.voter_try_delegate(alice)?;
    h.voter_try_delegate(john)?;

    // check roles
    let essence_info_alice = h.voter_query_user(alice, None)?;
    let essence_info_john = h.voter_query_user(john, None)?;

    assert_that(&essence_info_alice).is_equal_to(UserResponse {
        user_type: UserType::Delegator,
        essence_info: EssenceInfo::new::<u128>(0, 0, 1_000),
        essence_value: Uint128::new(1_000),
        weights: vec![],
        rewards: RewardsInfo::default(),
    });
    assert_that(&essence_info_john).is_equal_to(UserResponse {
        user_type: UserType::Delegator,
        essence_info: EssenceInfo::new::<u128>(0, 0, 3_000),
        essence_value: Uint128::new(3_000),
        weights: vec![],
        rewards: RewardsInfo::default(),
    });

    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    // vote
    h.voter_try_push()?;

    let block_time = h.get_block_time();
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 0),
        total_votes: vec![],
        vote_results: vec![VoteResults {
            epoch_id: 1,
            end_date: 1717372800,
            elector_essence: Uint128::new(0),
            dao_essence: Uint128::new(6_000),
            slacker_essence: Uint128::new(0),
            elector_weights: vec![],
            dao_weights: weights_dao.to_owned(),
            dao_treasury_eclip_rewards: Uint128::new(0),
            dao_delegators_eclip_rewards: Uint128::new(0),
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.5", &[]),
                PoolInfoItem::new(ntrn_atom, "0.3", &[]),
                PoolInfoItem::new(astro_atom, "0.2", &[]),
            ],
        }],
    });

    Ok(())
}

#[test]
fn delegators_and_dao_can_not_delegate() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let dao = &h.acc(Acc::Dao);
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_place_vote(alice, weights)?;
    h.voter_try_delegate(bob)?;
    h.voter_try_place_vote_as_dao(dao, weights)?;

    // delegate
    let res = h.voter_try_delegate(bob).unwrap_err();
    assert_error(&res, ContractError::DelegateTwice);
    let res = h.voter_try_delegate(dao).unwrap_err();
    assert_error(&res, ContractError::UserIsNotFound);

    Ok(())
}

#[test]
fn delegators_can_not_vote() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let dao = &h.acc(Acc::Dao);
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_place_vote(alice, weights)?;
    h.voter_try_delegate(bob)?;
    h.voter_try_place_vote_as_dao(dao, weights)?;

    // place vote as delegator
    let res = h.voter_try_place_vote(bob, weights).unwrap_err();
    assert_error(&res, ContractError::DelegatorCanNotVote);

    Ok(())
}

#[test]
fn undelegate_default() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let dao = &h.acc(Acc::Dao);
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights_alice = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.5"),
    ];
    let weights_dao = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_place_vote(alice, weights_alice)?;
    h.voter_try_delegate(bob)?;
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // undelegate
    h.voter_try_undelegate(bob)?;
    for user in [alice, john] {
        let res = h.voter_try_undelegate(user).unwrap_err();
        assert_error(&res, ContractError::DelegatorIsNotFound);
    }
    let res = h.voter_try_undelegate(dao).unwrap_err();
    assert_error(&res, ContractError::UserIsNotFound);

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    // vote
    h.voter_try_push()?;

    let block_time = h.get_block_time();
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 6_000),
        total_votes: vec![],
        vote_results: vec![VoteResults {
            epoch_id: 1,
            end_date: 1717372800,
            // 1_000 + 0.8 * 5_000
            elector_essence: Uint128::new(5_000),
            // 0.2 * 5_000
            dao_essence: Uint128::new(1_000),
            slacker_essence: Uint128::new(5_000),
            elector_weights: weights_alice.to_owned(),
            dao_weights: weights_dao.to_owned(),
            dao_treasury_eclip_rewards: Uint128::new(0),
            dao_delegators_eclip_rewards: Uint128::new(0),
            // (0.2, 0.3, 0.5) * (1_000 + 0.8 * 5_000) + (0.5, 0.3, 0.2) * 0.2 * 5_000 =
            // (1_000, 1_500, 2_500) + (500, 300, 200) = (1_500, 1_800, 2_700) = (0.25, 0.3, 0.45)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.25", &[]),
                PoolInfoItem::new(ntrn_atom, "0.3", &[]),
                PoolInfoItem::new(astro_atom, "0.45", &[]),
            ],
        }],
    });

    Ok(())
}

#[test]
fn reset_electors_and_dao_on_epoch_start() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let owner = &h.acc(Acc::Owner);
    let dao = &h.acc(Acc::Dao);
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights_alice = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.5"),
    ];
    let weights_dao = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_place_vote(alice, weights_alice)?;
    h.voter_try_delegate(bob)?;
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // check votes
    let alice_info = h.voter_query_user(alice, None)?;
    let dao_info = h.voter_query_dao_info(None)?;
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&alice_info).is_equal_to(UserResponse {
        user_type: UserType::Elector,
        essence_info: EssenceInfo::new::<u128>(0, 0, 1_000),
        essence_value: Uint128::new(1_000),
        weights: weights_alice.to_owned(),
        rewards: RewardsInfo::default(),
    });
    assert_that(&dao_info).is_equal_to(DaoResponse {
        essence_info: EssenceInfo::new::<u128>(0, 0, 2_000),
        essence_value: Uint128::new(2_000),
        weights: weights_dao.to_owned(),
    });
    assert_that(&voter_info.slacker_essence_acc).is_equal_to(EssenceInfo::new::<u128>(0, 0, 3_000));

    // final voting
    let date_config = h.voter_query_date_config()?;
    let epoch_length = date_config.epoch_length;
    let vote_delay = date_config.vote_delay;
    h.wait(h.voter_query_date_config()?.vote_delay);
    h.voter_try_push()?;

    // allocate rewards
    h.wait(epoch_length - vote_delay + 1);
    h.tribute_market_try_allocate_rewards(owner, &[&h.voter_contract_address()])?;
    // claim rewards
    h.voter_try_push()?;
    // swap rewards
    h.voter_try_push()?;

    // check votes
    let alice_info = h.voter_query_user(alice, None)?;
    let dao_info = h.voter_query_dao_info(None)?;
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&alice_info.user_type).is_equal_to(UserType::Slacker);
    assert_that(&dao_info.weights).is_equal_to(vec![]);
    assert_that(&voter_info.slacker_essence_acc).is_equal_to(EssenceInfo::new::<u128>(0, 0, 4_000));

    Ok(())
}

#[test]
fn rotating_claim_stage() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let owner = &h.acc(Acc::Owner);
    let dao = &h.acc(Acc::Dao);
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights_alice = &vec![
        WeightAllocationItem::new(eclip_atom, "0.2"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.5"),
    ];
    let weights_dao = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_place_vote(alice, weights_alice)?;
    h.voter_try_delegate(bob)?;
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // try vote too early
    let res = h.voter_try_push().unwrap_err();
    assert_error(&res, ContractError::VotingDelay);

    // final voting
    let date_config = h.voter_query_date_config()?;
    let epoch_length = date_config.epoch_length;
    let vote_delay = date_config.vote_delay;
    h.wait(h.voter_query_date_config()?.vote_delay);
    // vote
    h.voter_try_push()?;

    // try update essence, place vote, undelegate, delegate, claim user rewards
    h.eclipsepad_staking_try_stake(alice, 1_000, Denom::Eclip)
        .unwrap_err();
    let res = h.voter_try_place_vote(alice, weights_dao).unwrap_err();
    assert_error(&res, ContractError::AwaitSwappedStage);
    let res = h.voter_try_place_vote_as_dao(dao, weights_dao).unwrap_err();
    assert_error(&res, ContractError::AwaitSwappedStage);
    let res = h.voter_try_undelegate(bob).unwrap_err();
    assert_error(&res, ContractError::AwaitSwappedStage);
    let res = h.voter_try_delegate(john).unwrap_err();
    assert_error(&res, ContractError::AwaitSwappedStage);
    let res = h.voter_try_claim_rewards(alice).unwrap_err();
    assert_error(&res, ContractError::RewardsAreNotFound);
    // try claim rewards until they added
    let res = h.voter_try_push().unwrap_err();
    assert_error(&res, ContractError::RewardsAreNotFound);

    h.wait(epoch_length - vote_delay + 1);

    // try claim rewards until they added
    let res = h.voter_try_push().unwrap_err();
    assert_error(&res, ContractError::RewardsAreNotFound);

    // allocate rewards
    h.tribute_market_try_allocate_rewards(owner, &[&h.voter_contract_address()])?;

    // try update essence, place vote, undelegate, delegate, claim user rewards
    h.eclipsepad_staking_try_stake(alice, 1_000, Denom::Eclip)
        .unwrap_err();
    let res = h.voter_try_place_vote(alice, weights_dao).unwrap_err();
    assert_error(&res, ContractError::AwaitSwappedStage);
    let res = h.voter_try_place_vote_as_dao(dao, weights_dao).unwrap_err();
    assert_error(&res, ContractError::AwaitSwappedStage);
    let res = h.voter_try_undelegate(bob).unwrap_err();
    assert_error(&res, ContractError::AwaitSwappedStage);
    let res = h.voter_try_delegate(john).unwrap_err();
    assert_error(&res, ContractError::AwaitSwappedStage);
    let res = h.voter_try_claim_rewards(alice).unwrap_err();
    assert_error(&res, ContractError::RewardsAreNotFound);

    // claim rewards
    h.voter_try_push()?;

    // try update essence, place vote, undelegate, delegate, claim user rewards
    h.eclipsepad_staking_try_stake(alice, 1_000, Denom::Eclip)
        .unwrap_err();
    let res = h.voter_try_place_vote(alice, weights_dao).unwrap_err();
    assert_error(&res, ContractError::AwaitSwappedStage);
    let res = h.voter_try_place_vote_as_dao(dao, weights_dao).unwrap_err();
    assert_error(&res, ContractError::AwaitSwappedStage);
    let res = h.voter_try_undelegate(bob).unwrap_err();
    assert_error(&res, ContractError::AwaitSwappedStage);
    let res = h.voter_try_delegate(john).unwrap_err();
    assert_error(&res, ContractError::AwaitSwappedStage);
    let res = h.voter_try_claim_rewards(alice).unwrap_err();
    assert_error(&res, ContractError::RewardsAreNotFound);

    // swap rewards
    h.voter_try_push()?;

    // try vote too early
    let res = h.voter_try_push().unwrap_err();
    assert_error(&res, ContractError::VotingDelay);

    // claim user rewards
    h.voter_try_claim_rewards(alice)?;

    // try claim user rewards twice
    let res = h.voter_try_claim_rewards(alice).unwrap_err();
    assert_error(&res, ContractError::RewardsAreNotFound);

    Ok(())
}

#[test]
fn clearing_storages() -> StdResult<()> {
    let mut h = prepare_helper();

    let eclip_atom = &h.pool(Pool::EclipAtom);
    let ntrn_atom = &h.pool(Pool::NtrnAtom);
    let astro_atom = &h.pool(Pool::AstroAtom);

    let dao = &h.acc(Acc::Dao);
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);
    let john = &h.acc(Acc::John);

    let weights = &vec![
        WeightAllocationItem::new(eclip_atom, "0.5"),
        WeightAllocationItem::new(ntrn_atom, "0.3"),
        WeightAllocationItem::new(astro_atom, "0.2"),
    ];

    // stake and lock
    for (user, amount) in [(alice, 1_000), (bob, 2_000), (john, 3_000)] {
        h.eclipsepad_staking_try_stake(user, amount, Denom::Eclip)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_place_vote(alice, weights)?;
    h.voter_try_delegate(bob)?;
    h.voter_try_place_vote_as_dao(dao, weights)?;

    // unlock
    for user in [alice, bob, john] {
        h.eclipsepad_staking_try_unlock(user)?;
    }

    // check votes
    for user in [alice, bob, john] {
        let res = h.voter_query_user(user, None).unwrap_err();
        assert_error(&res, ContractError::UserIsNotFound);
    }

    let dao_info = h.voter_query_dao_info(None)?;
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&dao_info).is_equal_to(DaoResponse {
        essence_info: EssenceInfo::new::<u128>(0, 0, 0),
        essence_value: Uint128::new(0),
        weights: weights.to_owned(),
    });
    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time: h.get_block_time(),
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 0),
        total_votes: vec![],
        vote_results: vec![],
    });

    Ok(())
}

// TODO
// +EssenceInfo math, captured essence
// +calc_essence_allocation
// +calc_updated_essence_allocation
// +calc_scaled_essence_allocation
// +auto-updating essence in voter
// +essence update will change weights
// +2 slackers + 2 electors + 2 delegators + dao (default voting)
// +slackers + electors + dao
// +slackers + delegators + dao
// +electors + delegators + dao
// +slackers + dao
// +electors + dao
// +delegators + dao
// +electors
// +delegators
// +slackers
// +changing vote after dao
// +elector, slacker can delegate
// +delegator, dao can't delegate
// +delegator can't vote
// +delegator can undelegate; elector, slacker, dao can't undelegate
// +reset electors and dao on epoch start
// +rotating claim stage, users can act only during swapped stage
// +clearing storages
// wrong weights
// whitelisted pools
// changing wl pools in each epoch
// proper weights merging
// historical data, vote early, vote twice
// user voted in e1, delegated in e2, undelegated in e3 - rewards, weights, essence
// user delegated in e1, undelegated and voted in e2 - rewards, weights, essence
// delegate-undelegate loop - rewards, weights, essence
// vote-delegate-undelegate loop - rewards, weights, essence
// changing settings before next epoch
// rewards w/o tribute market
// query rewards, claim, claim again, query again
