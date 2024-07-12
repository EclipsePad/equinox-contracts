use crate::state::CONFIG;
use astroport::{
    asset::{Asset, AssetInfo},
    pair::{PoolResponse, QueryMsg as PoolQueryMsg},
    staking::QueryMsg as AstroportStakingQueryMsg,
};
use cosmwasm_std::{Deps, StdResult, Uint128, Uint256};
use equinox_msg::lp_depositor::Config;

/// query config
pub fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}
pub fn query_simulate(deps: Deps, asset: Asset) -> StdResult<Uint128> {
    let config = CONFIG.load(deps.storage)?;

    if asset.info.to_string() != config.eclipastro
        && asset.info.to_string() != config.astro
        && asset.info.to_string() != config.xastro
    {
        return Ok(Uint128::zero());
    }
    let astro_staking_total_deposit = query_astro_staking_total_deposit(deps)?;
    let astro_staking_total_shares = query_astro_staking_total_shares(deps)?;
    let amount_to_eclipastro = get_asset_amount_to_convert_eclipastro(deps, &asset)?;
    let eclipastro_amount = if asset.info.to_string() == config.xastro {
        amount_to_eclipastro.multiply_ratio(astro_staking_total_deposit, astro_staking_total_shares)
    } else {
        amount_to_eclipastro
    };
    let xastro_amount = if asset.info.to_string() == config.xastro {
        asset.amount - amount_to_eclipastro
    } else {
        (asset.amount - amount_to_eclipastro)
            .multiply_ratio(astro_staking_total_shares, astro_staking_total_deposit)
    };
    deps.querier.query_wasm_smart(
        config.lp_contract,
        &PoolQueryMsg::SimulateProvide {
            assets: vec![
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: config.eclipastro,
                    },
                    amount: eclipastro_amount,
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: config.xastro,
                    },
                    amount: xastro_amount,
                },
            ],
            slippage_tolerance: None,
        },
    )
}

pub fn get_asset_amount_to_convert_eclipastro(deps: Deps, asset: &Asset) -> StdResult<Uint128> {
    let config = CONFIG.load(deps.storage)?;
    let astro_staking_total_deposit = query_astro_staking_total_deposit(deps)?;
    let astro_staking_total_shares = query_astro_staking_total_shares(deps)?;
    let lp_pool_assets = query_lp_pool_assets(deps)?;
    let eclipastro_asset = lp_pool_assets
        .iter()
        .find(|a| a.info.to_string() == config.eclipastro)
        .unwrap();
    let xastro_asset = lp_pool_assets
        .iter()
        .find(|a| a.info.to_string() == config.xastro.clone())
        .unwrap();
    if eclipastro_asset.amount.is_zero() || xastro_asset.amount.is_zero() {
        return Ok(asset.amount.multiply_ratio(1u128, 2u128));
    }
    let numerator_one = Uint256::from_uint128(astro_staking_total_shares)
        .checked_mul(Uint256::from_uint128(eclipastro_asset.amount))
        .unwrap();
    let numerator_two = Uint256::from_uint128(astro_staking_total_deposit)
        .checked_mul(Uint256::from_uint128(xastro_asset.amount))
        .unwrap();
    let denominator = numerator_one + numerator_two;
    let mut amount_to_eclipastro = Uint256::from_uint128(asset.amount)
        .multiply_ratio(numerator_one, denominator)
        .try_into()
        .unwrap();
    if asset.info.to_string() == config.xastro {
        amount_to_eclipastro = Uint256::from_uint128(asset.amount)
            .multiply_ratio(numerator_two, denominator)
            .try_into()
            .unwrap();
    }
    Ok(amount_to_eclipastro)
}

pub fn query_astro_staking_total_deposit(deps: Deps) -> StdResult<Uint128> {
    let cfg = CONFIG.load(deps.storage)?;
    deps.querier.query_wasm_smart(
        cfg.staking_contract,
        &AstroportStakingQueryMsg::TotalDeposit {},
    )
}

pub fn query_astro_staking_total_shares(deps: Deps) -> StdResult<Uint128> {
    let cfg = CONFIG.load(deps.storage)?;
    deps.querier.query_wasm_smart(
        cfg.staking_contract,
        &AstroportStakingQueryMsg::TotalShares {},
    )
}

pub fn query_lp_pool_assets(deps: Deps) -> StdResult<Vec<Asset>> {
    let cfg = CONFIG.load(deps.storage)?;
    let response: PoolResponse = deps
        .querier
        .query_wasm_smart(cfg.lp_contract, &PoolQueryMsg::Pool {})?;
    Ok(response.assets)
}
