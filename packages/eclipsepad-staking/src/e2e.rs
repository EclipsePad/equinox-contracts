use cosmwasm_std::{Addr, Storage, Uint128};

use eclipse_base::{
    error::ContractError,
    staking::{
        state::{
            CONFIG, LOCKER_INFO, LOCKING_ESSENCE, LOCK_STATES, STAKER_INFO, STAKE_STATE,
            STAKING_ESSENCE_COMPONENTS, TOTAL_LOCKING_ESSENCE, TOTAL_STAKING_ESSENCE_COMPONENTS,
        },
        types::{Config, LockerInfo, StakerInfo, State, Vault},
    },
};

use crate::math;

// for e2e testing and gas profiling

pub fn add_users(storage: &mut dyn Storage, block_time: u64) -> Result<(), ContractError> {
    const VAULTS_LIMIT: u64 = 25; // requires 24M gas while limit is 25M
    const FUNDS: u128 = 1_000_000_000;
    const USER_AMOUNT: u64 = 98;

    let user_list = get_address_list(USER_AMOUNT as usize);
    let mut total_components = (Uint128::zero(), Uint128::zero());
    let mut total_locking_essence = Uint128::zero();

    for user in user_list {
        let staking_vaults = get_vaults(block_time, FUNDS, 0, VAULTS_LIMIT, 3 * VAULTS_LIMIT);
        let components = math::v3::calc_components_from_staking_vaults(&staking_vaults);
        total_components = (
            total_components.0 + components.0,
            total_components.1 + components.1,
        );

        STAKER_INFO.save(
            storage,
            &user,
            &StakerInfo {
                vaults: staking_vaults,
            },
        )?;

        STAKING_ESSENCE_COMPONENTS.save(storage, &user, &components)?;

        let tier_0_locking_vaults = get_vaults(
            block_time,
            FUNDS,
            VAULTS_LIMIT,
            2 * VAULTS_LIMIT,
            3 * VAULTS_LIMIT,
        );
        let tier_4_locking_vaults = get_vaults(
            block_time,
            FUNDS,
            2 * VAULTS_LIMIT,
            3 * VAULTS_LIMIT,
            3 * VAULTS_LIMIT,
        );

        let Config {
            lock_schedule,
            seconds_per_essence,
            ..
        } = CONFIG.load(storage)?;

        let (locking_period, _) = lock_schedule[0];
        let mut locking_essence = math::v3::calc_locking_essence_per_tier(
            &tier_0_locking_vaults,
            locking_period,
            seconds_per_essence,
        );

        let (locking_period, _) = lock_schedule[4];
        locking_essence += math::v3::calc_locking_essence_per_tier(
            &tier_4_locking_vaults,
            locking_period,
            seconds_per_essence,
        );
        total_locking_essence += locking_essence;

        LOCKER_INFO.save(
            storage,
            &user,
            &vec![
                LockerInfo {
                    lock_tier: 0,
                    vaults: tier_0_locking_vaults,
                },
                LockerInfo {
                    lock_tier: 4,
                    vaults: tier_4_locking_vaults,
                },
            ],
        )?;

        LOCKING_ESSENCE.save(storage, &user, &locking_essence)?;
    }

    TOTAL_STAKING_ESSENCE_COMPONENTS.save(storage, &total_components)?;
    TOTAL_LOCKING_ESSENCE.save(storage, &total_locking_essence)?;

    let total_bond_amount = Uint128::new((VAULTS_LIMIT as u128) * FUNDS);

    STAKE_STATE.save(
        storage,
        &State {
            distributed_rewards_per_tier: 0,
            total_bond_amount,
        },
    )?;

    LOCK_STATES.save(
        storage,
        &vec![
            State {
                distributed_rewards_per_tier: 0,
                total_bond_amount,
            },
            State {
                distributed_rewards_per_tier: 0,
                total_bond_amount: Uint128::zero(),
            },
            State {
                distributed_rewards_per_tier: 0,
                total_bond_amount: Uint128::zero(),
            },
            State {
                distributed_rewards_per_tier: 0,
                total_bond_amount: Uint128::zero(),
            },
            State {
                distributed_rewards_per_tier: 0,
                total_bond_amount,
            },
        ],
    )?;

    Ok(())
}

fn get_vaults(block_time: u64, funds: u128, from: u64, to: u64, offset: u64) -> Vec<Vault> {
    (from..to)
        .map(|x| Vault {
            amount: Uint128::new(funds),
            creation_date: block_time - offset + x,
            claim_date: block_time - offset + x,
            accumulated_rewards: Uint128::zero(),
        })
        .collect()
}

