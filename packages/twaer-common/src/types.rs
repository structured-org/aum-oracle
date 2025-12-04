use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};

#[cw_serde]
pub struct Config {
    /// The address allowed to record the exchange rate.
    pub recorder: Addr,
    /// The address allowed to publish the TWAER.
    pub publisher: Addr,
    /// A list of AUM oracle instances from where the contract gets individual AUMs.
    pub aum_oracles: Vec<Addr>,
    /// The address of the maxBTC core contract.
    pub maxbtc_core_contract: Addr,
    /// Time window in seconds for TWA calculation (e.g., 86400 for 24 hours).
    pub twa_window_seconds: u64,
    /// The minimal number of seconds required to pass between sequential TWAER publications.
    pub twaer_immutability_seconds: u64,
    /// Maximum allowed difference between newly calculated TWAER and the previous one,
    /// expressed in parts per million (PPM). For example, 10000 PPM = 1%.
    /// If set to None, the check is disabled.
    pub twaer_diff_ppm: Option<u128>,
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

#[cw_serde]
pub struct MaxBTCCoreConfig {
    pub deposit_denom: String,
    pub maxbtc_denom: String,
}
