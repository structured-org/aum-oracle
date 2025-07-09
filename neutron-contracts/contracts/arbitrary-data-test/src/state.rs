use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct TestData {
    pub frist_value: String,
    pub second_value: String,
}
