use crate::state::CONFIG;
use astroport::{
    asset::{Asset, AssetInfo},
    pair::{ConfigResponse, PoolResponse},
    pair_concentrated::{ConcentratedPoolParams, QueryMsg as ConcentratedQueryMsg},
    staking::QueryMsg as AstroportStakingQueryMsg,
    DecimalCheckedOps,
};
use cosmwasm_std::{from_json, Decimal, Deps, StdResult, Uint128};
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
    let mut eclipastro_amount = asset.amount;
    let mut xastro_amount = Uint128::zero();
    if asset.info.to_string() != config.eclipastro {
        let amount_to_eclipastro = get_asset_amount_to_convert_eclipastro(deps, &asset)?;
        eclipastro_amount = if asset.info.to_string() == config.xastro {
            amount_to_eclipastro
                .multiply_ratio(astro_staking_total_deposit, astro_staking_total_shares)
        } else {
            amount_to_eclipastro
        };
        xastro_amount = if asset.info.to_string() == config.xastro {
            asset.amount - amount_to_eclipastro
        } else {
            (asset.amount - amount_to_eclipastro)
                .multiply_ratio(astro_staking_total_shares, astro_staking_total_deposit)
        };
    }
    deps.querier.query_wasm_smart(
        config.lp_contract,
        &ConcentratedQueryMsg::SimulateProvide {
            assets: vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: config.eclipastro,
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
    let params = query_lp_pool_params(deps)?;

    let eclipastro_asset = lp_pool_assets
        .iter()
        .find(|a| a.info.to_string() == config.eclipastro)
        .unwrap();
    let xastro_asset = lp_pool_assets
        .iter()
        .find(|a| a.info.to_string() == config.xastro.clone())
        .unwrap();
    let price_scale = params.price_scale;
    let mut asset_amount_for_eclipastro = Uint128::zero();
    let mut asset_amount_remained = Uint128::zero();

    // eclipastro asset is bigger than xastro asset
    if Decimal::from_ratio(eclipastro_asset.amount, xastro_asset.amount).gt(&price_scale) {
        // xastro amount to add to match with price_scale
        let xastro_amount = Decimal::from_atomics(eclipastro_asset.amount, 0u32)
            .unwrap()
            .checked_div(price_scale)
            .unwrap()
            .to_uint_floor()
            .checked_sub(xastro_asset.amount)
            .unwrap();
        if asset.info.to_string() == config.astro {
            // astro amount to add to match with price_scale
            let astro_amount_to_xastro = xastro_amount
                .multiply_ratio(astro_staking_total_deposit, astro_staking_total_shares);
            if astro_amount_to_xastro.lt(&asset.amount) {
                asset_amount_remained = asset.amount.checked_sub(astro_amount_to_xastro).unwrap();
            }
        }
        if xastro_amount.lt(&asset.amount) {
            asset_amount_remained = asset.amount.checked_sub(xastro_amount).unwrap();
        }
    }
    // xastro asset is bigger than eclipastro asset
    if Decimal::from_ratio(eclipastro_asset.amount, xastro_asset.amount).lt(&price_scale) {
        // xastro amount to add to match with price_scale
        let eclipastro_amount = Decimal::from_atomics(xastro_asset.amount, 0u32)
            .unwrap()
            .checked_mul(price_scale)
            .unwrap()
            .to_uint_floor()
            .checked_sub(eclipastro_asset.amount)
            .unwrap();
        if asset.info.to_string() == config.astro {
            if eclipastro_amount.gt(&asset.amount) {
                asset_amount_for_eclipastro = asset.amount;
            } else {
                asset_amount_for_eclipastro = eclipastro_amount;
                asset_amount_remained = asset.amount.checked_sub(eclipastro_amount).unwrap();
            }
        } else {
            let xastro_amount_for_eclipastro = eclipastro_amount
                .multiply_ratio(astro_staking_total_shares, astro_staking_total_deposit);
            if xastro_amount_for_eclipastro.gt(&asset.amount) {
                asset_amount_for_eclipastro = asset.amount;
            } else {
                asset_amount_for_eclipastro = xastro_amount_for_eclipastro;
                asset_amount_remained = asset
                    .amount
                    .checked_sub(xastro_amount_for_eclipastro)
                    .unwrap();
            }
        }
    }
    if Decimal::from_ratio(eclipastro_asset.amount, xastro_asset.amount).eq(&price_scale) {
        asset_amount_remained = asset.amount;
    }

    if asset.info.to_string() == config.astro {
        Ok(asset_amount_remained
            .multiply_ratio(
                price_scale
                    .checked_mul_uint128(astro_staking_total_shares)
                    .unwrap(),
                astro_staking_total_deposit
                    .checked_add(
                        price_scale
                            .checked_mul_uint128(astro_staking_total_shares)
                            .unwrap(),
                    )
                    .unwrap(),
            )
            .checked_add(asset_amount_for_eclipastro)
            .unwrap())
    } else {
        Ok(asset_amount_remained.multiply_ratio(
            price_scale
                .checked_mul_uint128(astro_staking_total_deposit)
                .unwrap(),
            price_scale
                .checked_mul_uint128(astro_staking_total_shares)
                .unwrap()
                .checked_add(astro_staking_total_deposit)
                .unwrap(),
        ))
    }
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
        .query_wasm_smart(cfg.lp_contract, &ConcentratedQueryMsg::Pool {})?;
    Ok(response.assets)
}

pub fn query_lp_pool_params(deps: Deps) -> StdResult<ConcentratedPoolParams> {
    let cfg = CONFIG.load(deps.storage)?;
    let response: ConfigResponse = deps
        .querier
        .query_wasm_smart(cfg.lp_contract, &ConcentratedQueryMsg::Config {})?;
    from_json(response.params.unwrap())
}
