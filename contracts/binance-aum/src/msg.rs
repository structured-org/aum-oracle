use crate::state::BinanceData;
use consensus::consensus::{OracleData, Round};
use cosmwasm_std::SignedDecimal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {
    /// Owner of the contract
    pub owner: String,
    /// A list of oracles allowed to publish data
    pub oracles: Vec<String>,
    /// Initial threshold for consensus.
    pub threshold: u32,
    /// Delta in percent per million (ppm), for which two values are considered equal
    pub data_delta_ppm: u64,
    /// Consensus round length in seconds
    pub round_length: u64,
    /// Initial valid period for data in seconds
    pub consensus_data_valid_period: u64,
    /// Initial valid period for data in blocks
    pub price_data_valid_period: u64,
    /// Required binance positions that oracles must provide
    pub required_binance_positions: Vec<String>,
    /// Required binance spot assets that oracles must provide
    pub required_binance_spot_assets: Vec<String>,
    /// Price oracle contract address
    pub price_oracle_contract: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// PublishData allows a registered oracle to submit new Binance data.
    /// This message triggers the consensus check and updates `last_published_data` if consensus is reached.
    PublishData { new_data: BinanceData },
    /// UpdateConfig updates the contract's configuration parameters.
    /// Only callable by the admin. All fields are optional, allowing partial updates.
    UpdateConfig { new_config: UpdateConfig },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UpdateConfig {
    /// New owner address
    pub owner: Option<String>,
    /// New validity period for the consensus (in seconds)
    pub consensus_data_valid_period: Option<u64>,
    /// New validity period for price oracle (in blocks)
    pub price_data_valid_period: Option<u64>,
    /// New required binance positions
    pub required_binance_positions: Option<Vec<String>>,
    /// New required binance spot assets
    pub required_binance_spot_assets: Option<Vec<String>>,
    /// New price oracle contract address
    pub price_oracle_contract: Option<String>,

    /// A new list of oracles that are allowed to publish data
    pub oracles: Option<Vec<String>>,
    /// A new threshold value for the consensus
    pub threshold: Option<u32>,
    /// A new data delta for the consensus
    pub data_delta_ppm: Option<u64>,
    /// A new rounds length (in seconds)
    pub round_length: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub enum QueryMsg {
    /// Returns the latest published data
    GetData {},
    /// Returns the latest total AUM in Binance reported by oracles
    GetAum {},
    /// Returns current round info
    GetRoundInfo {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RoundInfoResponse {
    /// Currently pending round
    pub pending_round: Round,
    /// The next round
    pub next_round: Round,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GetDataResponse {
    /// The latest published data (can be null if there was no consensus reached)
    pub last_published_data: Option<OracleData<BinanceData>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GetAumResponse {
    /// The latest AUM in Binance reported by oracles
    pub aum_in_btc: SignedDecimal,
}
