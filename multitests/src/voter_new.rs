use cosmwasm_std::{coins, Addr, StdResult};
use cw_multi_test::Executor;

use eclipse_base::assets::{Currency, Token};
use voter::state::{EPOCHS_START, EPOCH_LENGTH, VOTE_COOLDOWN};

use crate::suite_astro::{
    extensions::{
        eclipsepad_staking::EclipsepadStakingExtension, minter::MinterExtension,
        voter::VoterExtension,
    },
    helper::ControllerHelper,
};

const INITIAL_LIQUIDITY: u128 = 1_000_000;
const ECLIP: &str = "eclip";
const ECLIP_ASTRO: &str = "eclipastro";

fn prepare_helper() -> StdResult<ControllerHelper> {
    let mut h = ControllerHelper::new();

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
        Some(vec![&h.owner.to_string()]),
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
        EPOCHS_START,
        EPOCH_LENGTH,
        VOTE_COOLDOWN,
    );

    h.eclipsepad_staking_try_update_config(
        &h.owner.to_string(),
        None,
        Some(h.voter_contract_address()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    for token in [ECLIP, &h.astro.clone()] {
        h.mint_tokens(
            &Addr::unchecked(&h.owner.to_string()),
            &coins(INITIAL_LIQUIDITY, token),
        )
        .unwrap();
    }

    for user in [h.alice.clone(), h.bob.clone()] {
        for token in [ECLIP, &h.astro.clone(), &h.xastro.clone()] {
            h.app
                .send_tokens(
                    h.owner.clone(),
                    Addr::unchecked(user.clone()),
                    &coins(INITIAL_LIQUIDITY / 10, token),
                )
                .unwrap();
        }
    }

    h.mint_tokens(
        &h.minter_contract_address(),
        &coins(INITIAL_LIQUIDITY, ECLIP_ASTRO),
    )
    .unwrap();

    h.minter_try_register_currency(
        &h.owner.to_string(),
        &Currency::new(&Token::new_native(ECLIP_ASTRO), 6),
        &h.voter_contract_address(),
    )?;

    Ok(h)
}

#[test]
fn swap_to_eclip_astro_default() -> StdResult<()> {
    let mut h = prepare_helper()?;
    let ControllerHelper {
        alice,
        bob,
        astro,
        xastro,
        ..
    } = &prepare_helper()?;

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
