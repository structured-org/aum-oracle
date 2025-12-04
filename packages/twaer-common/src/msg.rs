use crate::types::Config;
use aum_receiver_common::types::GetAumResponse;
use cosmwasm_schema::serde::{Deserialize, Deserializer};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_serde]
pub struct InstantiateMsg {
    /// Owner of the contract authorized to perform privileged operations.
    pub owner: String,
    /// The address allowed to record the exchange rate.
    pub recorder: String,
    /// The address allowed to publish the TWAER.
    pub publisher: String,
    /// A list of AUM oracle instances from where the contract gets individual AUMs.
    pub aum_oracles: Vec<String>,
    /// Time window in seconds for TWA calculation (e.g., 86400 for 24 hours).
    pub twa_window_seconds: u64,
    /// The minimal number of seconds required to pass between sequential TWAER publications.
    pub twaer_immutability_seconds: u64,
    /// The address of the maxBTC core contract.
    pub maxbtc_core_contract: String,

    /// The mocked maxBTC supply used instead of the real supply before the token is minted. To
    /// turn the real token supply usage on, the owner must set the maxbtc_denom in the config.
    /// If the supply is zero, instant exchange rate calculations return Decimal::one().
    pub mocked_maxbtc_supply: Uint128,

    /// Maximum allowed difference between newly calculated TWAER and the previous one,
    /// expressed in parts per million (PPM). For example, 10000 PPM = 1%.
    /// If set to None, the check is disabled.
    pub twaer_diff_ppm: Option<u128>,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Updates the contract's configuration parameters. Only callable by the owner.
    /// All fields are optional, allowing partial updates.
    UpdateConfig { new_config: UpdateConfig },

    /// Calculates an instant exchange rate based on AUM and supply data, then records it in the
    /// exchange rate history for TWA calculation purposes. This should be called frequently to
    /// to maintain accurate time-weighted average data. The recorded rates are used internally
    /// for TWA calculations but do not directly affect the published rate.
    /// Only callable by the owner or recorder.
    RecordEr {},

    /// Calculates and publishes the official Time-Weighted Average Exchange Rate that is exposed
    /// via GetTwaExchangeRate queries. The calculation uses the exchange rate history populated
    /// by RecordEr calls to compute the TWA over the configured time window.
    /// Only callable by the owner, publisher or recorder.
    /// Recorder can publish a new TWAER only if the new TWAER does not differ from the previous TWAER
    /// by more than the configured max (twaer_diff_ppm in the contract's config)
    PublishTwaer {},

    /// Resets the historical and aggregator values and sets the TWAER to a specific value.
    /// Only callable by the owner.
    ResetTwaerTo { value: Decimal },

    /// Sets the mocked maxBTC supply which is used if config maxbtc_denom is not set.
    /// Only callable by the owner.
    SetMockedMaxbtcSupply { value: Uint128 },

    /// Removes a specific ER datapoint from history and TWA Aggregator.
    /// Only callable by the owner.
    RemoveERDatapoint { er_timestamp: u64 },

    /// Removes the mocked supply from the contract. From that point the contract will use the real supply of maxBTC tokens
    Unmock {},
}

#[cw_serde]
pub struct UpdateConfig {
    /// New publisher address.
    pub publisher: Option<String>,
    /// New recorder address.
    pub recorder: Option<String>,
    /// A new list of AUM oracle instances from where the contract receives individual AUMs.
    pub aum_oracles: Option<Vec<String>>,
    /// New maxBTC core contract.
    pub maxbtc_core_contract: Option<String>,
    /// New time window in seconds for TWA calculation.
    pub twa_window_seconds: Option<u64>,
    /// New minimal number of seconds required to pass between sequential TWAER publications.
    pub twaer_immutability_seconds: Option<u64>,
    /// New maximum allowed difference between newly calculated TWAER and the previous one,
    /// expressed in parts per million (PPM). For example, 10000 PPM = 1%.
    /// - Missing field: None -> no change
    /// - Explicit null: Some(None) -> set to None ("twaer_diff_ppm": null)
    /// - String value: Some(Some(Addr)) -> set to String ("twaer_diff_ppm": 10000)
    #[serde(default, deserialize_with = "deserialize_nested_option")]
    pub twaer_diff_ppm: Option<Option<u128>>,
}

#[cw_ownable_query]
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

    #[returns(ErWindowInfoResponse)]
    /// Returns the information about the exchange rate history window.
    ErWindowInfo {},
}

#[cw_serde]
pub struct GetTwaerResponse {
    /// The Time-Weighted Average Exchange Rate.
    pub twaer: Decimal,

    /// The timestamp when the TWAER was published.
    pub published_at: u64,
}

#[cw_serde]
pub struct ErWindowInfoResponse {
    /// Timestamp of the oldest data point in the window.
    pub window_start: u64,
    /// Timestamp of the newest data point in the window.
    pub window_end: u64,
    /// Total number of data points in the window.
    pub total_points: u64,
    /// Data points itself
    pub data_points: Vec<(u64, Decimal)>,
}

/// MigrateMsg is used for contract migration.
#[cw_serde]
pub struct MigrateMsg {}

/// Custom deserializer for Option<Option<u128>> to distinguish between missing field and null.
/// - Missing field: None
/// - Explicit null: Some(None)
/// - u128 value: Some(Some(u128))
fn deserialize_nested_option<'de, D>(deserializer: D) -> Result<Option<Option<u128>>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}
