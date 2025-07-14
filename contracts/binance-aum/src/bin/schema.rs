use cosmwasm_schema::write_api;
use binance_aum_oracle_contract::msg::{InstantiateMsg, QueryMsg, ExecuteMsg};
fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    }
}