use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct MigrateMsg {
    pub version: String,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub astroport_voting_escrow: Addr,
    pub astroport_emission_controller: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    SetBribesAllocation {
        bribes_allocation: Vec<equinox_msg::voter::BribesAllocationItem>,
    },

    AllocateRewards {
        users: Vec<String>,
    },

    ClaimRewards {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<(Uint128, String)>)]
    Rewards { user: String },

    #[returns(Vec<equinox_msg::voter::BribesAllocationItem>)]
    BribesAllocation {},
}
