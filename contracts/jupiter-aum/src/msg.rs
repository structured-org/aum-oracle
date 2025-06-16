use cosmwasm_std::{Timestamp, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{CustodyAsset, PublishedData, SolanaData};

/// InstantiateMsg defines the message used to initialize the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    /// The address that will be the contract's admin.
    pub admin: String,
    /// Initial list of oracle addresses.
    pub oracles: Vec<String>,
    /// Initial threshold for consensus.
    pub threshold: u32,
    /// Initial value for extract_period (data extraction from each N-th Solana slot).
    pub extract_period: u64,
    /// Initial valid period for data in seconds.
    pub valid_period: u64,
}

/// ExecuteMsg defines the messages that can be executed on the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// UpdateConfig updates the contract's configuration parameters.
    /// Only callable by the admin. All fields are optional, allowing partial updates.
    UpdateConfig {
        /// New admin address.
        admin: Option<String>,
        /// New list of oracle addresses.
        oracles: Option<Vec<String>>,
        /// New threshold for consensus.
        threshold: Option<u32>,
        /// New value for extract_period.
        extract_period: Option<u64>,
        /// New valid period for data in seconds.
        valid_period: Option<u64>,
    },
    /// PublishData allows a registered oracle to submit new Solana data.
    /// This message triggers the consensus check and updates `last_published_data` if consensus is reached.
    PublishData {
        // TODO: descriptions
        timestamp: Timestamp,
        // TODO: descriptions
        custody_assets: Vec<CustodyAsset>,
        /// The Solana slot number of the published data.
        slot: u64,
        /// Jupiter's Assets Under Management value in USD.
        aum_usd: Uint128,
        /// The total supply of JLP tokens.
        jlp_total_supply: Uint128,
        /// The balance of JLP tokens held by the strategy.
        strategy_jlp_balance: Uint128,
    },
}

/// QueryMsg defines the messages that can be queried from the contract to get information.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Config returns the current contract configuration.
    Config {},
    /// GetData returns the last Solana data that was successfully published (consensus has been reached).
    GetData {},
    /// GetAUM calculates and returns the current Jupiter AUM value represented in BTC.
    /// Returns error if data is not valid.
    GetAUM {},
}

// --- Query Responses ---

/// ConfigResponse contains the current contract configuration.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ConfigResponse {
    /// The current admin address.
    pub admin: String,
    /// The current list of oracle addresses.
    pub oracles: Vec<String>,
    /// The current threshold for consensus.
    pub threshold: u32,
    /// The current value of extract_period.
    pub extract_period: u64,
    /// The current valid period in seconds.
    pub valid_period: u64,
}

/// GetDataResponse contains the last successfully published Solana data.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct GetDataResponse {
    /// The finalized Solana data, if available.
    pub data: Option<SolanaData>,
}

/// PublishedDataForSlotResponse contains a list of all pending data published by oracles for a given slot.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PublishedDataForSlotResponse {
    /// A vector of `PublishedData` entries for the queried slot.
    pub data: PublishedData,
}

/// GetAUMResponse contains the calculated AUM value in BTC.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct GetAUMResponse {
    /// The calculated AUM value in BTC, represented as a Uint128 with `U128_PRECISION`.
    pub aum_in_btc: Uint128,
}

/// MigrateMsg is used for contract migration.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MigrateMsg {}

// --- Slinky Oracle Common Message (Example, would ideally be in a separate crate) ---

/// SlinkyQueryMsg defines the query messages expected by a Slinky price oracle contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SlinkyQueryMsg {
    /// GetPrice queries the price of a specific asset.
    GetPrice {
        /// The symbol of the asset to query (e.g., "BTC", "ETH").
        asset: String,
    },
}

/// SlinkyPriceResponse defines the response structure for a Slinky price query.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct SlinkyPriceResponse {
    /// The price of the asset, typically represented as a Uint128 where
    /// 1 unit (e.g., 1 USD) equals `U128_PRECISION` (1,000,000).
    pub price: Uint128,
}
