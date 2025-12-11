use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, SignedDecimal256, Uint64};

#[cw_serde]
pub struct Config {
    pub locker: Addr,
    pub unlocker: Addr,
    pub contract: Addr,
    pub asset: String,
    // period after which AUM is considered stale (in seconds)
    pub aum_stale_period: Uint64,
}

#[cw_serde]
pub enum State {
    Locked {
        amount: SignedDecimal256,
        at_timestamp: Uint64,
    },
    Unlocked {},
}
