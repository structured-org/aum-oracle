use crate::state::Config;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    /// Owner of the contract authorized to perform privileged operations.
    pub owner: String,
    /// A list of AUM oracle instances from where the contract gets individual AUMs.
    pub aum_oracles: Vec<String>,
    /// The tokenfactory denom of the maxBTC token.
    pub maxbtc_denom: String,
    /// Time window in seconds for TWA calculation (e.g., 86400 for 24 hours).
    pub twa_window_seconds: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Updates the contract's configuration parameters. Only callable by the owner.
    /// All fields are optional, allowing partial updates.
    UpdateConfig { new_config: UpdateConfig },

    /// Calculates an instant exchange rate based on AUM and supply data, then records it in the
    /// exchange rate history for TWA calculation purposes. This should be called frequently to
    /// to maintain accurate time-weighted average data. The recorded rates are used internally
    /// for TWA calculations but do not directly affect the published rate.
    /// Only callable by the owner.
    RecordEr {},

    /// Calculates and publishes the official Time-Weighted Average Exchange Rate that is exposed
    /// via GetTwaExchangeRate queries. The calculation uses the exchange rate history populated
    /// by RecordEr calls to compute the TWA over the configured time window.
    /// Only callable by the owner.
    PublishTwaer {},
}

#[cw_serde]
pub struct UpdateConfig {
    /// New owner address.
    pub owner: Option<String>,
    /// A new list of AUM oracle instances from where the contract receives individual AUMs.
    pub aum_oracles: Option<Vec<String>>,
    /// New maxBTC denom.
    pub maxbtc_denom: Option<String>,
    /// New time window in seconds for TWA calculation.
    pub twa_window_seconds: Option<u64>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    /// Returns contract's configuration.
    GetConfig {},

    #[returns(GetAumResponse)]
    /// Returns the latest total AUM (in Bitcoin) reported by oracles.
    GetAum {},

    #[returns(GetTwaerResponse)]
    /// Returns the Time-Weighted Average Exchange Rate published by the contract. The rate is only
    /// updated when PublishTwaer is explicitly called, providing predictable and authorized rate
    /// changes.
    GetTwaer {},

    #[returns(Decimal)]
    /// PredictTwaer calculates and returns the current Time-Weighted Average Exchange Rate based
    /// on all recorded exchange rate history from RecordEr calls. This provides a real-time view
    /// of what the TWA ER would be if PublishTwaer were called at this moment.
    PredictTwaer {},
}

#[cw_serde]
pub struct GetAumResponse {
    /// The total BTC AUM reported by oracles.
    /// The value is in micro-Bitcoin (uwBTC) = 1wBTC = 100000000 uwBTC
    pub aum_in_btc: Uint128,
}

#[cw_serde]
pub struct GetTwaerResponse {
    /// The Time-Weighted Average Exchange Rate.
    pub twaer: Decimal,

    /// The timestamp when the TWAER was published.
    pub published_at: u64,
}