// 100 addresses max
fn get_address_list(amount: usize) -> Vec<Addr> {
    vec![
        "stars19y7a38cnf9d8cr264wz5d6dmrsgsmplxkf4lyw", // admin
        "stars1q5u23ppwrf7jvns33u9rm2xu8u37wyy699v2nx", // alice
        "stars1nngvah0u8wltpjrnl87quwacde05q8ys7juv3u", // bob
        "stars1r84jhpjt7tfty4hzpsh9wqlkqnjlv8z8gx449k", // owner
        "stars105yqjjdgl00nzwyj9aua98zgetdn4qyh6vaw4s",
        "stars10datnnlcjmrdl37ka0g4u83chvxpfafmr3459c",
        "stars10fwlvt749384x2278gylk50kxvr2lgc5xewmak",
        "stars10n5dcumwvsaf6ma3gup0msp5w9hvu70vg9qtc9",
        "stars10s944fgumfqtw384864xq0zl69z9gg2dxg82ct",
        "stars126vzpl58vyr40z2s4ncu74ddryn7zyud5due0e",
        "stars12rr5pw9rgm09g40ghp7n0983qljegylh6hvsj6",
        "stars12v0ndxalrq32k2kfs2rhn6p7rpu9pq0vltjqe5",
        "stars133xakkrfksq39wxy575unve2nyehg5npka2vsp",
        "stars1358fvs0alm355dkltxdya5jk77tuqca8uaalw7",
        "stars138pl8udw69f9x5xrhyt4atk83u2nyggugejrah",
        "stars13en8cc3pyp3xunt9jqffpv57405n9pgelt5dfu",
        "stars13h59ek02vmpra6zlcp47068pgxujaxje6zsvep",
        "stars13hjvwgg4uu04dg0tk892yjkqle9k3cra5r79s2",
        "stars13kup3wy4eszpm306ycvc4j5y05xms9p7yh0f7t",
        "stars13s5czcpgqem8gcya4qn74mj6yut4urudys6v5y",
        "stars13u0swjkxspchjzrftaa5v60rcagj35lqdegjlh",
        "stars13uhl4tp3flplqpfnwjhz45req2wj2fjzpdnjla",
        "stars13wsx8rujp6e7zunkrqypy490ccznfwk0rgkgzz",
        "stars143v8d9qzpsj46m30zsj7m2677x77604t8ctauf",
        "stars14640tst2rx45nxg3evqwlzuaestnnhm8fn0u6w",
        "stars14exvl768pree88sthmp9cp3za7z2cha24m9gs9",
        "stars14guqecmmlaayy6l7uajeqw3h09s7wlh65ecvfx",
        "stars14hj3ecw0wfaylqm4uakgnq48hvzwqxfdqk33l9",
        "stars14hkefuvrpgvg54k2w5h932s8pth4p2pwpka3yz",
        "stars14jlmpujasa5499wevxj0lu879cla4mjgd28lyz",
        "stars14jvp0nqap4srycs25nupnzq2qyd7xgdypyxn24",
        "stars14lhvu9zfa7n844zypvt33eck0aukdsa96427rk",
        "stars14upttxsxwpucpmacqzfsrfu4u6pqvdl54fr8y5",
        "stars150pycew3s0kk08ftv8k2wyvwd765affku5ys6z",
        "stars150wnq5vl9e58lgsvtwjrdeca6ny4lz88ssp2yf",
        "stars155cnljtwkm959yrptvqxu8z2zuetcxr4jfajwu",
        "stars156rx7ufuq73dzqyp929gqlfsjzm4kpnam5tcfm",
        "stars159n5w6gcz4vj9ntg5zqumyek52e2x4gum60khe",
        "stars15r4jppk780ucey6rdk29kz6j88m5qgsevtnlqx",
        "stars15ucydnzg76mlt6kc3j4qmqtxh733k38382e0xz",
        "stars15zwh39hdsv5vh6cc8lymuvt625eszgct2x79un",
        "stars165s3zw3vaytnjvfv390f9f876u79zv494kne7y",
        "stars166z7xyph4kfr8fwzzt4ljt2pyy6ql9tpcv9sp5",
        "stars16dmtz4c9p8aul5n7555lcw6tsc2djss6mpyxhy",
        "stars16h82h7aklrk73fhf6k6h8qgnc365j5hd4prvxh",
        "stars16hmvxnw99ay5r8qlz5njxk5nl7g3msjllktgm3",
        "stars16m85hj8d7k65txfzppcwfm67rx27c48jj95n48",
        "stars16m8mxwdea9t9vsatzvawmdm3z33v3hvazwm7aj",
        "stars173pn5d5lct5xkpx9l85ww8p97c8ssge3hex3ch",
        "stars173sxh0zms7d8nwwr96ahaaw657u3dj84vqsz7s",
        "stars175n75x0r38ymqrsm44hvjh4j3zk9usvh2jqsce",
        "stars17hgt963z420vwhdmtfc9gxnmtjgvqc37pu20pe",
        "stars17hmq0ku59g57c42mw78lcn6srfve0pwx78h2ls",
        "stars17hnlaav8d2reekqtn7vfrhfhskejvnq8n2fkdf",
        "stars17kp85f2vgpcnxcqe7v0hnu9g52sgmrmnvzwlkt",
        "stars17su0549xmtty7hfgf72dwh0nmp5rk9m5zr5gk2",
        "stars17t27p0y6fk77g5v8tuypc7d2qjexszcd32xv8f",
        "stars17wz08283rjqkra2qsgfrt8tf4pt5p86q3ht5w7",
        "stars17ypm0zx6tfm5zf49kpj3dpyvm6xz9vff9efzr0",
        "stars185yv4ka7ymn3f6meq7k3g42nxrgfeykjkua59c",
        "stars188upw04lt4w73w32gql9wf49u9zukwagepywkc",
        "stars18czqt6thyxeu9uapqwan09cgnzh2wgrhl83cse",
        "stars18fdu02ufsmak66z4qnm6u6es3ztkhdze4ewapl",
        "stars18jnqnds9xg085c3wr730ck4fvr6c0gu8h6yxl5",
        "stars18qk3mqm2yhshhhr62w39jpycdfnh607yayt5mu",
        "stars18s7s54yv2032mu4u5tnmmexj2lum7gjdkw9de9",
        "stars18u0a2rqm306nd965ehdufxpytxay7ckz2hw2pj",
        "stars18wpyskpyq3dy4d9u7z8zwz2cnxqntrg3mascpk",
        "stars1904h4pkdezcm7xcxrcum7cckr4wel3kad8m9r0",
        "stars195x63zceqj9jylnp5luahe7lvh3fvukf8eqv6d",
        "stars195z08s2qj0em8pzyyaqm5l29p0w9vrdesc64sg",
        "stars196zr4kee6ee5a3mgmz2julhzkx0j87ws0jqu2u",
        "stars197azlmrp8wvz4446jfnxu605snegznl4f2wvt3",
        "stars19aslyc8mtctyq037m7j6vr524kfrzp8dpal43t",
        "stars19fhgzel7809gr9kvhr4yzzl6vk5u0u4cup6n45",
        "stars19ulgjqpj05hfm8e2ey6tan4z2pl57dhk29kl0h",
        "stars19y0q6r3stt3k2t9aye2tnyx6kpyq5gnm4elhph",
        "stars1a02gxhfxr49c87secnw98sscps02f29ae89qvt",
        "stars1a9zmhn5ycxtz6h0x3f50cy2tdcve335d03tttf",
        "stars1apx87m4378vpnncglx8m6gklafhtnjhhr96027",
        "stars1aqffst0qq3y9nw7jr5z3h2xp6ud6sjkxykrlw4",
        "stars1arnhwy03z6r6w7mzpwf64rq7d8r9ddfupksav8",
        "stars1atumla4vkm62adgsty9tj23crxufh3aweuvhjy",
        "stars1c347zr3z9ck7kwf9csxnn5vj5d06xcngxpcpqu",
        "stars1c3yt57kz5mstnc2n08n47phjumn57ds0znapj8",
        "stars1c7l3lvqp53fdg4ux8yvemsf540s044ne29gwk4",
        "stars1cc90ecv3tx42750p72mkwkc8dph3c5ff44dk48",
        "stars1cg5p8qtl7xn7tjzm6dejqwl7l07udsxpdpt3x7",
        "stars1cr7lxv4srcesuw8uvm07n9jdfmnm33g2p07tnx",
        "stars1cvg5dsprer8c7xl0cl966qq0emnjthtwwh4994",
        "stars1cwecw85ecn5zjppfsm59ddsa7gqa3pnfl9c7d0",
        "stars1cz9fm34867lr43pmkq57zmwfj0yf330pc8n6ma",
        "stars1czwf7pq29sj4xmw2swz3dr8yfx7ptkv9ys3x29",
        "stars1d2ehppn5cq6k6e34zmsl8zuehdg40j4rc4h6xw",
        "stars1d74f6c7rgyyym57530rc2tt88fnl6wpdcpz6ta",
        "stars1d7sw0t3nyajddyjtvwargqkypqx0yumdarq302",
        "stars1dazjg75s45rt6kutxfgjcpsvf2kulqz42ktafs",
        "stars1df0eyj4kpazcev84rwrd2qfdwxem5cjhgnn8cy",
        "stars1dja8plw7swsrlsckjfwqaxmlm3vdceve7294q4",
        "stars1dz90gzearja6nnnzf32xexx5cl4g4xvp3g8xss",
    ]
    .into_iter()
    .take(amount)
    .map(Addr::unchecked)
    .collect()
}
