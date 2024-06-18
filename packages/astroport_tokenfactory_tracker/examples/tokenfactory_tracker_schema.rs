use cosmwasm_schema::write_api;

use astroport_tokenfactory_tracker::msg::{InstantiateMsg, QueryMsg, SudoMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        sudo: SudoMsg,
    }
}
