use std::str::FromStr;

use cosmwasm_std::{
    ensure_eq, entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use equinox_msg::lockdrop::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, UserAdjustRewards};
use semver::Version;

use crate::{
    entry::{
        execute::{
            _handle_callback, receive_cw20, try_claim_all_rewards, try_claim_blacklist_rewards,
            try_claim_rewards, try_extend_lockup, try_increase_incentives, try_increase_lockup,
            try_stake_to_vaults, try_unbond, try_unlock, try_update_config,
            try_update_lockdrop_periods, try_update_owner, try_update_reward_distribution_config,
        },
        instantiate::try_instantiate,
        query::{
            query_blacklist, query_blacklist_rewards, query_calculate_penalty_amount, query_config,
            query_incentives, query_lp_lockup_info, query_lp_lockup_state, query_owner,
            query_reward_config, query_single_lockup_info, query_single_lockup_state,
            query_user_lp_lockup_info, query_user_single_lockup_info,
        },
    },
    error::ContractError,
    state::{ADJUST_REWARDS, CONFIG, CONTRACT_NAME, CONTRACT_VERSION, SINGLE_USER_LOCKUP_INFO},
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    try_instantiate(deps, env, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { new_config } => try_update_config(deps, info, new_config),
        ExecuteMsg::UpdateRewardDistributionConfig { new_config } => {
            try_update_reward_distribution_config(deps, env, info, new_config)
        }
        ExecuteMsg::IncreaseLockup {
            stake_type,
            duration,
        } => try_increase_lockup(deps, env, info, stake_type, duration),
        ExecuteMsg::ExtendLock {
            stake_type,
            from,
            to,
        } => try_extend_lockup(deps, env, info, stake_type, from, to),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Unlock {
            stake_type,
            duration,
            amount,
        } => try_unlock(deps, env, info, stake_type, duration, amount),
        ExecuteMsg::StakeToVaults {} => try_stake_to_vaults(deps, env, info),
        ExecuteMsg::ClaimRewards {
            stake_type,
            duration,
            assets,
        } => try_claim_rewards(deps, env, info, stake_type, duration, assets),
        ExecuteMsg::ClaimAllRewards {
            stake_type,
            with_flexible,
            assets,
        } => try_claim_all_rewards(deps, env, info, stake_type, with_flexible, assets),
        ExecuteMsg::Callback(msg) => _handle_callback(deps, env, info, msg),
        ExecuteMsg::IncreaseIncentives { rewards } => {
            try_increase_incentives(deps, env, info, rewards)
        }
        ExecuteMsg::UpdateOwner { new_owner } => try_update_owner(deps, env, info, new_owner),
        ExecuteMsg::ClaimBlacklistRewards {} => try_claim_blacklist_rewards(deps, env),
        ExecuteMsg::UpdateLockdropPeriods { deposit, withdraw } => {
            try_update_lockdrop_periods(deps, env, info, deposit, withdraw)
        }
        ExecuteMsg::Unbond {} => try_unbond(deps, env, info),
    }
}

/// Exposes queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps, env)?)?),
        QueryMsg::RewardConfig {} => Ok(to_json_binary(&query_reward_config(deps, env)?)?),
        QueryMsg::Owner {} => Ok(to_json_binary(&query_owner(deps, env)?)?),
        QueryMsg::SingleLockupInfo {} => Ok(to_json_binary(&query_single_lockup_info(deps, env)?)?),
        QueryMsg::LpLockupInfo {} => Ok(to_json_binary(&query_lp_lockup_info(deps, env)?)?),
        QueryMsg::SingleLockupState {} => {
            Ok(to_json_binary(&query_single_lockup_state(deps, env)?)?)
        }
        QueryMsg::LpLockupState {} => Ok(to_json_binary(&query_lp_lockup_state(deps, env)?)?),
        QueryMsg::UserSingleLockupInfo { user } => Ok(to_json_binary(
            &query_user_single_lockup_info(deps, env, user)?,
        )?),
        QueryMsg::UserLpLockupInfo { user } => Ok(to_json_binary(&query_user_lp_lockup_info(
            deps, env, user,
        )?)?),
        QueryMsg::Incentives { stake_type } => {
            Ok(to_json_binary(&query_incentives(deps, stake_type)?)?)
        }
        QueryMsg::Blacklist {} => Ok(to_json_binary(&query_blacklist(deps)?)?),
        QueryMsg::BlacklistRewards {} => Ok(to_json_binary(&query_blacklist_rewards(deps, env)?)?),
        QueryMsg::CalculatePenaltyAmount { amount, duration } => Ok(to_json_binary(
            &query_calculate_penalty_amount(deps, env, amount, duration)?,
        )?),
    }
}

