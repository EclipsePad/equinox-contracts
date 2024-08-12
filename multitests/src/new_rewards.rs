use astroport::{asset::AssetInfo, router::SwapOperation};
use astroport_governance::emissions_controller::hub::{
    AstroPoolConfig, OutpostInfo, OutpostParams, VotedPoolInfo,
};

use cosmwasm_std::{coins, Addr, Decimal, StdResult, Uint128};
use cw_multi_test::Executor;

use speculoos::assert_that;
use strum::IntoEnumIterator;

use eclipse_base::{converters::str_to_dec, error::parse_err};

use equinox_msg::{
    single_sided_staking::{RewardConfig, RewardDetail, TimeLockConfig},
    voter::{
        msg::{DaoResponse, UserResponse, VoterInfoResponse},
        state::{EPOCH_LENGTH, GENESIS_EPOCH_START_DATE, VOTE_DELAY},
        types::{
            BribesAllocationItem, EssenceAllocationItem, EssenceInfo, PoolInfoItem, RewardsInfo,
            RouteItem, RouteListItem, UserType, VoteResults, WeightAllocationItem,
        },
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
        minter::MinterExtension, single_sided_staking::SingleSidedStakingExtension,
        token_converter::TokenConverterExtension, tribute_market_mocks::TributeMarketExtension,
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
    h.minter_prepare_contract(&None, &None, &None, &None, &None);
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
        Some(vec![owner.as_ref()]),
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

    h.minter_try_register_native(
        owner,
        &Denom::EclipAstro.to_string(),
        &None,
        &Some(vec![h.voter_contract_address()]),
        &None,
        &None,
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
            &[
                (100 * INITIAL_LIQUIDITY, Denom::Eclip),
                (100 * INITIAL_LIQUIDITY, Denom::Atom),
            ],
        ),
        BribesAllocationItem::new(
            h.pool(Pool::NtrnAtom),
            &[
                (200 * INITIAL_LIQUIDITY, Denom::Ntrn),
                (120 * INITIAL_LIQUIDITY, Denom::Atom),
            ],
        ),
        BribesAllocationItem::new(
            h.pool(Pool::AstroAtom),
            &[(100 * INITIAL_LIQUIDITY, Denom::Astro)],
        ),
    ];

    h.tribute_market_try_set_bribes_allocation(
        &h.tribute_market_contract_address(),
        owner,
        &bribes_allocation,
    )
    .unwrap();

    h
}

fn add_eclipsepad_tribute_market(h: &mut ControllerHelper) -> Addr {
    let owner = &h.acc(Acc::Owner);

    let eclipsepad_tribute_market_contract =
        h.tribute_market_instantiate_contract(&h.vxastro.clone(), &h.emission_controller.clone());

    h.voter_try_update_address_config(
        owner,
        None::<Addr>,
        None::<Vec<Addr>>,
        None,
        None,
        None,
        None,
        Some(eclipsepad_tribute_market_contract.clone()),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();

    // add bribes in tribute market
    let bribes_allocation: Vec<BribesAllocationItem> = vec![
        BribesAllocationItem::new(
            h.pool(Pool::EclipAtom),
            &[
                (100 * INITIAL_LIQUIDITY, Denom::Eclip),
                (100 * INITIAL_LIQUIDITY, Denom::Atom),
            ],
        ),
        BribesAllocationItem::new(
            h.pool(Pool::NtrnAtom),
            &[
                (200 * INITIAL_LIQUIDITY, Denom::Ntrn),
                (120 * INITIAL_LIQUIDITY, Denom::Atom),
            ],
        ),
        BribesAllocationItem::new(
            h.pool(Pool::AstroAtom),
            &[(100 * INITIAL_LIQUIDITY, Denom::Astro)],
        ),
    ];

    h.tribute_market_try_set_bribes_allocation(
        &eclipsepad_tribute_market_contract,
        owner,
        &bribes_allocation,
    )
    .unwrap();

    eclipsepad_tribute_market_contract
}

fn add_token_converter_and_flexible_vault(h: &mut ControllerHelper) {
    let astro = &h.astro.clone();
    let xastro = &h.xastro.clone();
    const FAKE_BECLIP: &str = "neutron1yme3yf9ce9z4qdte7n9s8gsavvxr8c92jr6tyz";
    let owner = &h.acc(Acc::Owner);

    h.token_converter_prepare_contract(astro, xastro, &h.staking.clone(), owner);
    h.single_sided_staking_prepare_contract(
        &Denom::EclipAstro.to_string(),
        &Some(vec![
            TimeLockConfig {
                duration: 0,
                early_unlock_penalty_bps: 0,
                reward_multiplier: 10000,
            },
            TimeLockConfig {
                duration: 86400 * 30,
                early_unlock_penalty_bps: 5000,
                reward_multiplier: 20000,
            },
            TimeLockConfig {
                duration: 86400 * 30 * 3,
                early_unlock_penalty_bps: 5000,
                reward_multiplier: 60000,
            },
            TimeLockConfig {
                duration: 86400 * 30 * 6,
                early_unlock_penalty_bps: 5000,
                reward_multiplier: 120000,
            },
            TimeLockConfig {
                duration: 86400 * 30 * 9,
                early_unlock_penalty_bps: 5000,
                reward_multiplier: 180000,
            },
            TimeLockConfig {
                duration: 86400 * 365,
                early_unlock_penalty_bps: 5000,
                reward_multiplier: 240000,
            },
        ]),
        &RewardConfig {
            eclip: RewardDetail {
                info: AssetInfo::NativeToken {
                    denom: Denom::Eclip.to_string(),
                },
                daily_reward: Uint128::from(1_000_000u128),
            },
            beclip: RewardDetail {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked(FAKE_BECLIP),
                },
                daily_reward: Uint128::from(2_000_000u128),
            },
        },
        &h.token_converter_contract_address(),
        owner,
    );
}

#[test]
fn default() -> StdResult<()> {
    let mut h = prepare_helper();
    add_token_converter_and_flexible_vault(&mut h);
    let ControllerHelper { astro, xastro, .. } = &ControllerHelper::new();

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

    // h.eclipsepad_staking_try_stake(alice, 1_000, Denom::Eclip)?;
    // h.eclipsepad_staking_try_lock(alice, 1_000, 4)?;

    Ok(())
}
