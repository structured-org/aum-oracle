use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};
use cw_storage_plus::{Item, Map};

pub const CONFIG: Item<Config> = Item::new("config");

/// The "agreed upon" TWA exchange rate (paired with a timestamp) that is returned by GetTwaer
/// queries. This value is updated when PublishTwaer is explicitly called by the admin, providing
/// a stable, controlled value.
pub const TWAER: Item<(Decimal, u64)> = Item::new("twaer");

/// Incremental TWA calculation state. Updated every time StoreInstantExchangeRate is called.
pub const TWA_AGGREGATOR: Item<TwaAggregator> = Item::new("twa_aggregator");

/// Historical exchange rate data points, keyed by timestamp.
pub const ER_HISTORY: Map<u64, Decimal> = Map::new("er_history");

#[cw_serde]
pub struct Config {
    /// Owner of the contract.
    pub owner: Addr,
    /// A list of AUM oracle instances from where the contract gets individual AUMs.
    pub aum_oracles: Vec<Addr>,
    /// The tokenfactory denom of the maxBTC token.
    pub maxbtc_denom: String,
    /// Time window in seconds for TWA calculation (e.g., 86400 for 24 hours).
    pub twa_window_seconds: u64,
}

#[cw_serde]
pub struct TwaAggregator {
    /// Weighted sum: Σ(rate_i × duration_i)
    pub weighted_sum: Decimal,
    /// Total duration: Σ(duration_i)
    pub total_duration: u64,
    /// Current TWA: weighted_sum / total_duration
    pub current_twa: Decimal,
    /// Timestamp of the oldest data point in the window
    pub window_start: u64,
    /// Timestamp of the newest data point in the window
    pub window_end: u64,
}

impl TwaAggregator {
    pub fn from_single_point(timestamp: u64, rate: Decimal) -> Self {
        Self {
            weighted_sum: Decimal::zero(),
            total_duration: 0,
            current_twa: rate,
            window_start: timestamp,
            window_end: timestamp,
        }
    }
}
