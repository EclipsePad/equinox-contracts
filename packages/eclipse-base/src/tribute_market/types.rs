use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cw_serde]
pub struct Config {
    pub astroport_voting_escrow: Addr,
    pub astroport_emission_controller: Addr,
}
