use cosmwasm_std::{DepsMut, Env, Response, Storage};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use equinox_msg::single_sided_staking::MigrateMsg;

use crate::{error::ContractError, state::CONTRACT_NAME};

pub fn migrate_contract(
    deps: DepsMut,
    _env: Env,
    msg: MigrateMsg,
) -> Result<Response, ContractError> {
    let (version_previous, version_new) = get_versions(deps.storage, msg)?;

    if version_new >= version_previous {
        set_contract_version(deps.storage, CONTRACT_NAME, version_new.to_string())?;

        // migration logic
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

// /// Manages contract migration.
// #[cfg_attr(not(feature = "library"), entry_point)]
// pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
//     let version: Version = CONTRACT_VERSION.parse()?;
//     let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;
//     let contract_name = get_contract_version(deps.storage)?.contract;

//     match msg.update_contract_name {
//         Some(true) => {}
//         _ => {
//             ensure_eq!(
//                 contract_name,
//                 CONTRACT_NAME,
//                 ContractError::ContractNameErr(contract_name)
//             );
//         }
//     }

//     ensure_eq!(
//         (version > storage_version),
//         true,
//         ContractError::VersionErr(storage_version.to_string())
//     );

//     if version > storage_version {
//         set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
//     }

//     if let Some(update_rewards) = msg.update_rewards {
//         let (time_config, new_reward) = update_rewards;
//         REWARD.update(deps.storage, time_config, |reward| -> StdResult<_> {
//             if let Some(old_reward) = reward {
//                 if old_reward.eclip + old_reward.beclip == new_reward.eclip + new_reward.beclip {
//                     return Ok(new_reward);
//                 }
//             }
//             Err(StdError::generic_err("Update Rewards error"))
//         })?;
//     }

//     Ok(Response::new().add_attribute("new_contract_version", CONTRACT_VERSION))
// }
