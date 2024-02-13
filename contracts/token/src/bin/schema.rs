use cosmwasm_schema::write_api;

use cw20_base::msg::{ExecuteMsg, QueryMsg};
use token::token::InstantiateMsg;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
    }
}
