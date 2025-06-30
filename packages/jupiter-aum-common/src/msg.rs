use crate::types::SolanaData;
use consensus::consensus::{OracleData, Round};
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    /// Delta in percent per million (ppm), for which two values are considered equal
    pub data_delta_ppm: u64,
    /// Consensus round length in seconds
    pub round_length: u64,
    /// Initial valid period for data in seconds.
    pub valid_period: u64,
    /// List of custody asset denoms required for consensus
    pub required_custody_assets: Vec<String>,
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
        /// New valid period for data in seconds.
        valid_period: Option<u64>,
        /// New required custody asset denoms.
        required_custody_assets: Option<Vec<String>>,
    },
    /// PublishData allows a registered oracle to submit new Solana data.
    /// This message triggers the consensus check and updates `last_published_data` if consensus is reached.
    PublishData { data: OracleData<SolanaData> },
    UpdateConsensusConfig {
        /// List of allowed oracle addresses.
        oracles: Option<Vec<String>>,
        /// Threshold for consensus.
        threshold: Option<u32>,
        // TODO
        data_delta_ppm: Option<u64>,
        /// Round length in seconds
        round_length: Option<u64>,
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
    /// Returned value is a decimal integer with precision of `DECIMAL_PRECISION`
    /// Returns error if data is not valid.
    GetAUM {},
    /// GetRoundInfo returns round info that is needed for oracles to know when to publish data
    GetRoundInfo {},
}

// --- Query Responses ---

/// ConfigResponse contains the current contract configuration.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ConfigResponse {
    /// The current admin address.
    pub admin: String,
    /// The current valid period in seconds.
    pub valid_period: u64,
}

/// GetDataResponse contains the last successfully published Solana data.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct GetDataResponse {
    /// The finalized Solana data, if available.
    pub last_published_data: Option<OracleData<SolanaData>>,
}

/// GetAUMResponse contains the calculated AUM value in BTC.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct GetAUMResponse {
    /// The calculated AUM value in BTC, represented as a Uint128 with `U128_PRECISION`.
    pub aum_in_btc: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RoundInfoResponse {
    pub pending_round: Round,
    pub next_round: Round,
}

/// MigrateMsg is used for contract migration.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MigrateMsg {}
