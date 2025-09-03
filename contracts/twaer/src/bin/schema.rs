use cosmwasm_schema::write_api;
use twaer_common::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    }
}
