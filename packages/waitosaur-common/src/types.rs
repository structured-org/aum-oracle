use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, SignedDecimal256};

#[cw_serde]
pub struct Config {
    pub locker: Addr,
    pub unlocker: Addr,
    pub contract: Addr,
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
