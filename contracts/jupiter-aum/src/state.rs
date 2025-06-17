use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ContractError;
use cosmwasm_std::{to_json_binary, Addr, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use hex::encode as hex_encode;
use sha2::{Digest, Sha256};

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
    /// How long (in seconds) do we consider data as valid after publishing (after consensus reached).
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
            return Err(ContractError::InvalidPeriod {});
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
    pub total_jlp_supply: Uint128,
    /// The balance of JLP tokens held by the strategy.
    pub strategy_jlp_balance: Uint128,
}

impl SolanaData {
    pub fn hash(&self) -> Result<String, ContractError> {
        let bin = to_json_binary(self).map_err(|e| ContractError::Std(e))?;
        let hash = Sha256::digest(bin).as_slice().to_vec();
        Ok(hex_encode(hash))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CustodyAsset {
    /// Amount of tokens in u<DENOM>. 1<DENOM> = 10^<decimals>u<denom>
    pub owned: u64,
    /// Amount of locked tokens (used in trading?) in u<DENOM>. 1<DENOM> = 10^<decimals>u<denom>
    pub locked: u64,
    /// The value in each custody account represents a total size estimate of all long positions
    pub guaranteed_usd: u64,
    /// How many decimals in each number above.
    pub decimals: u8,
    /// Custody denom.
    pub denom: String,
}

/// PendingData stores an individual oracle's publication for a specific Solana slot and hash.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PendingData {
    /// The Solana data published by an oracle.
    pub data: SolanaData,
    /// Oracle address that did the publishing.
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

/// ORACLE_PUBLICATIONS stores currently pending data from Solana from each oracle, grouped by slot and data hash.
/// The key is a tuple: (solana_slot, solana_data_hash).
/// The value is the list of pending data submitted for this specific slot and data.
pub const PENDING_DATA: Map<(u64, String), Vec<PendingData>> = Map::new("pending_data");
