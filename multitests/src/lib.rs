// #[cfg(test)]
// pub mod flexible_stake;
// #[cfg(test)]
// pub mod lockdrop;
// #[cfg(test)]
// pub mod lp_stake;
// #[cfg(test)]
// pub mod suite;
// #[cfg(test)]
// pub mod timelock_stake;
// #[cfg(test)]
// pub mod token_converter;
// #[cfg(test)]
// pub mod voter;
#[cfg(test)]
pub mod voter_new;

// https://github.com/astroport-fi/hidden_astroport_governance/tree/feat/revamped_vxastro/contracts/emissions_controller/tests/common
pub mod suite_astro {
    pub mod contracts;
    pub mod helper;
    pub mod ibc_module;
    pub mod neutron_module;
    pub mod stargate;

    pub mod extensions {
        pub mod astroport_router;
        pub mod eclipsepad_staking;
        pub mod minter;
        pub mod tribute_market_mocks;
        pub mod voter;
    }
}
