use cosmwasm_std::{Addr, Decimal, Deps, Env, StdResult, Uint128};
use eclipse_base::converters::u128_to_dec;
use equinox_msg::voter::{AddressConfig, DateConfig, TokenConfig};

use crate::state::{ADDRESS_CONFIG, DATE_CONFIG, TOKEN_CONFIG};

pub fn query_address_config(deps: Deps, _env: Env) -> StdResult<AddressConfig> {
    ADDRESS_CONFIG.load(deps.storage)
}

pub fn query_token_config(deps: Deps, _env: Env) -> StdResult<TokenConfig> {
    TOKEN_CONFIG.load(deps.storage)
}

pub fn query_date_config(deps: Deps, _env: Env) -> StdResult<DateConfig> {
    DATE_CONFIG.load(deps.storage)
}

pub fn query_xastro_price(deps: Deps, _env: Env) -> StdResult<Decimal> {
    let AddressConfig {
        eclipsepad_staking, ..
    } = ADDRESS_CONFIG.load(deps.storage)?;

    let xastro_amount: Uint128 = deps.querier.query_wasm_smart(
        eclipsepad_staking.to_string(),
        &astroport::staking::QueryMsg::TotalShares {},
    )?;

    let astro_amount: Uint128 = deps.querier.query_wasm_smart(
        eclipsepad_staking.to_string(),
        &astroport::staking::QueryMsg::TotalDeposit {},
    )?;

    Ok(u128_to_dec(astro_amount) / u128_to_dec(xastro_amount))
}

/// query voting power
pub fn query_voting_power(deps: Deps, env: Env, address: String) -> StdResult<Uint128> {
    let voter_address = &env.contract.address;
    let address = &deps.api.addr_validate(&address)?;
    let AddressConfig {
        astroport_voting_escrow,
        eclipsepad_staking,
        ..
    } = ADDRESS_CONFIG.load(deps.storage)?;

    // query total vxASTRO owned by voter contract
    let vxastro_amount: Uint128 = deps.querier.query_wasm_smart(
        astroport_voting_escrow,
        &astroport_governance::voting_escrow::QueryMsg::UserVotingPower {
            user: voter_address.to_string(),
            timestamp: None,
        },
    )?;

    // voter contract has full voting power
    if address == voter_address {
        return Ok(vxastro_amount);
    }

    // TODO: calculate essence at the epoch start
    // query essence from eclipsepad-staking v3
    let eclipse_base::staking::msg::QueryEssenceResponse { essence, .. } =
        deps.querier.query_wasm_smart(
            eclipsepad_staking.clone(),
            &eclipse_base::staking::msg::QueryMsg::QueryEssence {
                user: address.to_string(),
            },
        )?;

    let eclipse_base::staking::msg::QueryEssenceResponse {
        essence: total_essence,
        ..
    } = deps.querier.query_wasm_smart(
        eclipsepad_staking,
        &eclipse_base::staking::msg::QueryMsg::QueryTotalEssence {},
    )?;

    let voting_power = vxastro_amount * essence / total_essence;

    Ok(voting_power)
}

// pub fn query_voter_info(
//     deps: Deps,
//     _env: Env,
//     address: String,
// ) -> StdResult<astroport_governance::generator_controller::UserInfoResponse> {
//     let address = &deps.api.addr_validate(&address)?;
//     let Config {
//         astroport_generator_controller,
//         ..
//     } = CONFIG.load(deps.storage)?;

//     deps.querier.query_wasm_smart(
//         astroport_generator_controller,
//         &astroport_governance::generator_controller::QueryMsg::UserInfo {
//             user: address.to_string(),
//         },
//     )
// }
