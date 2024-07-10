use astroport_governance::emissions_controller::hub::{
    AstroPoolConfig, OutpostInfo, OutpostParams, VotedPoolInfo,
};
use cosmwasm_std::{coins, Addr, StdResult, Uint128};
use cw_multi_test::Executor;

use eclipse_base::{
    assets::{Currency, Token},
    converters::str_to_dec,
};
use equinox_msg::voter::{
    DaoResponse, EssenceAllocationItem, EssenceInfo, PoolInfoItem, UserResponse, VoteResults,
    VoterInfoResponse, WeightAllocationItem,
};
use speculoos::assert_that;
use strum::IntoEnumIterator;
use voter::{
    math::{
        calc_essence_allocation, calc_scaled_essence_allocation, calc_updated_essence_allocation,
        calc_weights_from_essence_allocation,
    },
    state::{EPOCH_LENGTH, GENESIS_EPOCH_START_DATE, VOTE_DELAY},
};

use crate::suite_astro::{
    extensions::{
        eclipsepad_staking::EclipsepadStakingExtension, minter::MinterExtension,
        voter::VoterExtension,
    },
    helper::{Acc, ControllerHelper, Pool},
};

const INITIAL_LIQUIDITY: u128 = 1_000_000;
const ECLIP: &str = "eclip";
const ECLIP_ASTRO: &str = "eclipastro";

