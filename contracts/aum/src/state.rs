use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    /// Owner of the contract.
    pub owner: Addr,
    /// A list of AUM oracle instances from where the contract need to get individual AUMs.
    pub oracles: Vec<Addr>,
    /// The tokenfactory denom of the maxBTC token.
    pub maxbtc_denom: String,
    /// Time window in seconds for TWA calculation (e.g., 86400 for 24 hours).
    pub twa_window_seconds: u64,
}

#[cw_serde]
pub struct ExchangeRateDataPoint {
    /// The exchange rate value at this point in time.
    pub rate: Decimal,
    /// UNIX timestamp in seconds when this rate was recorded.
    pub timestamp: u64,
    /// Block height when this rate was recorded.
    pub block_height: u64,
}

pub const CONFIG: Item<Config> = Item::new("config");

/// Current TWA exchange rate (calculated and stored).
pub const TWA_EXCHANGE_RATE: Item<Decimal> = Item::new("twa_exchange_rate");

/// Historical exchange rate data points, keyed by timestamp.
pub const EXCHANGE_RATE_HISTORY: Map<u64, ExchangeRateDataPoint> =
    Map::new("exchange_rate_history");
