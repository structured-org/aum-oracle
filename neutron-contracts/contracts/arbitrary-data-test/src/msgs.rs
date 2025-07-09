use cosmwasm_schema::{cw_serde, QueryResponses};

#[allow(unused_imports)]
use crate::state::TestData;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(TestData)]
    GetTestData {},
}
