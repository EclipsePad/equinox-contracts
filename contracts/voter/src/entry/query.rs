use cosmwasm_std::{Addr, Deps, Env, StdResult, Uint128};
use equinox_msg::voter::{AddressConfig, DateConfig};

use crate::state::{ADDRESS_CONFIG, DATE_CONFIG};
use astroport::staking::QueryMsg as AstroStakingQueryMsg;

pub fn query_date_config(deps: Deps, _env: Env) -> StdResult<DateConfig> {
    DATE_CONFIG.load(deps.storage)
}

// /// query convert ratio
// pub fn query_convert_ratio(deps: Deps, _env: Env) -> StdResult<(Uint128, Uint128)> {
//     let config = CONFIG.load(deps.storage)?;
//     // xASTRO amount
//     let total_shares: Uint128 = deps.querier.query_wasm_smart(
//         config.staking_contract.to_string(),
//         &AstroStakingQueryMsg::TotalShares {},
//     )?;
//     // ASTRO amount
//     let total_deposit: Uint128 = deps.querier.query_wasm_smart(
//         config.staking_contract.to_string(),
//         &AstroStakingQueryMsg::TotalDeposit {},
//     )?;
//     Ok((total_deposit, total_shares))
// }

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
