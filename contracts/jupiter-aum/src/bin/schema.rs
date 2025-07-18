use cosmwasm_schema::write_api;
use jupiter_aum_common::msg::{InstantiateMsg, QueryMsg, ExecuteMsg};
fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    }
}
