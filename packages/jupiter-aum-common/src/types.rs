use crate::error::ContractError;
use consensus::consensus::{
    all_items_equal, consensus_on_items, consensus_on_items_dec256, consensus_on_items_u64,
    consensus_on_items_u8, consensus_on_items_uint128, ConsensusData,
};
use consensus::error::ConsensusError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Int256, SignedDecimal256, Uint128};
use neutron_std::types::neutron::util::precdec::PrecDec;
use neutron_std::types::slinky::types::v1::CurrencyPair;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Config defines the contract's configuration parameters.
#[cw_serde]
pub struct Config {
    /// How long (in seconds) do we consider data as valid after publishing (after consensus reached).
    pub consensus_data_valid_period: u64,
    /// How many blocks we consider the last price from oracle as valid
    pub price_data_valid_period: u64,
    /// List of custody asset denoms required for consensus
    pub required_custody_assets: Vec<String>,
    /// List of solana addresses (key) which balances of assets (value) are required for consensus.
    /// Must contain jlp_token balance tracking for the strategy_address.
    pub required_solana_balances: HashMap<String, Vec<String>>,
    /// List of solana tokens which total supply is required for consensus
    pub required_solana_token_total_supply: Vec<String>,
    /// The address of the strategy contract used in AUM calculations. Must be specified in the
    /// required_solana_balances along with jlp_token.
    pub strategy_address: String,
    /// The address of the JLP token used in AUM calculations. Must be specified in the
    /// required_solana_balances along with strategy_address.
    pub jlp_token: String,
    /// Map of Solana asset names to Slinky oracle asset names for price lookups
    pub solana_slinky_map: HashMap<String, String>,
}

