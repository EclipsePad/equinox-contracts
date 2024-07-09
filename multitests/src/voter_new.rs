use astroport_governance::emissions_controller::hub::{
    AstroPoolConfig, OutpostInfo, OutpostParams,
};
use cosmwasm_std::{coins, Addr, StdResult, Uint128};
use cw_multi_test::Executor;

use eclipse_base::{
    assets::{Currency, Token},
    converters::str_to_dec,
};
use equinox_msg::voter::{
    DaoResponse, EssenceAllocationItem, EssenceInfo, UserResponse, VoterInfoResponse,
    WeightAllocationItem,
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
    assert_eq!(alice_astro, 100_000);
    assert_eq!(alice_xastro, 100_000);
    assert_eq!(alice_eclip_astro, 0);

    let bob_astro = h.query_balance(bob, astro);
    let bob_xastro = h.query_balance(bob, xastro);
    let bob_eclip_astro = h.query_balance(bob, ECLIP_ASTRO);
    assert_eq!(bob_astro, 100_000);
    assert_eq!(bob_xastro, 100_000);
    assert_eq!(bob_eclip_astro, 0);

    h.voter_try_swap_to_eclip_astro(alice, 1_000, astro)?;
    h.voter_try_swap_to_eclip_astro(bob, 1_000, xastro)?;

    let alice_astro = h.query_balance(alice, astro);
    let alice_xastro = h.query_balance(alice, xastro);
    let alice_eclip_astro = h.query_balance(alice, ECLIP_ASTRO);
    assert_eq!(alice_astro, 99_000);
    assert_eq!(alice_xastro, 100_000);
    assert_eq!(alice_eclip_astro, 1_000);

    let bob_astro = h.query_balance(bob, astro);
    let bob_xastro = h.query_balance(bob, xastro);
    let bob_eclip_astro = h.query_balance(bob, ECLIP_ASTRO);
    assert_eq!(bob_astro, 100_000);
    assert_eq!(bob_xastro, 99_000);
    assert_eq!(bob_eclip_astro, 1_000);

    Ok(())
}

#[test]
fn essence_info_math() -> StdResult<()> {
    let essence_info_1 = EssenceInfo::new(20, 500_000_000, 40);
    let essence_info_2 = EssenceInfo::new(10, 250_000_000, 20);

    assert_eq!(essence_info_2.add(&essence_info_2), essence_info_1);
    assert_eq!(essence_info_1.sub(&essence_info_2), essence_info_2);
    assert_eq!(essence_info_1.sub(&essence_info_1).is_zero(), true);
    // (20 * 50_000_000 - 500_000_000) / 31_536_000 + 40 = 55
    assert_eq!(essence_info_1.capture(50_000_000).u128(), 55);

    Ok(())
}

#[test]
fn essence_allocation_math() -> StdResult<()> {
    let essence_info = &EssenceInfo::new(20, 500_000_000, 40);
    let weights = &vec![
        WeightAllocationItem {
            lp_token: "eclip-atom".to_string(),
            weight: str_to_dec("0.2"),
        },
        WeightAllocationItem {
            lp_token: "ntrn-atom".to_string(),
            weight: str_to_dec("0.3"),
        },
        WeightAllocationItem {
            lp_token: "astro-atom".to_string(),
            weight: str_to_dec("0.5"),
        },
    ];
    let expected_essence_allocation = vec![
        EssenceAllocationItem {
            lp_token: "eclip-atom".to_string(),
            essence_info: EssenceInfo::new(4, 100_000_000, 8),
        },
        EssenceAllocationItem {
            lp_token: "ntrn-atom".to_string(),
            essence_info: EssenceInfo::new(6, 150_000_000, 12),
        },
        EssenceAllocationItem {
            lp_token: "astro-atom".to_string(),
            essence_info: EssenceInfo::new(10, 250_000_000, 20),
        },
    ];

    let essence_allocation = calc_essence_allocation(essence_info, weights);
    assert_eq!(essence_allocation, expected_essence_allocation);

    Ok(())
}

