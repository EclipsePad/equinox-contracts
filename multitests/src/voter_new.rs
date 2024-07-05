use cosmwasm_std::{coins, StdResult};
use cw_multi_test::Executor;

use eclipse_base::assets::{Currency, Token};
use strum::IntoEnumIterator;
use voter::state::{EPOCH_LENGTH, GENESIS_EPOCH_START_DATE, VOTE_DELAY};

use crate::suite_astro::{
    extensions::{
        eclipsepad_staking::EclipsepadStakingExtension, minter::MinterExtension,
        voter::VoterExtension,
    },
    helper::{Acc, ControllerHelper},
};

const INITIAL_LIQUIDITY: u128 = 1_000_000;
const ECLIP: &str = "eclip";
const ECLIP_ASTRO: &str = "eclipastro";

fn prepare_helper() -> ControllerHelper {
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
        Some(vec![&h.acc(Acc::Owner).to_string()]),
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
        &h.acc(Acc::Owner).to_string(),
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
        h.mint_tokens(&h.acc(Acc::Owner), &coins(INITIAL_LIQUIDITY, token))
            .unwrap();
    }

    for user in Acc::iter() {
        for token in [ECLIP, &h.astro.clone(), &h.xastro.clone()] {
            h.app
                .send_tokens(
                    h.acc(Acc::Owner),
                    h.acc(user),
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
        &h.acc(Acc::Owner).to_string(),
        &Currency::new(&Token::new_native(ECLIP_ASTRO), 6),
        &h.voter_contract_address(),
    )
    .unwrap();

    h
}

#[test]
fn swap_to_eclip_astro_default() -> StdResult<()> {
    let mut h = prepare_helper();
    let ControllerHelper { astro, xastro, .. } = &ControllerHelper::new();

    let alice_astro = h.query_balance(&h.acc(Acc::Alice), astro);
    let alice_xastro = h.query_balance(&h.acc(Acc::Alice), xastro);
    let alice_eclip_astro = h.query_balance(&h.acc(Acc::Alice), ECLIP_ASTRO);
    assert_eq!(alice_astro, 100_000);
    assert_eq!(alice_xastro, 100_000);
    assert_eq!(alice_eclip_astro, 0);

    let bob_astro = h.query_balance(&h.acc(Acc::Bob), astro);
    let bob_xastro = h.query_balance(&h.acc(Acc::Bob), xastro);
    let bob_eclip_astro = h.query_balance(&h.acc(Acc::Bob), ECLIP_ASTRO);
    assert_eq!(bob_astro, 100_000);
    assert_eq!(bob_xastro, 100_000);
    assert_eq!(bob_eclip_astro, 0);

    h.voter_try_swap_to_eclip_astro(&h.acc(Acc::Alice), 1_000, astro)?;
    h.voter_try_swap_to_eclip_astro(&h.acc(Acc::Bob), 1_000, xastro)?;

    let alice_astro = h.query_balance(&h.acc(Acc::Alice), astro);
    let alice_xastro = h.query_balance(&h.acc(Acc::Alice), xastro);
    let alice_eclip_astro = h.query_balance(&h.acc(Acc::Alice), ECLIP_ASTRO);
    assert_eq!(alice_astro, 99_000);
    assert_eq!(alice_xastro, 100_000);
    assert_eq!(alice_eclip_astro, 1_000);

    let bob_astro = h.query_balance(&h.acc(Acc::Bob), astro);
    let bob_xastro = h.query_balance(&h.acc(Acc::Bob), xastro);
    let bob_eclip_astro = h.query_balance(&h.acc(Acc::Bob), ECLIP_ASTRO);
    assert_eq!(bob_astro, 100_000);
    assert_eq!(bob_xastro, 99_000);
    assert_eq!(bob_eclip_astro, 1_000);

    Ok(())
}
