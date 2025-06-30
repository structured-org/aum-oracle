use crate::error::ContractError;
use consensus::consensus::{
    consensus_on_items, consensus_on_items_u64, consensus_on_items_uint128,
    exact_consensus_on_items, ConsensusData,
};
use cosmwasm_std::{Addr, SignedDecimal, StdError, StdResult, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Config defines the contract's configuration parameters.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    /// The address that is allowed to perform management actions in the contract.
    pub admin: Addr,
    /// How long (in seconds) do we consider data as valid after publishing (after consensus reached).
    pub valid_period: u64,
    /// List of custody asset denoms required for consensus
    pub required_custody_assets: Vec<String>,
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
    /// Slice of CustodyAssets from each Custody
    pub custody_assets: Vec<CustodyAsset>,
    /// Jupiter's Assets Under Management value in USD.
    pub aum_usd: Uint128,
    /// JLP token decimal precision
    pub jlp_token_decimals: u8,
    /// The total supply of JLP (Jupiter Liquidity Provider) tokens.
    pub total_jlp_supply: Uint128,
    /// The balance of JLP tokens held by the strategy.
    pub strategy_jlp_balance: Uint128,
}

impl SolanaData {
    // Leaves only required data, sorts it and checks that all required assets passed
    pub fn clean_and_validate(&mut self, required_custody_assets: Vec<String>) -> StdResult<()> {
        self.custody_assets
            .retain(|c| required_custody_assets.contains(&c.denom.to_string()));
        self.custody_assets
            .sort_by(|c1, c2| c1.denom.cmp(&c2.denom));
        self.custody_assets.dedup_by(|a, b| a.denom.eq(&b.denom));

        if self.custody_assets.len() != required_custody_assets.len() {
            return Err(StdError::generic_err(
                "Solana custody assets have some required custody assets missing",
            ));
        }

        Ok(())
    }
}

// TODO: tests for different scenarios:
// - NO data
// - Consensus not reached
// - Consensus reached
// - Different scenarios tested for when consensus reached:
//    - all oracles give exact data
//    - almost all oracles exact data, one is not
//    - all oracles with totally different values

impl ConsensusData for SolanaData {
    fn try_consensus(data: &[SolanaData], threshold: usize, delta_ppm: u64) -> Option<SolanaData> {
        if data.len() < threshold {
            return None;
        }

        // check top level fields
        let consensus_aum_usd = consensus_on_field_u128(data, |d| d.aum_usd, threshold, delta_ppm)?;
        let consensus_total_jlp_supply =
            consensus_on_field_u128(data, |d| d.total_jlp_supply, threshold, delta_ppm)?;
        let consensus_strategy_jlp_balance =
            consensus_on_field_u128(data, |d| d.strategy_jlp_balance, threshold, delta_ppm)?;
        let consensus_jlp_token_decimals =
            exact_consensus_on_field(data, |d| d.jlp_token_decimals)?;

        // check that all custody assets have the same length
        let custody_assets_lengths = data
            .iter()
            .map(|d| d.custody_assets.len() as u32)
            .collect::<Vec<_>>();
        exact_consensus_on_items(&custody_assets_lengths)?;

        // check custody assets properties
        let mut consensus_custody_assets = Vec::new();
        for (i, _) in data[0].custody_assets.iter().enumerate() {
            let guaranteed_usd_items = data
                .iter()
                .map(|d| d.custody_assets[i].guaranteed_usd)
                .collect::<Vec<u64>>();
            let owned_items = data
                .iter()
                .map(|d| d.custody_assets[i].owned)
                .collect::<Vec<u64>>();
            let locked_items = data
                .iter()
                .map(|d| d.custody_assets[i].locked)
                .collect::<Vec<u64>>();
            let decimals_items = data
                .iter()
                .map(|d| d.custody_assets[i].decimals)
                .collect::<Vec<u8>>();
            let denom_items = data
                .iter()
                .map(|d| d.custody_assets[i].denom.clone())
                .collect::<Vec<String>>();

            let consensus_guaranteed_usd =
                consensus_on_items_u64(&guaranteed_usd_items, threshold, delta_ppm)?;
            let consensus_owned = consensus_on_items_u64(&owned_items, threshold, delta_ppm)?;
            let consensus_locked = consensus_on_items_u64(&locked_items, threshold, delta_ppm)?;
            let consensus_decimals = exact_consensus_on_items(&decimals_items)?;
            let consensus_denom = exact_consensus_on_items(&denom_items)?;

            consensus_custody_assets.push(CustodyAsset {
                owned: consensus_owned,
                locked: consensus_locked,
                guaranteed_usd: consensus_guaranteed_usd,
                decimals: consensus_decimals,
                denom: consensus_denom,
            });
        }

        Some(SolanaData {
            custody_assets: consensus_custody_assets,
            aum_usd: consensus_aum_usd,
            jlp_token_decimals: consensus_jlp_token_decimals,
            total_jlp_supply: consensus_total_jlp_supply,
            strategy_jlp_balance: consensus_strategy_jlp_balance,
        })
    }
}

// Single field exact consensus
fn exact_consensus_on_field<F>(data: &[SolanaData], extract: F) -> Option<u8>
where
    F: Fn(&SolanaData) -> u8,
{
    let items: Vec<u8> = data.iter().map(&extract).collect();
    exact_consensus_on_items(&items)
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

// Single field consensus
pub fn consensus_on_field_u128<F>(
    data: &[SolanaData],
    extract: F,
    threshold: usize,
    delta_ppm: u64,
) -> Option<Uint128>
where
    F: Fn(&SolanaData) -> Uint128,
{
    let items: Vec<Uint128> = data.iter().map(&extract).collect();
    consensus_on_items_uint128(&items, threshold, delta_ppm)
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
