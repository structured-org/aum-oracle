use crate::types::SolanaData;
use aum_receiver_common::types::{GetAumResponse, RoundInfoResponse};
use consensus::consensus::ConsensusOutcome;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

/// InstantiateMsg defines the message used to initialize the contract.
#[cw_serde]
pub struct InstantiateMsg {
    /// The address that will be the contract's owner.
    pub owner: String,
    /// Initial list of messenger addresses.
    pub messengers: Vec<String>,
    /// Initial threshold for consensus.
    pub threshold: u32,
    /// Delta in percent per million (ppm), for which two values are considered equal
    pub data_delta_ppm: u64,
    /// Consensus round length in seconds
    pub round_length: u64,
    /// Initial valid period for data in seconds.
    pub consensus_data_valid_period: u64,
    /// List of custody asset denoms required for consensus
    pub required_custody_assets: Vec<String>,
    /// How many blocks we consider BTC/USD price from oracle as valid.
    pub price_data_valid_period: u64,
}

/// ExecuteMsg defines the messages that can be executed on the contract.
#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// UpdateConfig updates the contract's configuration parameters.
    /// Only callable by the owner. All fields are optional, allowing partial updates.
    UpdateConfig { new_config: UpdateConfig },
    /// PublishData allows a registered messenger to submit new Solana data.
    /// This message triggers the consensus check and updates `last_published_data` if consensus is reached.
    PublishData { new_data: SolanaData },
}

#[cw_serde]
pub struct UpdateConfig {
    /// Contract config updates.
    ///
    /// New validity period for data in seconds.
    pub consensus_data_valid_period: Option<u64>,
    /// New required custody asset denoms.
    pub required_custody_assets: Option<Vec<String>>,
    /// New value for how many blocks we consider BTC/USD price from oracle as valid.
    pub price_data_valid_period: Option<u64>,

    /// Consensus configuration updates
    ///
    /// New list of messengers.
    pub messengers: Option<Vec<String>>,
    /// New threshold needed for consensus.
    pub threshold: Option<u32>,
    /// New delta in percent per million (ppm), for which two values are considered equal.
    pub data_delta_ppm: Option<u64>,
    /// New round length in seconds.
    pub round_length: Option<u64>,
}

/// QueryMsg defines the messages that can be queried from the contract to get information.
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// GetData returns the last Solana data that was successfully published (consensus has been reached).
    #[returns(GetDataResponse)]
    GetData {},
    /// GetAum calculates and returns the current Jupiter AUM value represented in BTC.
    /// Returned value is a decimal integer with precision of `WBTC_DECIMALS`
    /// Returns error if data is not valid.
    #[returns(GetAumResponse)]
    GetAum {},
    /// GetRoundInfo returns round info that is needed for messengers to know when to publish data
    #[returns(RoundInfoResponse)]
    GetRoundInfo {},
    /// Config returns the current contract configuration.
    #[returns(ConfigResponse)]
    GetConfig {},
}

// --- Query Responses ---

/// ConfigResponse contains the current contract configuration.
#[cw_serde]
pub struct ConfigResponse {
    /// The current valid period in seconds.
    pub consensus_data_valid_period: u64,
    /// List of custody asset denoms required for consensus
    pub required_custody_assets: Vec<String>,
    /// How many blocks we consider the last price from oracle as valid
    pub price_data_valid_period: u64,
    /// a list of messengers that can submit data for consensus
    pub messengers: Vec<Addr>,
    /// threshold of the consensus (how many messengers must submit data for consensus to be reached)
    pub threshold: u32,
    /// delta in percent per million (ppm), for which two values are considered equal
    pub data_delta_ppm: u64,
    /// length of a round in seconds
    pub round_length: u64,
}

/// GetDataResponse contains the last successfully published Solana data.
#[cw_serde]
pub struct GetDataResponse {
    /// The finalized Solana data, if available.
    pub last_published_data: Option<ConsensusOutcome<SolanaData>>,
}

/// MigrateMsg is used for contract migration.
#[cw_serde]
pub struct MigrateMsg {}
