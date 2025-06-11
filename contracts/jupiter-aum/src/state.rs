use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ContractError;
use cosmwasm_std::{to_json_binary, Addr, StdError, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use hex::encode as hex_encode;
use sha2::{Digest, Sha256}; // from `hex` crate

/// Config defines the contract's configuration parameters.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    /// The address that is allowed to perform management actions in the contract.
    pub admin: Addr,
    /// A list of oracle addresses responsible for publishing off-chain data.
    pub oracles: Vec<Addr>,
    /// The minimum number of oracles that must agree on published off-chain data for consensus.
    pub threshold: u32,
    /// Data must be extracted from Solana from each extract_period slot.
    pub extract_period: u64,
    /// How long (in seconds) do we consider data as valid after publishing.
    pub valid_period: u64,
}

impl Config {
    /// Validates the configuration parameters.
    /// Ensures that the consensus threshold is valid (not zero and not greater than the number of oracles).
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.threshold == 0 {
            return Err(ContractError::InvalidThreshold {
                threshold: self.threshold,
                oracles: self.oracles.len(),
            });
        }
        if self.threshold > self.oracles.len() as u32 {
            return Err(ContractError::InvalidThreshold {
                threshold: self.threshold,
                oracles: self.oracles.len(),
            });
        }
        if self.extract_period == 0 {
            // TODO: specific error
            return Err(ContractError::Std(StdError::generic_err(
                "extract period must be greater than 0",
            )));
        }
        Ok(())
    }
}

/// SolanaData represents the off-chain data pulled from the Solana blockchain
/// specifically for Jupiter AUM (Assets Under Management) calculation.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct SolanaData {
    /// Timestamp when the data has been published
    pub timestamp: Timestamp,
    /// The Solana slot number from which this data was extracted.
    pub slot: u64,
    /// Slice of CustodyAssets from each Custody
    pub custody_assets: Vec<CustodyAsset>,
    /// Jupiter's Assets Under Management value in USD.
    pub aum_usd: Uint128,
    /// The total supply of JLP (Jupiter Liquidity Provider) tokens.
    pub jlp_total_supply: Uint128,
    /// The balance of JLP tokens held by the strategy.
    pub strategy_jlp_balance: Uint128,
}

impl SolanaData {
    pub fn hash(&self) -> Result<String, ContractError> {
        let bin =
            to_json_binary(self).map_err(|_| StdError::generic_err("json serialization error"))?; // TODO: contract error
        let hash = Sha256::digest(bin).as_slice().to_vec();
        Ok(hex_encode(hash))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CustodyAsset {
    // TODO: field descriptions
    pub owned: u64,
    pub locked: u64,
    pub guaranteed_usd: u64,
    pub decimals: u8,
    pub denom: String,
}

/// PendingData stores an individual oracle's publication for a specific Solana slot.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PendingData {
    /// The Solana data published by an oracle.
    pub data: SolanaData,
    /// TODO
    pub oracle: Addr,
}

/// PublishedData stores published data.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PublishedData {
    /// The Solana data published by an oracle.
    pub data: SolanaData,
    /// The timestamp when this data was published on-chain.
    pub published_at: Timestamp,
}

// --- State Storage ---

/// CONFIG stores the contract's configuration parameters.
pub const CONFIG: Item<Config> = Item::new("config");

/// LAST_PUBLISHED_DATA stores the most recent Solana data that achieved consensus.
pub const LAST_PUBLISHED_DATA: Item<PublishedData> = Item::new("last_published_data");

/// ORACLE_PUBLICATIONS stores currently pending data from Solana from each oracle, grouped by slot.
/// This acts as the `pending_data` map.
/// The key is a tuple: (solana_slot, solana_data_hash).
/// The value is the `PublishedData` submitted by that oracle for that slot.
pub const PENDING_DATA: Map<(u64, String), Vec<PendingData>> = Map::new("pending_data");