#[test]
fn updated_essence_allocation_math() -> StdResult<()> {
    let essence_info = &EssenceInfo::new(20, 500_000_000, 40);
    let alice_weights = &vec![
        WeightAllocationItem {
            lp_token: "eclip-atom".to_string(),
            weight: str_to_dec("0.2"),
        },
        WeightAllocationItem {
            lp_token: "ntrn-atom".to_string(),
            weight: str_to_dec("0.3"),
        },
        WeightAllocationItem {
            lp_token: "astro-atom".to_string(),
            weight: str_to_dec("0.5"),
        },
    ];
    let bob_weights_initial = &vec![
        WeightAllocationItem {
            lp_token: "eclip-atom".to_string(),
            weight: str_to_dec("0.2"),
        },
        WeightAllocationItem {
            lp_token: "ntrn-atom".to_string(),
            weight: str_to_dec("0.3"),
        },
        WeightAllocationItem {
            lp_token: "astro-atom".to_string(),
            weight: str_to_dec("0.5"),
        },
    ];
    let bob_weights_equal = &vec![
        WeightAllocationItem {
            lp_token: "eclip-atom".to_string(),
            weight: str_to_dec("0.5"),
        },
        WeightAllocationItem {
            lp_token: "ntrn-atom".to_string(),
            weight: str_to_dec("0.3"),
        },
        WeightAllocationItem {
            lp_token: "astro-atom".to_string(),
            weight: str_to_dec("0.2"),
        },
    ];
    let bob_weights_small = &vec![
        WeightAllocationItem {
            lp_token: "eclip-atom".to_string(),
            weight: str_to_dec("0.7"),
        },
        WeightAllocationItem {
            lp_token: "ntrn-atom".to_string(),
            weight: str_to_dec("0.3"),
        },
    ];
    let bob_weights_large = &vec![
        WeightAllocationItem {
            lp_token: "eclip-atom".to_string(),
            weight: str_to_dec("0.1"),
        },
        WeightAllocationItem {
            lp_token: "ntrn-atom".to_string(),
            weight: str_to_dec("0.3"),
        },
        WeightAllocationItem {
            lp_token: "astro-atom".to_string(),
            weight: str_to_dec("0.3"),
        },
        WeightAllocationItem {
            lp_token: "eclipastro-atom".to_string(),
            weight: str_to_dec("0.3"),
        },
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
    assert_eq!(
        essence_allocation,
        vec![
            EssenceAllocationItem {
                lp_token: "eclip-atom".to_string(),
                essence_info: EssenceInfo::new(8, 200_000_000, 16),
            },
            EssenceAllocationItem {
                lp_token: "ntrn-atom".to_string(),
                essence_info: EssenceInfo::new(12, 300_000_000, 24),
            },
            EssenceAllocationItem {
                lp_token: "astro-atom".to_string(),
                essence_info: EssenceInfo::new(20, 500_000_000, 40),
            },
        ]
    );

    // update bob allocation - equal amount of pools
    // (20, 500_000_000, 40) * (0.2, 0.3, 0.5) + (20, 500_000_000, 40) * (0.5, 0.3, 0.2) =
    // (20, 500_000_000, 40) * (0.7, 0.6, 0.7) =
    // [(14, 350_000_000, 28), (12, 300_000_000, 24), (14, 350_000_000, 28)]
    assert_eq!(
        calc_updated_essence_allocation(
            &essence_allocation,
            essence_allocation_bob_equal,
            essence_allocation_bob_initial,
        ),
        vec![
            EssenceAllocationItem {
                lp_token: "eclip-atom".to_string(),
                essence_info: EssenceInfo::new(14, 350_000_000, 28),
            },
            EssenceAllocationItem {
                lp_token: "ntrn-atom".to_string(),
                essence_info: EssenceInfo::new(12, 300_000_000, 24),
            },
            EssenceAllocationItem {
                lp_token: "astro-atom".to_string(),
                essence_info: EssenceInfo::new(14, 350_000_000, 28),
            },
        ]
    );

    // update bob allocation - less amount of pools
    // [(4, 100_000_000, 8), (6, 150_000_000, 12), (10, 250_000_000, 20)] +
    // [(14, 350_000_000, 28), (6, 150_000_000, 12), (0, 0, 0)] =
    // [(18, 450_000_000, 36), (12, 300_000_000, 24), (10, 250_000_000, 20)]
    assert_eq!(
        calc_updated_essence_allocation(
            &essence_allocation,
            essence_allocation_bob_small,
            essence_allocation_bob_initial,
        ),
        vec![
            EssenceAllocationItem {
                lp_token: "eclip-atom".to_string(),
                essence_info: EssenceInfo::new(18, 450_000_000, 36),
            },
            EssenceAllocationItem {
                lp_token: "ntrn-atom".to_string(),
                essence_info: EssenceInfo::new(12, 300_000_000, 24),
            },
            EssenceAllocationItem {
                lp_token: "astro-atom".to_string(),
                essence_info: EssenceInfo::new(10, 250_000_000, 20),
            },
        ]
    );

    // update bob allocation - greater amount of pools
    // [(4, 100_000_000, 8), (6, 150_000_000, 12), (10, 250_000_000, 20)] +
    // [(2, 50_000_000, 4), (6, 150_000_000, 12), (6, 150_000_000, 12), (6, 150_000_000, 12)] =
    // [(6, 150_000_000, 12), (12, 300_000_000, 24), (16, 400_000_000, 32), (6, 150_000_000, 12)]
    assert_eq!(
        calc_updated_essence_allocation(
            &essence_allocation,
            essence_allocation_bob_large,
            essence_allocation_bob_initial,
        ),
        vec![
            EssenceAllocationItem {
                lp_token: "eclip-atom".to_string(),
                essence_info: EssenceInfo::new(6, 150_000_000, 12),
            },
            EssenceAllocationItem {
                lp_token: "ntrn-atom".to_string(),
                essence_info: EssenceInfo::new(12, 300_000_000, 24),
            },
            EssenceAllocationItem {
                lp_token: "astro-atom".to_string(),
                essence_info: EssenceInfo::new(16, 400_000_000, 32),
            },
            EssenceAllocationItem {
                lp_token: "eclipastro-atom".to_string(),
                essence_info: EssenceInfo::new(6, 150_000_000, 12),
            },
        ]
    );

    Ok(())
}

#[test]
fn scaled_essence_allocation_math() -> StdResult<()> {
    let essence_info = &EssenceInfo::new(20, 500_000_000, 40);
    let weights = &vec![
        WeightAllocationItem {
            lp_token: "eclip-atom".to_string(),
            weight: str_to_dec("0.2"),
        },
        WeightAllocationItem {
            lp_token: "ntrn-atom".to_string(),
            weight: str_to_dec("0.3"),
        },
        WeightAllocationItem {
            lp_token: "astro-atom".to_string(),
            weight: str_to_dec("0.5"),
        },
    ];
    let essence_allocation = &calc_essence_allocation(essence_info, weights);
    let additional_essence = &EssenceInfo::new(20, 500_000_000, 40);
    let additional_essence_fraction = str_to_dec("0.5");
    let block_time: u64 = 50_000_000;

    let scaled_essence_allocation = calc_scaled_essence_allocation(
        essence_allocation,
        additional_essence,
        additional_essence_fraction,
        block_time,
    );
    // (20, 500_000_000, 40) * (0.2, 0.3, 0.5) * 1.5 =
    // [(6, 150_000_000, 12), (9, 225_000_000, 18), (15, 375_000_000, 30)]
    assert_eq!(
        scaled_essence_allocation,
        vec![
            EssenceAllocationItem {
                lp_token: "eclip-atom".to_string(),
                essence_info: EssenceInfo::new(6, 150_000_000, 12),
            },
            EssenceAllocationItem {
                lp_token: "ntrn-atom".to_string(),
                essence_info: EssenceInfo::new(9, 225_000_000, 18),
            },
            EssenceAllocationItem {
                lp_token: "astro-atom".to_string(),
                essence_info: EssenceInfo::new(15, 375_000_000, 30),
            },
        ]
    );

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
        WeightAllocationItem {
            lp_token: eclip_atom.to_string(),
            weight: str_to_dec("0.2"),
        },
        WeightAllocationItem {
            lp_token: ntrn_atom.to_string(),
            weight: str_to_dec("0.3"),
        },
        WeightAllocationItem {
            lp_token: astro_atom.to_string(),
            weight: str_to_dec("0.5"),
        },
    ];

    // stake
    for user in [alice, bob, john] {
        h.eclipsepad_staking_try_stake(user, 1_000, ECLIP)?;
    }

    let essence_info_alice = h.voter_query_user(alice, None)?;
    let essence_info_bob = h.voter_query_user(bob, None)?;
    let essence_info_john = h.voter_query_user(john, None)?;

    assert_eq!(
        essence_info_alice,
        UserResponse::Slacker {
            essence_info: EssenceInfo::new(1000, 1716163200000, 0),
            essence_value: Uint128::zero()
        }
    );
    assert_eq!(
        essence_info_bob,
        UserResponse::Slacker {
            essence_info: EssenceInfo::new(1000, 1716163200000, 0),
            essence_value: Uint128::zero()
        }
    );
    assert_eq!(
        essence_info_john,
        UserResponse::Slacker {
            essence_info: EssenceInfo::new(1000, 1716163200000, 0),
            essence_value: Uint128::zero()
        }
    );

    // take roles: alice - elector, bob - delegator, john - slacker
    h.voter_try_place_vote(alice, weights)?;
    h.voter_try_delegate(bob)?;

    let essence_info_alice = h.voter_query_user(alice, None)?;
    let essence_info_bob = h.voter_query_user(bob, None)?;
    let essence_info_john = h.voter_query_user(john, None)?;

    assert_eq!(
        essence_info_alice,
        UserResponse::Elector {
            essence_info: EssenceInfo::new(1000, 1716163200000, 0),
            essence_value: Uint128::zero(),
            weights: weights.to_vec()
        }
    );
    assert_eq!(
        essence_info_bob,
        UserResponse::Delegator {
            essence_info: EssenceInfo::new(1000, 1716163200000, 0),
            essence_value: Uint128::zero()
        }
    );
    assert_eq!(
        essence_info_john,
        UserResponse::Slacker {
            essence_info: EssenceInfo::new(1000, 1716163200000, 0),
            essence_value: Uint128::zero()
        }
    );

    // change essence for slacker, elector, delegator
    // lock
    for user in [alice, bob, john] {
        h.eclipsepad_staking_try_lock(user, 1_000, 4)?;
    }

    let essence_info_alice = h.voter_query_user(alice, None)?;
    let essence_info_bob = h.voter_query_user(bob, None)?;
    let essence_info_john = h.voter_query_user(john, None)?;

    assert_eq!(
        essence_info_alice,
        UserResponse::Elector {
            essence_info: EssenceInfo::new(0, 0, 1000),
            essence_value: Uint128::new(1000),
            weights: weights.to_vec()
        }
    );
    assert_eq!(
        essence_info_bob,
        UserResponse::Delegator {
            essence_info: EssenceInfo::new(0, 0, 1000),
            essence_value: Uint128::new(1000),
        }
    );
    assert_eq!(
        essence_info_john,
        UserResponse::Slacker {
            essence_info: EssenceInfo::new(0, 0, 1000),
            essence_value: Uint128::new(1000),
        }
    );

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
        WeightAllocationItem {
            lp_token: eclip_atom.to_string(),
            weight: str_to_dec("0.2"),
        },
        WeightAllocationItem {
            lp_token: ntrn_atom.to_string(),
            weight: str_to_dec("0.3"),
        },
        WeightAllocationItem {
            lp_token: astro_atom.to_string(),
            weight: str_to_dec("0.5"),
        },
    ];
    let weights_dao = &vec![
        WeightAllocationItem {
            lp_token: eclip_atom.to_string(),
            weight: str_to_dec("0.5"),
        },
        WeightAllocationItem {
            lp_token: ntrn_atom.to_string(),
            weight: str_to_dec("0.3"),
        },
        WeightAllocationItem {
            lp_token: astro_atom.to_string(),
            weight: str_to_dec("0.2"),
        },
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
        essence_info: EssenceInfo::new(0, 0, 1_000),
        essence_value: Uint128::new(1_000),
        weights: weights_alice.to_vec(),
    });
    assert_that(&essence_info_dao).is_equal_to(DaoResponse {
        essence_info: EssenceInfo::new(0, 0, 1_000),
        essence_value: Uint128::new(1_000),
        weights: weights_dao.to_vec(),
    });
    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time: 1716163200,
        elector_votes: vec![
            EssenceAllocationItem {
                lp_token: eclip_atom.to_string(),
                essence_info: EssenceInfo::new(0, 0, 200),
            },
            EssenceAllocationItem {
                lp_token: ntrn_atom.to_string(),
                essence_info: EssenceInfo::new(0, 0, 300),
            },
            EssenceAllocationItem {
                lp_token: astro_atom.to_string(),
                essence_info: EssenceInfo::new(0, 0, 500),
            },
        ],
        slacker_essence_acc: EssenceInfo::new(0, 0, 1_000),
        total_votes: vec![
            EssenceAllocationItem {
                lp_token: eclip_atom.to_string(),
                essence_info: EssenceInfo::new(0, 0, 700),
            },
            EssenceAllocationItem {
                lp_token: ntrn_atom.to_string(),
                essence_info: EssenceInfo::new(0, 0, 600),
            },
            EssenceAllocationItem {
                lp_token: astro_atom.to_string(),
                essence_info: EssenceInfo::new(0, 0, 700),
            },
        ],
        vote_results: vec![],
    });

    assert_that(&calc_weights_from_essence_allocation(&voter_info.elector_votes, block_time).1)
        .is_equal_to(weights_alice);
    assert_that(&calc_weights_from_essence_allocation(&voter_info.total_votes, block_time).1)
        .is_equal_to(vec![
            WeightAllocationItem {
                lp_token: eclip_atom.to_string(),
                weight: str_to_dec("0.35"),
            },
            WeightAllocationItem {
                lp_token: ntrn_atom.to_string(),
                weight: str_to_dec("0.3"),
            },
            WeightAllocationItem {
                lp_token: astro_atom.to_string(),
                weight: str_to_dec("0.35"),
            },
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
        essence_info: EssenceInfo::new(0, 0, 2_000),
        essence_value: Uint128::new(2_000),
        weights: weights_alice.to_vec(),
    });
    assert_that(&essence_info_dao).is_equal_to(DaoResponse {
        essence_info: EssenceInfo::new(0, 0, 1_000),
        essence_value: Uint128::new(1_000),
        weights: weights_dao.to_vec(),
    });
    assert_that(&voter_info).is_equal_to(VoterInfoResponse {
        block_time,
        elector_votes: vec![
            EssenceAllocationItem {
                lp_token: eclip_atom.to_string(),
                essence_info: EssenceInfo::new(0, 0, 400),
            },
            EssenceAllocationItem {
                lp_token: ntrn_atom.to_string(),
                essence_info: EssenceInfo::new(0, 0, 600),
            },
            EssenceAllocationItem {
                lp_token: astro_atom.to_string(),
                essence_info: EssenceInfo::new(0, 0, 1_000),
            },
        ],
        slacker_essence_acc: EssenceInfo::new(0, 0, 1_000),
        // (400, 600, 1_000) + (500, 300, 200) = (900, 900, 1_200)
        total_votes: vec![
            EssenceAllocationItem {
                lp_token: eclip_atom.to_string(),
                essence_info: EssenceInfo::new(0, 0, 900),
            },
            EssenceAllocationItem {
                lp_token: ntrn_atom.to_string(),
                essence_info: EssenceInfo::new(0, 0, 900),
            },
            EssenceAllocationItem {
                lp_token: astro_atom.to_string(),
                essence_info: EssenceInfo::new(0, 0, 1_200),
            },
        ],
        vote_results: vec![],
    });

    assert_that(&calc_weights_from_essence_allocation(&voter_info.elector_votes, block_time).1)
        .is_equal_to(weights_alice);
    // (900, 900, 1_200) / 3_000 = (0.3, 0.3, 0.4)
    assert_that(&calc_weights_from_essence_allocation(&voter_info.total_votes, block_time).1)
        .is_equal_to(vec![
            WeightAllocationItem {
                lp_token: eclip_atom.to_string(),
                weight: str_to_dec("0.3"),
            },
            WeightAllocationItem {
                lp_token: ntrn_atom.to_string(),
                weight: str_to_dec("0.3"),
            },
            WeightAllocationItem {
                lp_token: astro_atom.to_string(),
                weight: str_to_dec("0.4"),
            },
        ]);

    Ok(())
}

// #[test]
// fn slackers_electors_delegators_dao_voting() -> StdResult<()> {
//     let mut h = prepare_helper();
//     let ControllerHelper { astro, xastro, .. } = &ControllerHelper::new();
//     let alice = &h.acc(Acc::Alice);
//     let bob = &h.acc(Acc::Bob);

//     Ok(())
// }

// TODO
// +EssenceInfo math, captured essence
// +calc_essence_allocation
// +calc_updated_essence_allocation
// +calc_scaled_essence_allocation
// +auto-updating essence in voter
// +essence update will change weights
// 2 slackers + 2 electors + 2 delegators + dao (default voting)
// slackers + electors + dao
// slackers + delegators + dao
// electors + delegators + dao
// slackers + dao
// electors + dao
// delegators + dao
// electors
// delegators
// slackers
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
// historical data
// user voted in e1, delegated in e2, undelegated in e3 - rewards, weights, essence
// user delegated in e1, undelegated and voted in e2 - rewards, weights, essence
// delegate-undelegate loop - rewards, weights, essence
// vote-delegate-undelegate loop - rewards, weights, essence
// changing settings before next epoch
