use crate::error::ContractError;
use consensus::consensus::{
    all_items_equal, consensus_on_items, consensus_on_items_dec256, consensus_on_items_u64,
    ConsensusData,
};
use consensus::error::ConsensusError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Int256, SignedDecimal256, Uint128};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Config defines the contract's configuration parameters.
#[cw_serde]
pub struct Config {
    /// The address that is allowed to perform management actions in the contract.
    pub owner: Addr,
    /// How long (in seconds) do we consider data as valid after publishing (after consensus reached).
    pub consensus_data_valid_period: u64,
    /// List of custody asset denoms required for consensus
    pub required_custody_assets: Vec<String>,
    /// How many blocks we consider the last price from oracle as valid
    pub price_data_valid_period: u64,
}

impl Config {
    /// Validates the configuration parameters.
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.consensus_data_valid_period == 0 {
            return Err(ContractError::InvalidConsensusPeriod {});
        }

        if self.price_data_valid_period == 0 {
            return Err(ContractError::InvalidPriceDataPeriod {});
        }

        Ok(())
    }
}

/// SolanaData represents the off-chain data pulled from the Solana blockchain
/// specifically for Jupiter AUM (Assets Under Management) calculation.

#[cw_serde]
pub struct SolanaData {
    /// Slice of CustodyAssets from each Custody
    pub custody_assets: Vec<CustodyAsset>,
    /// Jupiter's Assets Under Management value in USD.
    pub aum_usd: Uint128,
    /// The total supply of JLP (Jupiter Liquidity Provider) tokens.
    pub total_jlp_supply: Uint128,
    /// JLP token supply decimal precision
    pub total_jlp_supply_decimals: u8,
    /// The balance of JLP tokens held by the strategy.
    pub strategy_jlp_balance: Uint128,
    /// JLP token balance decimal precision
    pub strategy_jlp_balance_decimals: u8,
}

impl ConsensusData<Config> for SolanaData {
    fn prepublish_cleanup(&mut self, config: Config) -> Result<(), ConsensusError> {
        self.custody_assets.retain(|c| {
            config
                .required_custody_assets
                .contains(&c.denom.to_string())
        });
        self.custody_assets
            .sort_by(|c1, c2| c1.denom.cmp(&c2.denom));
        self.custody_assets.dedup_by(|a, b| a.denom.eq(&b.denom));

        if self.custody_assets.len() != config.required_custody_assets.len() {
            return Err(ConsensusError::PrepublishError {
                msg: "Solana custody assets have some required custody assets missing".into(),
            });
        }

        Ok(())
    }

    fn try_consensus(data: &[SolanaData], threshold: usize, delta_ppm: u64) -> Option<SolanaData> {
        if data.len() < threshold {
            return None;
        }

        // iterate over custody assets and pick out the majority on decimals
        let mut non_matching_indices: HashSet<usize> = HashSet::new();
        let first = data.first()?;
        for (i, _) in first.custody_assets.iter().enumerate() {
            let custody_asset_decimals = |value: &SolanaData| value.custody_assets[i].decimals;
            non_matching_indices = find_unequal_indices_mapped(
                non_matching_indices,
                data,
                threshold,
                custody_asset_decimals,
            )?;
        }

        non_matching_indices =
            find_unequal_indices_mapped(non_matching_indices, data, threshold, |value| {
                value.total_jlp_supply_decimals
            })?;

        non_matching_indices =
            find_unequal_indices_mapped(non_matching_indices, data, threshold, |value| {
                value.strategy_jlp_balance_decimals
            })?;

        // remove all non-matching indices
        let data: Vec<SolanaData> = data
            .iter()
            .cloned()
            .enumerate()
            .filter(|(i, _)| !non_matching_indices.contains(i))
            .map(|(_, v)| v)
            .collect();

        // check top level fields
        let consensus_aum_usd =
            consensus_on_field_u128(&data, |d| d.aum_usd, threshold, delta_ppm)?;
        let consensus_total_jlp_supply =
            consensus_on_field_u128(&data, |d| d.total_jlp_supply, threshold, delta_ppm)?;
        let consensus_strategy_jlp_balance =
            consensus_on_field_u128(&data, |d| d.strategy_jlp_balance, threshold, delta_ppm)?;

        // check that all custody assets have the same length
        let custody_assets_lengths = data
            .iter()
            .map(|d| d.custody_assets.len() as u32)
            .collect::<Vec<_>>();
        all_items_equal(&custody_assets_lengths)?;

        let consensus_total_jlp_supply_decimals = data.first()?.total_jlp_supply_decimals;
        let consensus_strategy_jlp_balance_decimals = data.first()?.strategy_jlp_balance_decimals;

        // check and assign custody assets properties
        let mut consensus_custody_assets = Vec::new();
        for (i, _) in data[0].custody_assets.iter().enumerate() {
            let decimals_items = data
                .iter()
                .map(|d| d.custody_assets[i].decimals)
                .collect::<Vec<u8>>();

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
            let denom_items = data
                .iter()
                .map(|d| d.custody_assets[i].denom.clone())
                .collect::<Vec<String>>();

            let consensus_guaranteed_usd =
                consensus_on_items_u64(&guaranteed_usd_items, threshold, delta_ppm)?;
            let consensus_owned = consensus_on_items_u64(&owned_items, threshold, delta_ppm)?;
            let consensus_locked = consensus_on_items_u64(&locked_items, threshold, delta_ppm)?;
            // because of the work we did to filter out non-majority over each custody asset, we can just take the first one
            let consensus_decimals = decimals_items.first()?;
            // because we have a cleanup of custody assets to ensure their exact denoms, we are sure the data is already same and filtered
            // so no need for filtering, only sanity check
            let consensus_denom = denom_items.first()?;

            consensus_custody_assets.push(CustodyAsset {
                owned: consensus_owned,
                locked: consensus_locked,
                guaranteed_usd: consensus_guaranteed_usd,
                decimals: *consensus_decimals,
                denom: consensus_denom.to_string(),
            });
        }

        Some(SolanaData {
            custody_assets: consensus_custody_assets,
            aum_usd: consensus_aum_usd,
            total_jlp_supply_decimals: consensus_total_jlp_supply_decimals,
            total_jlp_supply: consensus_total_jlp_supply,
            strategy_jlp_balance: consensus_strategy_jlp_balance,
            strategy_jlp_balance_decimals: consensus_strategy_jlp_balance_decimals,
        })
    }
}

