use crate::state::{Config, ExchangeRateDataPoint};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    /// Owner of the contract
    pub owner: String,
    /// A list of AUM oracle instances from where the contract need to get individual AUMs and sum them up
    pub oracles: Vec<String>,
    /// The tokenfactory denom of the maxBTC token.
    pub maxbtc_denom: String,
    /// Time window in seconds for TWA calculation (e.g., 86400 for 24 hours).
    pub twa_window_seconds: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// UpdateConfig updates the contract's configuration parameters. Only callable by the admin.
    /// All fields are optional, allowing partial updates.
    UpdateConfig { new_config: UpdateConfig },

    /// StoreInstantExchangeRate calculates and stores instant exchange rate based on the current
    /// circumstances. Instant exchange rates are used for calculating the TWA exchange rate.
    StoreInstantExchangeRate {},

    /// UpdateTwaExchangeRate updates the TWA exchange rate the contract returns.
    UpdateTwaExchangeRate {},
}

#[cw_serde]
pub struct UpdateConfig {
    /// New owner address.
    pub owner: Option<String>,
    /// A new list of AUM oracle instances from where the contract receives individual AUMs.
    pub oracles: Option<Vec<String>>,
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

    #[returns(Decimal)]
    /// Returns the current TWA exchange rate calculated by the contract.
    GetTwaExchangeRate {},

    #[returns(Decimal)]
    /// PredictTwaExchangeRate predicts the TWA exchange rate based on the stored instant exchange
    /// rates.
    PredictTwaExchangeRate {},

    #[returns(GetHistoricalDataResponse)]
    /// Returns historical exchange rate data points within the TWA window.
    GetHistoricalData {
        /// Optional limit on the number of data points to return (default: 100).
        limit: Option<u32>,
    },

    #[returns(u32)]
    /// Returns the count of stored exchange rate data points within the TWA window.
    GetDataPointCount {},
}

#[cw_serde]
pub struct GetAumResponse {
    /// The total BTC AUM reported by oracles.
    /// The value is in micro-Bitcoin (uwBTC) = 1wBTC = 100000000 uwBTC
    pub aum_in_btc: Uint128,
}

#[cw_serde]
pub struct GetHistoricalDataResponse {
    /// Historical exchange rate data points within the TWA window.
    pub data_points: Vec<ExchangeRateDataPoint>,
    /// Total count of data points in the window.
    pub total_count: u32,
}
