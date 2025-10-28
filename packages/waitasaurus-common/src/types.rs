use cosmwasm_schema::cw_serde;
use cosmwasm_std::SignedDecimal256;

#[cw_serde]
pub struct Config {
    pub locker: String,
    pub unlocker: String,
    pub contract: String,
    pub asset: String,
}

#[cw_serde]
pub enum State {
    Locked {
        amount: SignedDecimal256,
        at_timestamp: u64,
    },
    Unlocked {},
}
