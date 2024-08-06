use cosmwasm_std::{DepsMut, Env, Response, Storage};
use cw2::{get_contract_version, set_contract_version};

use semver::Version;

use eclipse_base::{
    error::ContractError,
    staking::{msg::MigrateMsg, state::CONTRACT_NAME},
};

// v2.7.2 - vault aggregation
// v2.7.3 - current live version: staking auto merging
// v2.9.0 - conditional essence storages
// v3.1.0 - v3 rewards model
// v3.2.0 - bECLIP

// v2.7.2 -> v2.7.3
// 1) frontend: add vault aggregation UI

// v2.7.3 -> v2.9.0
// 1) frontend: v2v3 types, staking rewards, staking APR
// 2) update types, add empty essence storages via migrate_1, Pause will be applied automatically
// 3) filter staker and locker storages using FilterStakers, FilterLockers
// 4) fill essence storages using UpdateStakingEssenceStorages, UpdateLockingEssenceStorages
// 5) Unpause
// 6) calculate eclip_per_second to get proper APRs, use v3 APR query to tune eclip_per_second
// eclip_per_second = sum(apr_per_tier * total_bond_amount_per_tier) / year_in_seconds

// v2.9.0 -> v3.1.0
// 1) tune APRs if it's needed via migrations
// 2) remove unused code

pub fn migrate_contract(
    deps: DepsMut,
    env: Env,
    msg: MigrateMsg,
) -> Result<Response, ContractError> {
    let (version_previous, version_new) = get_versions(deps.storage, msg)?;

    if version_new >= version_previous {
        set_contract_version(deps.storage, CONTRACT_NAME, version_new.to_string())?;

        let block_time = env.block.time.seconds();

        // use only on local network
        if env.block.chain_id == "stargaze-0" {
            crate::e2e::add_users(deps.storage, block_time)?;
        }
    }

    Ok(Response::new())
}

fn get_versions(
    storage: &dyn Storage,
    msg: MigrateMsg,
) -> Result<(Version, Version), ContractError> {
    let version_previous: Version = get_contract_version(storage)?
        .version
        .parse()
        .map_err(|_| ContractError::ParsingPrevVersion)?;

    let version_new: Version = env!("CARGO_PKG_VERSION")
        .parse()
        .map_err(|_| ContractError::ParsingNewVersion)?;

    if version_new.to_string() != msg.version {
        Err(ContractError::ImproperMsgVersion)?;
    }

    Ok((version_previous, version_new))
}
