use crate::state::{BinanceData, Config};
use consensus::consensus::{Config as ConsensusConfig, ConsensusOutcome, Round};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Int256;

#[cw_serde]
pub struct InstantiateMsg {
    /// Owner of the contract
    pub owner: String,
    /// A list of messengers allowed publishing data
    pub messengers: Vec<String>,
    /// Initial threshold for consensus.
    pub threshold: u32,
    /// Delta in percent per million (ppm), for which two values are considered equal
    pub data_delta_ppm: u64,
    /// Consensus round length in seconds
    pub round_length: u64,
    /// Initial validity period for data (that reached consensus) in the contract in seconds. If the data is too old,
    /// Binance AUM contract cannot rely on it in AUM calculations,
    /// and in that case the contract just doesn't calculate AUM and returns an error in the corresponding query.
    pub consensus_data_valid_period: u64,
    /// Initial validity period for prices from the oracle contract in blocks. If the prices are too old,
    /// Binance AUM contract cannot rely on them in AUM calculations,
    /// and in that case the contract just doesn't calculate AUM and returns an error in the corresponding query.
    pub price_data_valid_period: u64,
    /// Required binance positions that messengers must provide
    pub required_binance_positions: Vec<String>,
    /// Required binance spot assets that messengers must provide
    pub required_binance_spot_assets: Vec<String>,
    /// Price oracle contract address
    pub price_oracle_contract: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Allows a registered messenger to submit new Binance data.
    /// This message triggers the consensus check and updates `last_published_data` if consensus is reached.
    PublishData { new_data: BinanceData },
    /// Updates the contract's configuration parameters.
    /// Only callable by the owner. All fields are optional, allowing partial updates.
    UpdateConfig { new_config: UpdateConfig },
}

#[cw_serde]
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

    /// A new list of messengers that are allowed to publish data
    pub messengers: Option<Vec<String>>,
    /// A new threshold value for the consensus
    pub threshold: Option<u32>,
    /// A new data delta for the consensus
    pub data_delta_ppm: Option<u64>,
    /// A new rounds length (in seconds)
    pub round_length: Option<u64>,
}

#[cw_serde]
#[allow(clippy::enum_variant_names)]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the latest published data
    #[returns(GetDataResponse)]
    GetData {},
    /// Returns the latest total AUM in Binance reported by messengers.
    /// Returns an error if published data by messengers are too old,
    /// or token prices reported by price oracle contract are too old
    #[returns(GetAumResponse)]
    GetAum {},
    /// Returns current round info
    #[returns(RoundInfoResponse)]
    GetRoundInfo {},
    /// Returns the current configuration of the contract and it's consensus mechanism
    #[returns(GetConfigResponse)]
    GetConfig {},
}

#[cw_serde]
pub struct RoundInfoResponse {
    /// Currently pending round
    pub pending_round: Round,
    /// The next round
    pub next_round: Round,
}

#[cw_serde]
pub struct GetDataResponse {
    /// The latest published data (can be null if there was no consensus reached)
    pub last_published_data: Option<ConsensusOutcome<BinanceData>>,
}

#[cw_serde]
pub struct GetAumResponse {
    /// The latest AUM in Binance reported by messengers
    pub aum_in_btc: Int256,
    /// Represents the number of decimals that the aum_in_btc is
    /// represented in. It is used to scale the aum_in_btc to its base BTC value.
    /// E.g. `base_aum_in_btc = aum_in_btc / 10^decimals`
    pub decimals: u32,
}

#[cw_serde]
pub struct GetConfigResponse {
    /// The current config of the consensus mechanism
    pub consensus_config: ConsensusConfig,
    /// The current config of the contract itself
    pub contract_config: Config,
}