impl Config {
    /// Validates the configuration parameters.
    pub fn validate(&self) -> Result<(), ContractError> {
        // check validity periods
        if self.consensus_data_valid_period == 0 {
            return Err(ContractError::InvalidConsensusPeriod {});
        }
        if self.price_data_valid_period == 0 {
            return Err(ContractError::InvalidPriceDataPeriod {});
        }

        // check strategy and jlp token addresses
        if self.strategy_address.is_empty() {
            return Err(ContractError::InvalidStrategyAddress {});
        }
        if self.jlp_token.is_empty() {
            return Err(ContractError::InvalidJlpToken {});
        }

        // check required custody assets for duplicates
        if let Some(duplicate) = find_duplicate(&self.required_custody_assets) {
            return Err(ContractError::DuplicateCustodyAsset {
                asset: duplicate.to_string(),
            });
        }

        // check required solana balances for duplicates by asset
        for (address, assets) in self.required_solana_balances.iter() {
            if let Some(duplicate) = find_duplicate(assets) {
                return Err(ContractError::DuplicateSolanaBalanceAsset {
                    address: address.to_string(),
                    asset: duplicate.to_string(),
                });
            }
        }
        // check that strategy JLP balance tracking is required
        if let Some(strategy_address_assets) =
            self.required_solana_balances.get(&self.strategy_address)
        {
            if !strategy_address_assets.contains(&self.jlp_token) {
                return Err(ContractError::StrategyJlpBalanceNotTracked {});
            }
        } else {
            return Err(ContractError::StrategyJlpBalanceNotTracked {});
        }

        // check token supply for duplicates
        if let Some(duplicate) = find_duplicate(&self.required_solana_token_total_supply) {
            return Err(ContractError::DuplicateSolanaTokenTotalSupply {
                asset: duplicate.to_string(),
            });
        }
        // check that JLP token supply tracking is required
        if !self
            .required_solana_token_total_supply
            .contains(&self.jlp_token)
        {
            return Err(ContractError::JlpTotalSupplyNotTracked {});
        }

        // check that all assets from required_solana_balances except jlp_token are represented in solana_slinky_map
        for asset in self.required_solana_balances.values().flatten() {
            if asset != &self.jlp_token && !self.solana_slinky_map.contains_key(asset) {
                return Err(ContractError::AssetNotInSlinkyMap {
                    asset: asset.clone(),
                });
            }
        }

        // check that jlp_token is not present in solana_slinky_map
        if self.solana_slinky_map.contains_key(&self.jlp_token) {
            return Err(ContractError::JlpTokenInSlinkyMap {});
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
    /// Contains information about asset balances on Solana accounts
    pub solana_balances: Vec<SolanaBalance>,
    /// Contains information about the total supply of Solana tokens
    pub solana_token_total_supply: Vec<SolanaTokenTotalSupply>,
    /// Contains information about token decimals on Solana
    pub solana_token_decimals: Vec<SolanaTokenDecimals>,
}

impl ConsensusData<Config> for SolanaData {
    fn prepublish_cleanup(&mut self, config: Config) -> Result<(), ConsensusError> {
        // CUSTODY ASSETS
        // retain only custody assets that are defined as required in the config
        self.custody_assets.retain(|c| {
            config
                .required_custody_assets
                .contains(&c.denom.to_string())
        });
        // sort and dedup custody assets by denom
        self.custody_assets
            .sort_by(|c1, c2| c1.denom.cmp(&c2.denom));
        self.custody_assets.dedup_by(|a, b| a.denom.eq(&b.denom));
        // check if only required custody assets are present
        if self.custody_assets.len() != config.required_custody_assets.len() {
            return Err(ConsensusError::PrepublishError {
                msg: "Provided solana custody assets don't match required ones".into(),
            });
        }

        // SOLANA BALANCES
        // retain only solana balances that are defined as required in the config
        self.solana_balances.retain(|b| {
            let required_address_balances = config.required_solana_balances.get(&b.address);
            if let Some(required_address_balances) = required_address_balances {
                return required_address_balances.contains(&b.asset);
            }
            false
        });
        // sort solana balances by address and asset
        self.solana_balances.sort_by(|a, b| {
            let a_key = format!("{}{}", a.address, a.asset);
            let b_key = format!("{}{}", b.address, b.asset);
            a_key.cmp(&b_key)
        });
        // check if only required solana balances are present
        if self.solana_balances.len()
            != config
                .required_solana_balances
                .values()
                .flatten()
                .collect::<Vec<_>>()
                .len()
        {
            return Err(ConsensusError::PrepublishError {
                msg: "Provided solana balances don't match required ones".into(),
            });
        }

        // SOLANA TOKEN TOTAL SUPPLY
        // retain only solana token total supplies that are defined as required in the config
        self.solana_token_total_supply
            .retain(|t| config.required_solana_token_total_supply.contains(&t.asset));
        // sort solana token total supplies by asset
        self.solana_token_total_supply
            .sort_by(|a, b| a.asset.cmp(&b.asset));
        // check if only required solana token total supplies are present
        if self.solana_token_total_supply.len() != config.required_solana_token_total_supply.len() {
            return Err(ConsensusError::PrepublishError {
                msg: "Provided solana token total supplies don't match required ones".into(),
            });
        }

        // SOLANA TOKEN DECIMALS
        // retain only solana token decimals of assets that are specified in the required config values
        let required_balance_assets: Vec<String> = config
            .required_solana_balances
            .values()
            .flatten()
            .map(|s| s.to_string())
            .collect();
        let required_total_supply_assets: Vec<String> = config
            .required_solana_token_total_supply
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut required_assets: Vec<String> = required_balance_assets
            .into_iter()
            .chain(required_total_supply_assets)
            .collect();
        self.solana_token_decimals
            .retain(|d| required_assets.contains(&d.asset));
        // sort and dedup required assets
        required_assets.sort();
        required_assets.dedup();
        // check if only required solana token decimals are present
        if self.solana_token_decimals.len() != required_assets.len() {
            return Err(ConsensusError::PrepublishError {
                msg: "Provided solana token decimals don't match required ones".into(),
            });
        }

        Ok(())
    }

    fn try_consensus(data: &[SolanaData], threshold: usize, delta_ppm: u64) -> Option<SolanaData> {
        if data.len() < threshold {
            return None;
        }

        // iterate over custody assets and decimals and pick out the majority
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
        for (i, _) in first.solana_token_decimals.iter().enumerate() {
            let solana_token_decimals =
                |value: &SolanaData| value.solana_token_decimals[i].decimals;
            non_matching_indices = find_unequal_indices_mapped(
                non_matching_indices,
                data,
                threshold,
                solana_token_decimals,
            )?;
        }

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

        // Consistency checks on vector lengths
        let custody_assets_lengths = data
            .iter()
            .map(|d| d.custody_assets.len() as u32)
            .collect::<Vec<_>>();
        all_items_equal(&custody_assets_lengths)?;
        let solana_balances_lengths = data
            .iter()
            .map(|d| d.solana_balances.len() as u32)
            .collect::<Vec<_>>();
        all_items_equal(&solana_balances_lengths)?;
        let solana_token_total_supply_lengths = data
            .iter()
            .map(|d| d.solana_token_total_supply.len() as u32)
            .collect::<Vec<_>>();
        all_items_equal(&solana_token_total_supply_lengths)?;
        let solana_token_decimals_lengths = data
            .iter()
            .map(|d| d.solana_token_decimals.len() as u32)
            .collect::<Vec<_>>();
        all_items_equal(&solana_token_decimals_lengths)?;

        // check and assign custody assets properties
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

            let consensus_guaranteed_usd =
                consensus_on_items_u64(&guaranteed_usd_items, threshold, delta_ppm)?;
            let consensus_owned = consensus_on_items_u64(&owned_items, threshold, delta_ppm)?;
            let consensus_locked = consensus_on_items_u64(&locked_items, threshold, delta_ppm)?;
            // because of the work we did to filter out non-majority over each custody asset, we can just take the first one
            let consensus_decimals = data.first()?.custody_assets[i].decimals;
            // because we have a cleanup of custody assets to ensure their exact denoms, we are sure the data is already same and filtered
            // so no need for filtering, only sanity check
            let consensus_denom = data.first()?.custody_assets[i].denom.to_string();

            consensus_custody_assets.push(CustodyAsset {
                owned: consensus_owned,
                locked: consensus_locked,
                guaranteed_usd: consensus_guaranteed_usd,
                decimals: consensus_decimals,
                denom: consensus_denom.to_string(),
            });
        }

        // check and assign solana balances properties
        let mut consensus_solana_balances = Vec::new();
        for (i, _) in data[0].solana_balances.iter().enumerate() {
            let amount_items = data
                .iter()
                .map(|d| d.solana_balances[i].amount)
                .collect::<Vec<Uint128>>();

            let consensus_amount = consensus_on_items_uint128(&amount_items, threshold, delta_ppm)?;
            let consensus_address = data.first()?.solana_balances[i].address.to_string();
            let consensus_asset = data.first()?.solana_balances[i].asset.to_string();

            consensus_solana_balances.push(SolanaBalance {
                address: consensus_address,
                asset: consensus_asset,
                amount: consensus_amount,
            });
        }

        // check and assign solana token total supply properties
        let mut consensus_solana_token_total_supply = Vec::new();
        for (i, _) in data[0].solana_token_total_supply.iter().enumerate() {
            let total_supply_items = data
                .iter()
                .map(|d| d.solana_token_total_supply[i].total_supply)
                .collect::<Vec<Uint128>>();

            let consensus_total_supply =
                consensus_on_items_uint128(&total_supply_items, threshold, delta_ppm)?;
            let consensus_asset = data.first()?.solana_token_total_supply[i].asset.to_string();

            consensus_solana_token_total_supply.push(SolanaTokenTotalSupply {
                asset: consensus_asset,
                total_supply: consensus_total_supply,
            });
        }

        // check and assign solana token decimals properties
        let mut consensus_solana_token_decimals = Vec::new();
        for (i, _) in data[0].solana_token_decimals.iter().enumerate() {
            let decimals_items = data
                .iter()
                .map(|d| d.solana_token_decimals[i].decimals)
                .collect::<Vec<u8>>();

            let consensus_decimals = consensus_on_items_u8(&decimals_items, threshold, delta_ppm)?;
            let consensus_asset = data.first()?.solana_token_decimals[i].asset.to_string();

            consensus_solana_token_decimals.push(SolanaTokenDecimals {
                asset: consensus_asset,
                decimals: consensus_decimals,
            });
        }

        Some(SolanaData {
            custody_assets: consensus_custody_assets,
            aum_usd: consensus_aum_usd,
            solana_balances: consensus_solana_balances,
            solana_token_total_supply: consensus_solana_token_total_supply,
            solana_token_decimals: consensus_solana_token_decimals,
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

/// Represents a single asset balance on a Solana account.
#[cw_serde]
pub struct SolanaBalance {
    /// Address of the account.
    pub address: String,
    /// Asset name.
    pub asset: String,
    /// Amount of the asset on the account balance.
    pub amount: Uint128,
}

/// Represents the total supply of a token on Solana.
#[cw_serde]
pub struct SolanaTokenTotalSupply {
    /// Asset name.
    pub asset: String,
    /// Total supply of the asset.
    pub total_supply: Uint128,
}

/// Represents the decimals of a token on Solana.
#[cw_serde]
pub struct SolanaTokenDecimals {
    /// Asset name.
    pub asset: String,
    /// Decimals of the asset.
    pub decimals: u8,
}

#[cw_serde]
pub struct AumInWBTC {
    /// Amount of aum in uWBTC
    pub amount: Int256,
    /// Timestamp when aum was calculated
    pub timestamp: u64,
}

/// Helper function to find duplicates in a vector of strings.
/// Returns the first duplicate found, or None if no duplicates exist.
fn find_duplicate(items: &[String]) -> Option<&String> {
    let mut seen = HashSet::new();
    items.iter().find(|&item| !seen.insert(item))
}

#[cw_serde]
pub enum QueryMsg {
    GetPrices {
        token_a: TokenData,
        token_b: TokenData,
    },
}

#[cw_serde]
pub struct TokenData {
    pub denom: String,
    pub decimals: u8,
    pub pair: CurrencyPair,
    pub max_blocks_old: u64,
}

#[cw_serde]
pub struct CombinedPriceResponse {
    pub token_0_price: PrecDec,
    pub token_1_price: PrecDec,
    pub price_0_to_1: PrecDec,
}
