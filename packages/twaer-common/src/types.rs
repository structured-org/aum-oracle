use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};

#[cw_serde]
pub struct Config {
    /// The address allowed to publish the TWAER.
    pub publisher: Addr,
    /// A list of AUM oracle instances from where the contract gets individual AUMs.
    pub aum_oracles: Vec<Addr>,
    /// The tokenfactory denom of the maxBTC token. If set to None, the MOCKED_MAXBTC_SUPPLY
    /// is used in exchange rate calculation.
    pub maxbtc_denom: Option<String>,
    /// Time window in seconds for TWA calculation (e.g., 86400 for 24 hours).
    pub twa_window_seconds: u64,
    /// The minimal number of seconds required to pass between sequential TWAER publications.
    pub twaer_immutability_seconds: u64,
}

#[cw_serde]
pub struct TwaAggregator {
    /// Weighted sum: Σ(rate_i × duration_i)
    pub weighted_sum: Decimal,
    /// Current TWA: weighted_sum / (window_end - window_start)
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
            current_twa: rate,
            window_start: timestamp,
            window_end: timestamp,
        }
    }
}