pub fn find_unequal_indices_mapped<T: Eq + Hash, F>(
    filtered_out_items: HashSet<usize>,
    data: &[SolanaData],
    threshold: usize,
    f: F,
) -> Option<HashSet<usize>>
where
    F: Fn(&SolanaData) -> T,
{
    let items_enumerated: Vec<(usize, T)> = data
        .iter()
        .enumerate()
        .map(|(index, value)| (index, f(value)))
        .collect();
    find_indices_of_non_matching_items(filtered_out_items, &items_enumerated, threshold)
}

pub fn find_indices_of_non_matching_items<T: Eq + Hash>(
    filtered_out_items: HashSet<usize>,
    items: &[(usize, T)],
    threshold: usize,
) -> Option<HashSet<usize>> {
    let mut groups: HashMap<&T, Vec<usize>> = HashMap::new();

    // group all items by equality
    for (index, value) in items.iter() {
        if !filtered_out_items.contains(index) {
            groups.entry(value).or_default().push(*index);
        }
    }

    let mut result: HashSet<usize> = filtered_out_items;
    let mut consensus_found = false;

    for (_, indices) in groups {
        // add all groups of non-matching items to result
        if indices.len() < threshold {
            let indices_set: HashSet<usize> = indices.into_iter().collect();
            result = result.union(&indices_set).copied().collect();
        } else {
            // if the threshold is reached, overall consensus is found
            consensus_found = true
        }
    }

    if consensus_found {
        Some(result)
    } else {
        None
    }
}

// Single field consensus on SignedDecimal256
pub fn consensus_on_field_dec_256<F>(
    data: &[SolanaData],
    extract: F,
    threshold: usize,
    delta_ppm: u64,
) -> Option<SignedDecimal256>
where
    F: Fn(&SolanaData) -> SignedDecimal256,
{
    let items: Vec<SignedDecimal256> = data.iter().map(&extract).collect();
    consensus_on_items_dec256(&items, threshold, delta_ppm)
}

// Single field consensus on u128
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
    let ppm = Decimal::from_ratio(delta_ppm, 1_000_000u64);
    consensus_on_items(
        &items,
        threshold,
        |high, low| {
            let diff = high.abs_diff(low);
            let decimal_high = Decimal::from_atomics(high, 0).ok()?;
            let max_dispersion = (decimal_high * ppm).to_uint_floor();
            Some(diff <= max_dispersion)
        },
        Uint128::new(2),
    )
}

// Represents Jupiter asset under custody in JLP pool
#[cw_serde]
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

#[cw_serde]
pub struct AumInWBTC {
    /// Amount of aum in uWBTC
    pub amount: Int256,
    /// Timestamp when aum was calculated
    pub timestamp: u64,
}