fn prepare_helper() -> ControllerHelper {
    let mut h = ControllerHelper::new();
    let astro = &h.astro.clone();
    let owner = &h.acc(Acc::Owner);

    h.minter_prepare_contract();
    h.eclipsepad_staking_prepare_contract(
        None,
        None,
        Some(ECLIP),
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
        None,
        &h.astro.clone(),
        &h.xastro.clone(),
        ECLIP_ASTRO,
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

    for token in [ECLIP, &h.astro.clone()] {
        h.mint_tokens(owner, &coins(1_000 * INITIAL_LIQUIDITY, token))
            .unwrap();
    }

    for user in Acc::iter() {
        for token in [ECLIP, &h.astro.clone(), &h.xastro.clone()] {
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
        &coins(1_000 * INITIAL_LIQUIDITY, ECLIP_ASTRO),
    )
    .unwrap();

    h.minter_try_register_currency(
        owner,
        &Currency::new(&Token::new_native(ECLIP_ASTRO), 6),
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
        // create pair
        let (denom1, denom2) = pool.get_pair();
        let pair = &Addr::unchecked(h.create_pair(denom1, denom2));
        // add pair in pool_list
        h.pool_list.push((pool, pair.to_owned()));
        // add in wl
        h.whitelist(owner, pair, &coins(1_000_000, astro)).unwrap();
    }

    h.voter_try_swap_to_eclip_astro(owner, 100_000_000, astro)
        .unwrap();

    h
}

#[test]
fn swap_to_eclip_astro_default() -> StdResult<()> {
    let mut h = prepare_helper();
    let ControllerHelper { astro, xastro, .. } = &ControllerHelper::new();
    let alice = &h.acc(Acc::Alice);
    let bob = &h.acc(Acc::Bob);

    let alice_astro = h.query_balance(alice, astro);
    let alice_xastro = h.query_balance(alice, xastro);
    let alice_eclip_astro = h.query_balance(alice, ECLIP_ASTRO);
    assert_that(&alice_astro).is_equal_to(100_000);
    assert_that(&alice_xastro).is_equal_to(100_000);
    assert_that(&alice_eclip_astro).is_equal_to(0);

    let bob_astro = h.query_balance(bob, astro);
    let bob_xastro = h.query_balance(bob, xastro);
    let bob_eclip_astro = h.query_balance(bob, ECLIP_ASTRO);
    assert_that(&bob_astro).is_equal_to(100_000);
    assert_that(&bob_xastro).is_equal_to(100_000);
    assert_that(&bob_eclip_astro).is_equal_to(0);

    h.voter_try_swap_to_eclip_astro(alice, 1_000, astro)?;
    h.voter_try_swap_to_eclip_astro(bob, 1_000, xastro)?;

    let alice_astro = h.query_balance(alice, astro);
    let alice_xastro = h.query_balance(alice, xastro);
    let alice_eclip_astro = h.query_balance(alice, ECLIP_ASTRO);
    assert_that(&alice_astro).is_equal_to(99_000);
    assert_that(&alice_xastro).is_equal_to(100_000);
    assert_that(&alice_eclip_astro).is_equal_to(1_000);

    let bob_astro = h.query_balance(bob, astro);
    let bob_xastro = h.query_balance(bob, xastro);
    let bob_eclip_astro = h.query_balance(bob, ECLIP_ASTRO);
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
    assert_that(&essence_info_1.sub(&essence_info_2)).is_equal_to(essence_info_2);
    assert_that(&essence_info_1.sub(&essence_info_1).is_zero()).is_equal_to(true);
    // (20 * 50_000_000 - 500_000_000) / 31_536_000 + 40 = 55
    assert_that(&essence_info_1.capture(50_000_000).u128()).is_equal_to(55);

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
        h.eclipsepad_staking_try_stake(user, 1_000, ECLIP)?;
    }

    let essence_info_alice = h.voter_query_user(alice, None)?;
    let essence_info_bob = h.voter_query_user(bob, None)?;
    let essence_info_john = h.voter_query_user(john, None)?;

    assert_that(&essence_info_alice).is_equal_to(UserResponse::Slacker {
        essence_info: EssenceInfo::new::<u128>(1000, 1716163200000, 0),
        essence_value: Uint128::zero(),
    });
    assert_that(&essence_info_bob).is_equal_to(UserResponse::Slacker {
        essence_info: EssenceInfo::new::<u128>(1000, 1716163200000, 0),
        essence_value: Uint128::zero(),
    });
    assert_that(&essence_info_john).is_equal_to(UserResponse::Slacker {
        essence_info: EssenceInfo::new::<u128>(1000, 1716163200000, 0),
        essence_value: Uint128::zero(),
    });

    // take roles: alice - elector, bob - delegator, john - slacker
    h.voter_try_place_vote(alice, weights)?;
    h.voter_try_delegate(bob)?;

    let essence_info_alice = h.voter_query_user(alice, None)?;
    let essence_info_bob = h.voter_query_user(bob, None)?;
    let essence_info_john = h.voter_query_user(john, None)?;

    assert_that(&essence_info_alice).is_equal_to(UserResponse::Elector {
        essence_info: EssenceInfo::new::<u128>(1000, 1716163200000, 0),
        essence_value: Uint128::zero(),
        weights: weights.to_vec(),
    });
    assert_that(&essence_info_bob).is_equal_to(UserResponse::Delegator {
        essence_info: EssenceInfo::new::<u128>(1000, 1716163200000, 0),
        essence_value: Uint128::zero(),
    });
    assert_that(&essence_info_john).is_equal_to(UserResponse::Slacker {
        essence_info: EssenceInfo::new::<u128>(1000, 1716163200000, 0),
        essence_value: Uint128::zero(),
    });

    // change essence for slacker, elector, delegator
    // lock
    for user in [alice, bob, john] {
        h.eclipsepad_staking_try_lock(user, 1_000, 4)?;
    }

    let essence_info_alice = h.voter_query_user(alice, None)?;
    let essence_info_bob = h.voter_query_user(bob, None)?;
    let essence_info_john = h.voter_query_user(john, None)?;

    assert_that(&essence_info_alice).is_equal_to(UserResponse::Elector {
        essence_info: EssenceInfo::new::<u128>(0, 0, 1000),
        essence_value: Uint128::new(1000),
        weights: weights.to_vec(),
    });
    assert_that(&essence_info_bob).is_equal_to(UserResponse::Delegator {
        essence_info: EssenceInfo::new::<u128>(0, 0, 1000),
        essence_value: Uint128::new(1000),
    });
    assert_that(&essence_info_john).is_equal_to(UserResponse::Slacker {
        essence_info: EssenceInfo::new::<u128>(0, 0, 1000),
        essence_value: Uint128::new(1000),
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
        h.eclipsepad_staking_try_stake(user, 1_000, ECLIP)?;
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

    assert_that(&essence_info_alice).is_equal_to(UserResponse::Elector {
        essence_info: EssenceInfo::new::<u128>(0, 0, 1_000),
        essence_value: Uint128::new(1_000),
        weights: weights_alice.to_vec(),
    });
    assert_that(&essence_info_dao).is_equal_to(DaoResponse {
        essence_info: EssenceInfo::new::<u128>(0, 0, 1_000),
        essence_value: Uint128::new(1_000),
        weights: weights_dao.to_vec(),
    });
    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time: 1716163200,
        elector_votes: vec![
            EssenceAllocationItem::new(eclip_atom, &EssenceInfo::new::<u128>(0, 0, 200)),
            EssenceAllocationItem::new(ntrn_atom, &EssenceInfo::new::<u128>(0, 0, 300)),
            EssenceAllocationItem::new(astro_atom, &EssenceInfo::new::<u128>(0, 0, 500)),
        ],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 1_000),
        total_votes: vec![
            EssenceAllocationItem::new(eclip_atom, &EssenceInfo::new::<u128>(0, 0, 700)),
            EssenceAllocationItem::new(ntrn_atom, &EssenceInfo::new::<u128>(0, 0, 600)),
            EssenceAllocationItem::new(astro_atom, &EssenceInfo::new::<u128>(0, 0, 700)),
        ],
        vote_results: vec![],
    });

    assert_that(&calc_weights_from_essence_allocation(&voter_info.elector_votes, block_time).1)
        .is_equal_to(weights_alice);
    assert_that(&calc_weights_from_essence_allocation(&voter_info.total_votes, block_time).1)
        .is_equal_to(vec![
            WeightAllocationItem::new(eclip_atom, "0.35"),
            WeightAllocationItem::new(ntrn_atom, "0.3"),
            WeightAllocationItem::new(astro_atom, "0.35"),
        ]);

    // change alice essence
    h.wait(1);
    h.eclipsepad_staking_try_stake(alice, 1_000, ECLIP)?;
    h.eclipsepad_staking_try_lock(alice, 1_000, 4)?;

    let essence_info_alice = h.voter_query_user(alice, None)?;
    let essence_info_dao = h.voter_query_dao_info(None)?;
    let voter_info = h.voter_query_voter_info(None)?;
    let block_time = h.get_block_time();

    assert_that(&essence_info_alice).is_equal_to(UserResponse::Elector {
        essence_info: EssenceInfo::new::<u128>(0, 0, 2_000),
        essence_value: Uint128::new(2_000),
        weights: weights_alice.to_vec(),
    });
    assert_that(&essence_info_dao).is_equal_to(DaoResponse {
        essence_info: EssenceInfo::new::<u128>(0, 0, 1_000),
        essence_value: Uint128::new(1_000),
        weights: weights_dao.to_vec(),
    });
    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![
            EssenceAllocationItem::new(eclip_atom, &EssenceInfo::new::<u128>(0, 0, 400)),
            EssenceAllocationItem::new(ntrn_atom, &EssenceInfo::new::<u128>(0, 0, 600)),
            EssenceAllocationItem::new(astro_atom, &EssenceInfo::new::<u128>(0, 0, 1_000)),
        ],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 1_000),
        // (400, 600, 1_000) + (500, 300, 200) = (900, 900, 1_200)
        total_votes: vec![
            EssenceAllocationItem::new(eclip_atom, &EssenceInfo::new::<u128>(0, 0, 900)),
            EssenceAllocationItem::new(ntrn_atom, &EssenceInfo::new::<u128>(0, 0, 900)),
            EssenceAllocationItem::new(astro_atom, &EssenceInfo::new::<u128>(0, 0, 1_200)),
        ],
        vote_results: vec![],
    });

    assert_that(&calc_weights_from_essence_allocation(&voter_info.elector_votes, block_time).1)
        .is_equal_to(weights_alice);
    // (900, 900, 1_200) / 3_000 = (0.3, 0.3, 0.4)
    assert_that(&calc_weights_from_essence_allocation(&voter_info.total_votes, block_time).1)
        .is_equal_to(vec![
            WeightAllocationItem::new(eclip_atom, "0.3"),
            WeightAllocationItem::new(ntrn_atom, "0.3"),
            WeightAllocationItem::new(astro_atom, "0.4"),
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
        h.eclipsepad_staking_try_stake(user, amount, ECLIP)?;
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
    h.voter_try_vote()?;

    let block_time = h.get_block_time();
    let voter_info = h.voter_query_voter_info(None)?;
    let voted_pools = h.query_voted_pools(None)?;

    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 11_000),
        total_votes: vec![],
        vote_results: vec![VoteResults {
            epoch_id: 1,
            end_date: 1717372800,
            // 3_000 + 0.8 * 11_000 + 7_000 + 0.2 * 11_000 = 21_000 (20_999)
            essence: Uint128::new(20_999),
            // ((0.2, 0.3, 0.5) * 1_000 + (0.1, 0.7, 0.2) * 2_000) * ((3_000 + 0.8 * 11_000) / 3_000) +
            // (0.5, 0.3, 0.2) * (7_000 + 0.2 * 11_000) =
            // ((200, 300, 500) + (200, 1_400, 400)) * (11_800 / 3_000) + (0.5, 0.3, 0.2) * 9_200 =
            // (400, 1_700, 900) * 3.93 + (4_600 + 2_760 + 1_840) =
            // (6_173, 9_446, 5_379) = (0.294, 0.45, 0.256)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.293966379351397685", 0),
                PoolInfoItem::new(ntrn_atom, "0.449830944330682413", 0),
                PoolInfoItem::new(astro_atom, "0.2562026763179199", 0),
            ],
        }],
    });

    assert_that(&voted_pools).is_equal_to(vec![
        (
            astro_atom.to_string(),
            VotedPoolInfo {
                init_ts: 1716163200,
                voting_power: Uint128::new(25620267),
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
            eclip_atom.to_string(),
            VotedPoolInfo {
                init_ts: 1716163200,
                voting_power: Uint128::new(29396637),
            },
        ),
    ]);

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
        h.eclipsepad_staking_try_stake(user, amount, ECLIP)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_place_vote(alice, weights_alice)?;
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    h.voter_try_vote()?;

    let block_time = h.get_block_time();
    let voter_info = h.voter_query_voter_info(None)?;

    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![],
        slacker_essence_acc: EssenceInfo::new::<u128>(0, 0, 5_000),
        total_votes: vec![],
        vote_results: vec![VoteResults {
            epoch_id: 1,
            end_date: 1717372800,
            // 1_000 + 0.8 * 5_000 + 0.2 * 5_000 = 6_000
            essence: Uint128::new(6_000),
            // (0.2, 0.3, 0.5) * (1_000 + 0.8 * 5_000) + (0.5, 0.3, 0.2) * (0.2 * 5_000) =
            // (1_000, 1_500, 2_500) + (500, 300, 200) = (1_500, 1_800, 2_700) = (0.25, 0.3, 0.45)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.25", 0),
                PoolInfoItem::new(ntrn_atom, "0.3", 0),
                PoolInfoItem::new(astro_atom, "0.45", 0),
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
        h.eclipsepad_staking_try_stake(user, amount, ECLIP)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_delegate(bob)?;
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    h.voter_try_vote()?;

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
            // 2_000 + 0.2 * 4_000 = 2_800
            essence: Uint128::new(2_800),
            // (0.5, 0.3, 0.2) * (2_000 + 0.2 * 4_000) =
            // (1_400, 740, 560) = (0.5, 0.3, 0.2)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.5", 0),
                PoolInfoItem::new(ntrn_atom, "0.3", 0),
                PoolInfoItem::new(astro_atom, "0.2", 0),
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
        h.eclipsepad_staking_try_stake(user, amount, ECLIP)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_place_vote(alice, weights_alice)?;
    h.voter_try_delegate(bob)?;
    h.voter_try_delegate(john)?;
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    h.voter_try_vote()?;

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
            // 1_000 + 5_000 = 6_000
            essence: Uint128::new(6_000),
            // (0.2, 0.3, 0.5) * 1_000 + (0.5, 0.3, 0.2) * 5_000 =
            // (200, 300, 500) + (2_500, 1_500, 1_000) = (2_700, 1_800, 1_500) = (0.45, 0.3, 0.25)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.45", 0),
                PoolInfoItem::new(ntrn_atom, "0.3", 0),
                PoolInfoItem::new(astro_atom, "0.25", 0),
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
        h.eclipsepad_staking_try_stake(user, amount, ECLIP)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // place votes
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    h.voter_try_vote()?;

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
            // 0.2 * 6_000 = 1_200
            essence: Uint128::new(1_200),
            // (0.5, 0.3, 0.2) * 1_200 = (600, 360, 240) = (0.5, 0.3, 0.2)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.5", 0),
                PoolInfoItem::new(ntrn_atom, "0.3", 0),
                PoolInfoItem::new(astro_atom, "0.2", 0),
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
        h.eclipsepad_staking_try_stake(user, amount, ECLIP)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
        h.voter_try_place_vote(user, weights_alice)?;
    }

    // place votes
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    h.voter_try_vote()?;

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
            // 6_000
            essence: Uint128::new(6_000),
            // (0.2, 0.3, 0.5) * 6_000 = (0.2, 0.3, 0.5)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.2", 0),
                PoolInfoItem::new(ntrn_atom, "0.3", 0),
                PoolInfoItem::new(astro_atom, "0.5", 0),
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
        h.eclipsepad_staking_try_stake(user, amount, ECLIP)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
        h.voter_try_delegate(user)?;
    }

    // place votes
    h.voter_try_place_vote_as_dao(dao, weights_dao)?;

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    h.voter_try_vote()?;

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
            // 6_000
            essence: Uint128::new(6_000),
            // (0.5, 0.3, 0.2) * 6_000 = (0.5, 0.3, 0.2)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.5", 0),
                PoolInfoItem::new(ntrn_atom, "0.3", 0),
                PoolInfoItem::new(astro_atom, "0.2", 0),
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
        h.eclipsepad_staking_try_stake(user, amount, ECLIP)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
        // place votes
        h.voter_try_place_vote(user, weights_alice)?;
    }

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    h.voter_try_vote()?;

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
            // 6_000
            essence: Uint128::new(6_000),
            // (0.2, 0.3, 0.5) * 6_000 = (0.2, 0.3, 0.5)
            pool_info_list: vec![
                PoolInfoItem::new(eclip_atom, "0.2", 0),
                PoolInfoItem::new(ntrn_atom, "0.3", 0),
                PoolInfoItem::new(astro_atom, "0.5", 0),
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
        h.eclipsepad_staking_try_stake(user, amount, ECLIP)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
        h.voter_try_delegate(user)?;
    }

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    h.voter_try_vote()?;

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
            essence: Uint128::new(0),
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
        h.eclipsepad_staking_try_stake(user, amount, ECLIP)?;
        h.eclipsepad_staking_try_lock(user, amount, 4)?;
    }

    // final voting
    h.wait(h.voter_query_date_config()?.vote_delay);
    h.voter_try_vote()?;

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
            essence: Uint128::new(0),
            pool_info_list: vec![],
        }],
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
// changing vote after dao
// can't place vote after final voting
// elector, slacker can delegate
// delegator, dao can't delegate
// delegator can't vote
// delegator can undelegate
// elector, slacker, dao can't undelegate
// elector new epoch reset
// dao new epoch reset
// clearing storages
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