/// Manages contract migration.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;
    let contract_name = get_contract_version(deps.storage)?.contract;
    let cfg = CONFIG.load(deps.storage)?;

    match msg.update_contract_name {
        Some(true) => {}
        _ => {
            ensure_eq!(
                contract_name,
                CONTRACT_NAME,
                ContractError::ContractNameErr(contract_name)
            );
        }
    }

    ensure_eq!(
        (version > storage_version),
        true,
        ContractError::VersionErr(storage_version.to_string())
    );

    if version > storage_version {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    }

    let more_claimed_rewards: Vec<UserAdjustRewards> = vec![
        UserAdjustRewards {
            user: "neutron1ktaqdmlchv065tlt49c50ecmhqejw8ugdtp6pv".to_string(),
            amount: Uint128::from_str("285064690").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron17zxc4ypxz57pu8z7t9e3wv9f7dd52qsgykee4n".to_string(),
            amount: Uint128::from_str("55334115356").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1cnzcqss00dv09kpwqmuxkqssevr7u77j75gyhd".to_string(),
            amount: Uint128::from_str("39685621").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1wv40aq6p9x2mfclt66wgznwefk298p53epr4hd".to_string(),
            amount: Uint128::from_str("2987934087").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron136wrasqzaplzsakzf62g0czq3jeh6wxn9crmtj".to_string(),
            amount: Uint128::from_str("34702933").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1ej828l97c3jxd88vr8c26qy3lekpmsqyjpy3ll".to_string(),
            amount: Uint128::from_str("95390285").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1m2nlz3024wmpwmytmeawntdvglde2vx7nf8lpr".to_string(),
            amount: Uint128::from_str("663261").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron10hvz04hh92xzct5hxnpsn5h2fp3p4ammcg425r".to_string(),
            amount: Uint128::from_str("5104025").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron10a70lpas3rkydmpnqs0dtr5my5kt8ggfem6u7z".to_string(),
            amount: Uint128::from_str("500367330").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1zf0836n2gjlevnc8jekurn939xn688ydp6a2au".to_string(),
            amount: Uint128::from_str("1478564611").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron18thwcddjk76wkyytu0vym9z9sl3mkzhjm80wfh".to_string(),
            amount: Uint128::from_str("167441204").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1cgxsf2kt729a42eegqqw6q2el7g764hl0snjdm".to_string(),
            amount: Uint128::from_str("648494986").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1l0q6w9gemqhaml8e2s9ptw4su4svweax7k9g7a".to_string(),
            amount: Uint128::from_str("154050293").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1qms0znsxnagqr56welqdu90wu00fspc8gn7l4q".to_string(),
            amount: Uint128::from_str("1736285612").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1lynr3f7zqhpcwacvatewcwj0ar5q9sgxxpy074".to_string(),
            amount: Uint128::from_str("1880250038").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1la4nlstynp2wl0r236du0evjpqh8h9fq0m87w5".to_string(),
            amount: Uint128::from_str("1020782170").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron18qqmyc57wemv4qkcmcwpvvwjnmpkv2w62c5dvs".to_string(),
            amount: Uint128::from_str("24671427").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1t3usy5x8xfggzspknnka02ny7u65u6k04uyew9".to_string(),
            amount: Uint128::from_str("583857841").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1sypqufzxkqqmul7g8f8xz0hjfkgsj9quwcujuk".to_string(),
            amount: Uint128::from_str("4673392").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1pyzxvfa7f8gr5x20n8kl0lu78jczuevn974805".to_string(),
            amount: Uint128::from_str("176301616").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1z4374xv43seaxh5r2gr03h990hfaj0564r89vu".to_string(),
            amount: Uint128::from_str("131986384").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron17rx4caclnlkqwlkq4hq3aq0cvj993pnfvkzsas".to_string(),
            amount: Uint128::from_str("567324600").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1u5gaduhhs0ht3aqj4u74wfpxrqn0nwuq362f46".to_string(),
            amount: Uint128::from_str("57153061").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1nzzdvyack8qv63d854g58ktm0xyxhq4skatrwq".to_string(),
            amount: Uint128::from_str("19252").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1jp9hyv8sj9ucswsmwraydg54cxqvapsmkw4pwl".to_string(),
            amount: Uint128::from_str("36312880").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1y0t5g650jcn49v66ta4ueumkd6qwxp4qt7egl0".to_string(),
            amount: Uint128::from_str("204870280").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1q9t5fa4zhv9pn52snhyga4y54fceyhrdvxus3q".to_string(),
            amount: Uint128::from_str("101028572").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1ypprtuvnr38pa98ypffx4yg3p9jj77y427v4pm".to_string(),
            amount: Uint128::from_str("6688").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1xn2lfrcm5nd3lf4s703pugfuvelxcf922r0924".to_string(),
            amount: Uint128::from_str("29993718").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron18ypcg06xvmjvwam5jxucaazanyn00g5jkv52f3".to_string(),
            amount: Uint128::from_str("17635742").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1g692jde5e7nqpml4rmxgge22m3qvft8utrcrph".to_string(),
            amount: Uint128::from_str("37364220").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron12sq9v0gmz3wvvye7zve45s03693hdmzwjwh5v4".to_string(),
            amount: Uint128::from_str("94214").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1jj3xudkz3n60j9paq66xzejtmwzvrmn8as99nv".to_string(),
            amount: Uint128::from_str("72926394").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1rxdslevtdk4xgju50g4pa3ktf93x9gjvt45mrf".to_string(),
            amount: Uint128::from_str("47829").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1l4y9c7y9esvnuw2ewgfc29f9sscur385wsrvg6".to_string(),
            amount: Uint128::from_str("103928146").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1xgx7ux8sw49nadsne8hcjemt55j7nkqtupj2jh".to_string(),
            amount: Uint128::from_str("140817283").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1kzu6yxgdag4r0y4h3fdsp9kzsrrmydnknc65ht".to_string(),
            amount: Uint128::from_str("27023073").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1qeu7uxxsthwcxcevzlwatjfnvew44tnlue2jc4".to_string(),
            amount: Uint128::from_str("86331840").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron145kvyy7njm970wu58ytpv38caxl7yvm544h4v4".to_string(),
            amount: Uint128::from_str("892732497").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1tveyveuc2t8arvj0lxac9mzagxl29tk33xyzs3".to_string(),
            amount: Uint128::from_str("51139841").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1zqlq0slcear0rggnvcgsxpv99mtlyck8heav28".to_string(),
            amount: Uint128::from_str("246616797").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1yen5f0ej9njg0d9pa8nn2hwjpqqm36zjr2p785".to_string(),
            amount: Uint128::from_str("199699936").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron18tnusz5r6emg8uk2a988zq685a5gpfxummzr7m".to_string(),
            amount: Uint128::from_str("108042403").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1vt2jalakgpvuln3nj698ss80nr8x4cmep40uv0".to_string(),
            amount: Uint128::from_str("133690853").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron18a3eyd0mfpruc6y9g60hljdx5jn2f5tra4ylrc".to_string(),
            amount: Uint128::from_str("18203625").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron10rdut36lnhsrudd3t4zqqgpvrmxam4led82zwa".to_string(),
            amount: Uint128::from_str("518385313").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1chncjjdkvcjah0f934lmdvqj94e549e9shhcsp".to_string(),
            amount: Uint128::from_str("27332").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1ud5s9ll94pftkp3ye5sj4gdyu3asjkgs3rny0h".to_string(),
            amount: Uint128::from_str("311641977").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron17kauqk0hc5hut2af3e0a6t63n7f7hr6wma5hl2".to_string(),
            amount: Uint128::from_str("57875428").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron197nvjpgu79qsw27zzk0gyt9qh8cmq8wdyypwg4".to_string(),
            amount: Uint128::from_str("22907047").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1s3n42apstpdcqnyhc0cqavr78kgxl4x0k5n3sd".to_string(),
            amount: Uint128::from_str("61764800").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1gjej6szndkuaadfsu4u8mhrul6qhyfrqnzn5s3".to_string(),
            amount: Uint128::from_str("73013").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1x66f4rcnyx0u23qk72u6kakk7yhh89v2rp6gga".to_string(),
            amount: Uint128::from_str("4680").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1cerae8nhevv3jlq96w3x3wk3z09wfx5e0pv5mk".to_string(),
            amount: Uint128::from_str("104353").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron19xaz6wwtypjh7292zwg35q932nztzxfa83s5fz".to_string(),
            amount: Uint128::from_str("1430968734").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron18mgj6w322m4agfekvdn5fynfypnn2mmeagcp6u".to_string(),
            amount: Uint128::from_str("1182451361").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron12tpkd4ftz73a2ap8s9tmql7f9q9hrczn0283ty".to_string(),
            amount: Uint128::from_str("397832362").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron10l40z2kcd2fs8zvuxtr4m5wxqwzzr52ynkmyew".to_string(),
            amount: Uint128::from_str("174076300").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1trzmw63uq50r30c9rlrz766mxk76tx0fdlksnw".to_string(),
            amount: Uint128::from_str("61255").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron16m4yxukw8sgd0k7w3mwa29kqejq4ym6xxg9ag6".to_string(),
            amount: Uint128::from_str("2114776").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron17qg3pf85kj56z8lye25kpwvjx6cgftq05sw3jl".to_string(),
            amount: Uint128::from_str("7056").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron17w0jscv784knz238avr0dajzmdh2l5fqxunpat".to_string(),
            amount: Uint128::from_str("44020403").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron13w44qxlgl7ued0w6jep6yca73htmrwas08znxf".to_string(),
            amount: Uint128::from_str("252149742").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1qf08dql0ufmq78rs9ecnk9cexm3aspre338czp".to_string(),
            amount: Uint128::from_str("75841417").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1f4w0hhfm6c6e9q80f3u8zf4fht2vm54veyt80n".to_string(),
            amount: Uint128::from_str("62922022").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1d2r0uw49jgfeg8rcs8dp9alfq5sqyht4wy9spt".to_string(),
            amount: Uint128::from_str("17498476").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1v8h45qelm6r7ucsdmex48ntkvnrnq9urkyptah".to_string(),
            amount: Uint128::from_str("49306").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1najzatgap3dq92hk9uax69ymkdmx2qtyx2j94k".to_string(),
            amount: Uint128::from_str("10384954").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1haa2p9zsnklupq80a449ex0d5y0xqa39ycshap".to_string(),
            amount: Uint128::from_str("30664").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1x5sms7rp44jqg8v9qgn92kal6ld38zqyzuk0v7".to_string(),
            amount: Uint128::from_str("3957894").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron19t5vjq8jlp5uy3jp4eu2nwms6u5yv7gnjtkwar".to_string(),
            amount: Uint128::from_str("32626756").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1up5xyleh6qrc0lnpnszp7tqnwqu4edkzg2qgs4".to_string(),
            amount: Uint128::from_str("4536569").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1uyphdztvh3r4jv23utrht7qtgf0ref34u607qd".to_string(),
            amount: Uint128::from_str("35516").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1fs3pd75pgd0qjjz324fzmfxtrm9g2dcjtqae9e".to_string(),
            amount: Uint128::from_str("7182148").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1d68rccckj3zfgqayat5sexl6gvsrgy3x0ps09s".to_string(),
            amount: Uint128::from_str("8052886").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron10pfvzsq22ylk30n2vt2mk8937jmpzggs9fjthy".to_string(),
            amount: Uint128::from_str("87279736").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1lxzzvjxsegryqs3k8jlghce2gmxg5dtr4tj0fy".to_string(),
            amount: Uint128::from_str("35754").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1kkwklh7kyr20ktq29uctagkxc7rc27ymr3mtfs".to_string(),
            amount: Uint128::from_str("2875976").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1h82nvuc4gw53l58ccjlqy3wdlttnnkes4e9zza".to_string(),
            amount: Uint128::from_str("164377993").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1y0r96xq4glwxpg0jgcauu3yufhn2l8dvpqj0kk".to_string(),
            amount: Uint128::from_str("76238").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron10ns6fccrf7zdmmdsehrgng7grmqc2let56qr82".to_string(),
            amount: Uint128::from_str("286").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1m75uf4f0cn0qklwyxwaujmpd49xxhhuyzj9au2".to_string(),
            amount: Uint128::from_str("49176069").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1lwrna9hj3awewqr8ryx2wlpdc2dcgq7asd5na9".to_string(),
            amount: Uint128::from_str("22").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron16r5q7lef9nyv79djge3yw55l9x67g2yxl0v8sa".to_string(),
            amount: Uint128::from_str("18734837").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1estvx6e2fe49na58mn5yrmuxwxduyregp7j33d".to_string(),
            amount: Uint128::from_str("268417").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron16he9gjnqghk0ljhx8v25w5y2wugnaddq6pnadp".to_string(),
            amount: Uint128::from_str("226859661").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1ez6r6jj0fgxauy3z9j8n85myjwnzt49nw3fkz2".to_string(),
            amount: Uint128::from_str("4628412304").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron18r536pthsh6ya00298c37matrmhm5rcppz5k8u".to_string(),
            amount: Uint128::from_str("292421439").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1d6ytk5nmc64r6p3mjfdqg9g5rlm5ca3m75587t".to_string(),
            amount: Uint128::from_str("14173147").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1wsc62azzkuxl42au9et5x8j27mcfxm8vpt3zlw".to_string(),
            amount: Uint128::from_str("2055011").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1jml69sl5cur3qhrx0lk23q6jsslarwft9rh67l".to_string(),
            amount: Uint128::from_str("75573966").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron19e5lvf5lnkupj4x2hefjvw4486hnjpvwt5hpqw".to_string(),
            amount: Uint128::from_str("43658459").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1sp7t2x56z2dvut66lxtke0wf097tv8fpavn0sg".to_string(),
            amount: Uint128::from_str("5998218").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1nv4vn7hq4ln32jpe0qw0vsv8mmkp9durx0sm6k".to_string(),
            amount: Uint128::from_str("3254513").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1nlwft7nqz8cyytjkn3xhfdwalk8rdvpyauwe70".to_string(),
            amount: Uint128::from_str("20426283").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1uaux0dl3gh40y9czng2eqn2d52uu8afv5ujcdg".to_string(),
            amount: Uint128::from_str("22509127").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron13ukrwufmm4sgwgptvhz7ll34h5flqjnkva04um".to_string(),
            amount: Uint128::from_str("976585530").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1rqqzgxs8ty03rm5s7f9x94wj4tnx404wf23t0f".to_string(),
            amount: Uint128::from_str("24063807").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1xrrfdlh2tattd4rq8uz5s6d58rxz4xy8lm2cqp".to_string(),
            amount: Uint128::from_str("3212538").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1gm9stluhw4xklwyv5kwx9v3veqsm2uq958qs8j".to_string(),
            amount: Uint128::from_str("292968225").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1xvunup2m747gu6yr5qpmpehal5tr654cgv42q4".to_string(),
            amount: Uint128::from_str("7872").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron14pk90lt7ecrglkr6fzkxzuxzuzcz5celhly53z".to_string(),
            amount: Uint128::from_str("45162990").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1aewp32snk3mapzzlfxc97xk64yhvduh8lxhxfs".to_string(),
            amount: Uint128::from_str("5459912").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1k9e3vvjwkuvmprf0808lzqanaq4s69fh0e2y8s".to_string(),
            amount: Uint128::from_str("28852719").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1xkn622n8u82uxymvjnaul478ywqd96ah2ewju8".to_string(),
            amount: Uint128::from_str("6707573").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1rxw8wng2cn0hkvq3yjc7nrgc7rfdffjafuq0vs".to_string(),
            amount: Uint128::from_str("41575782").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1qnm7vjlp06set0r9dxmlxlssz4l7l9qm89m0ml".to_string(),
            amount: Uint128::from_str("8884057").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1kkhhlxxdjk9dw0rjcx5yqzuwuhht0pwa7k5r35".to_string(),
            amount: Uint128::from_str("464296643").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1arpjwpz8eqn4dxl7drgjdslngkx84kzrm44mq7".to_string(),
            amount: Uint128::from_str("13362984").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1nnsmwlg0krhcjr3alt9e72pwlyt6fa4xg69sxd".to_string(),
            amount: Uint128::from_str("1629948").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron15fhr3305rajmmt8g48h9hnnnxt9qm4az2jwh0j".to_string(),
            amount: Uint128::from_str("1816497669").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron13rez78f88wymvr954g4g8xr9kcqrlv33q8d2a8".to_string(),
            amount: Uint128::from_str("514795856").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron18m26lkjly2hkck25t7sdsrnu72x0g6gxujn99s".to_string(),
            amount: Uint128::from_str("127934141").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron19f7g0r7ptvxdns5456h3ypqgkp6gaa4kwzqtvw".to_string(),
            amount: Uint128::from_str("2124988").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1zkm7azc55ffvr82hct083t9gxt6qdyz67lew0u".to_string(),
            amount: Uint128::from_str("249567").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1tk76yp96uztyvrupeqxq0h8uger6nttpl0zsek".to_string(),
            amount: Uint128::from_str("5145223").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron10lnxkvncgdxqv8z8aee9pvyf9h8ddcmlj2uh77".to_string(),
            amount: Uint128::from_str("4350615").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1mzemvvz765vh3a5t6qxe88c2k2ts3x9zcgqxuk".to_string(),
            amount: Uint128::from_str("11982544").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1k0nhmllmgvzvf3m2qvcd8h3qlwxzpl3tlu2rar".to_string(),
            amount: Uint128::from_str("6843915").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1q4g308ya3e0whz6n3ecuz69adgaeznct3y0qv4".to_string(),
            amount: Uint128::from_str("1174275374").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1nfydcr6kp5tf69emnrhtaxwh7cmrxuzxew8yyv".to_string(),
            amount: Uint128::from_str("2935304").unwrap(),
        },
    ];
    let need_to_claim: Vec<UserAdjustRewards> = vec![
        UserAdjustRewards {
            user: "neutron1ez4ev32pznjc6lp7yakdc7yj2quqhjrf3y9dhx".to_string(),
            amount: Uint128::from_str("11566236").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1au342rrkx42rzz32k3tj23e9h76fxqfwzss8s6".to_string(),
            amount: Uint128::from_str("14").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1r5m978tlcuuuvmaw6xqdj2k2qpv5myxwxpssvt".to_string(),
            amount: Uint128::from_str("560").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1yxqajr79ldggu5w9tvd0tk8z5ux5qkn70fst67".to_string(),
            amount: Uint128::from_str("256").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron15hy7ky6hs7aeg2wpxhkwqs7tzzvjt4qnpn8m3g".to_string(),
            amount: Uint128::from_str("612622203").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1up4w90c0zm672mh0euch77avtygun0yfq0vffx".to_string(),
            amount: Uint128::from_str("13671998").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron12uq952unhc2m5l58lrvm9nnu7a3ag873cmunuq".to_string(),
            amount: Uint128::from_str("7648").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron15u4qt7vp53h8jsm3caenyxskmp3ma3cg2mpddr".to_string(),
            amount: Uint128::from_str("3348182306").unwrap(),
        },
        UserAdjustRewards {
            user: "neutron1pr8sqns2srqktd0cq35tn8pvup9vaf5vrcpmlc".to_string(),
            amount: Uint128::from_str("743389284").unwrap(),
        },
    ];

    for rewards in need_to_claim.iter() {
        let mut eclipastro_total_staked = Uint128::zero();
        for lock_cfg in cfg.lock_configs.iter() {
            let user_staking = SINGLE_USER_LOCKUP_INFO
                .load(deps.storage, (&rewards.user, lock_cfg.duration))
                .unwrap_or_default();
            eclipastro_total_staked +=
                user_staking.total_eclipastro_staked - user_staking.total_eclipastro_withdrawed;
        }
        for lock_cfg in cfg.lock_configs.iter() {
            let mut user_staking = SINGLE_USER_LOCKUP_INFO
                .load(deps.storage, (&rewards.user, lock_cfg.duration))
                .unwrap_or_default();
            let user_eclipastro =
                user_staking.total_eclipastro_staked - user_staking.total_eclipastro_withdrawed;
            if user_eclipastro.is_zero() {
                continue;
            }
            user_staking.unclaimed_rewards.eclipastro +=
                rewards.amount * user_eclipastro / eclipastro_total_staked;
            SINGLE_USER_LOCKUP_INFO
                .save(
                    deps.storage,
                    (&rewards.user, lock_cfg.duration),
                    &user_staking,
                )
                .unwrap();
        }
    }

    for rewards in more_claimed_rewards.iter() {
        let mut eclipastro_total_staked = Uint128::zero();
        let mut total_unclaimed_rewards = Uint128::zero();
        for lock_cfg in cfg.lock_configs.iter() {
            let user_staking = SINGLE_USER_LOCKUP_INFO
                .load(deps.storage, (&rewards.user, lock_cfg.duration))
                .unwrap();
            total_unclaimed_rewards += user_staking.unclaimed_rewards.eclipastro;
            eclipastro_total_staked +=
                user_staking.total_eclipastro_staked - user_staking.total_eclipastro_withdrawed;
        }
        let remained_unclaimed_rewards = if total_unclaimed_rewards.ge(&rewards.amount) {
            total_unclaimed_rewards - rewards.amount
        } else {
            Uint128::zero()
        };
        let more_claimed = if total_unclaimed_rewards.lt(&rewards.amount) {
            Uint128::zero()
        } else {
            rewards.amount - total_unclaimed_rewards
        };
        for lock_cfg in cfg.lock_configs.iter() {
            let mut user_staking = SINGLE_USER_LOCKUP_INFO
                .load(deps.storage, (&rewards.user, lock_cfg.duration))
                .unwrap_or_default();
            let user_eclipastro =
                user_staking.total_eclipastro_staked - user_staking.total_eclipastro_withdrawed;
            if user_eclipastro.is_zero() {
                continue;
            }
            if !remained_unclaimed_rewards.is_zero() {
                user_staking.unclaimed_rewards.eclipastro =
                    remained_unclaimed_rewards * user_eclipastro / eclipastro_total_staked;
            } else {
                user_staking.unclaimed_rewards.eclipastro = Uint128::zero();
                let amount = more_claimed * user_eclipastro / eclipastro_total_staked;
                ADJUST_REWARDS
                    .save(
                        deps.storage,
                        &(rewards.user.clone(), lock_cfg.duration),
                        &amount,
                    )
                    .unwrap();
            }
            SINGLE_USER_LOCKUP_INFO
                .save(
                    deps.storage,
                    (&rewards.user, lock_cfg.duration),
                    &user_staking,
                )
                .unwrap();
        }
    }

    Ok(Response::new().add_attribute("new_contract_version", CONTRACT_VERSION))
}
