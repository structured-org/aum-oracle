use crate::error::ContractError;
use consensus::consensus::{consensus_on_items, ConsensusData};
use cosmwasm_std::{Addr, SignedDecimal, Timestamp, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Config defines the contract's configuration parameters.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    /// The address that is allowed to perform management actions in the contract.
    pub admin: Addr,
    /// How long (in seconds) do we consider data as valid after publishing (after consensus reached).
    pub valid_period: u64,
}

impl Config {
    /// Validates the configuration parameters.
    pub fn validate(&self) -> Result<(), ContractError> {
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
    // pub slot: u64,
    /// Slice of CustodyAssets from each Custody
    pub custody_assets: Vec<CustodyAsset>,
    /// Jupiter's Assets Under Management value in USD.
    pub aum_usd: Uint128,
    /// The total supply of JLP (Jupiter Liquidity Provider) tokens.
    pub total_jlp_supply: Uint128,
    /// The balance of JLP tokens held by the strategy.
    pub strategy_jlp_balance: Uint128,
}

impl ConsensusData for SolanaData {
    fn try_consensus(data: &[SolanaData], threshold: usize, delta_ppm: u64) -> Option<SolanaData> {
        if data.len() < threshold {
            return None;
        }
        // Will fill with consensus values
        // For each field (flattened below)
        // let consensus_aum_usd = consensus_on_field(data, |d| d.aum_usd, threshold, delta_ppm)?;
        // TODO: consensus on all fields

        Some(SolanaData {
            timestamp: Default::default(), // TODO
            custody_assets: vec![],        // TODO
            aum_usd: Default::default(),
            total_jlp_supply: Default::default(),
            strategy_jlp_balance: Default::default(),
        })
    }
}

// Single field consensus
pub fn consensus_on_field<F>(
    data: &[SolanaData],
    extract: F,
    threshold: usize,
    delta_ppm: u64,
) -> Option<SignedDecimal>
where
    F: Fn(&SolanaData) -> SignedDecimal,
{
    let items: Vec<SignedDecimal> = data.iter().map(&extract).collect();
    consensus_on_items(&items, threshold, delta_ppm)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CustodyAsset {
    /// Amount of tokens in u<DENOM>. 1<DENOM> = 10^<decimals>u<denom>
    pub owned: u64,
    /// Amount of locked tokens (used by traders) in u<DENOM>. 1<DENOM> = 10^<decimals>u<denom>
    pub locked: u64,
    /// The value in each custody account represents a total size estimate of all long positions
    pub guaranteed_usd: u64,
    /// How many decimals in each number above.
    pub decimals: u8,
    /// Custody denom.
    pub denom: String,
}
