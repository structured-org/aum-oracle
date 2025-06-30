use consensus::consensus::{consensus_on_items, ConsensusData, State};
use cosmwasm_std::{Addr, SignedDecimal, StdError, StdResult};
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    /// owner of the contract
    pub admin: Addr,
    /// address of price oracle contract
    pub price_oracle_contract: Addr,
    /// how many seconds we consider the last published consensus as valid
    pub consensus_data_valid_period: u64,
    /// how many seconds we consider the last price from oracle as valid
    pub price_max_blocks_old: u64,
    /// required binance positions and spot assets that oracles must provide
    pub required_binance_positions: Vec<String>,
    pub required_binance_spot_assets: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Position {
    pub symbol: String,
    pub amount: SignedDecimal,
    pub pnl: SignedDecimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SpotBalance {
    pub asset: String,
    pub amount: SignedDecimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BinanceData {
    pub unimmr: SignedDecimal,
    pub positions: Vec<Position>,
    pub um_balance_usdt: SignedDecimal,
    pub spot_balances: Vec<SpotBalance>,
    pub pm_account_actual_equity: SignedDecimal,
    pub withdrawable_usdt: SignedDecimal,
}

impl BinanceData {
    /// Cleans the data to only contain the required binance positions and spot assets
    /// Validates the data to contain required positions and spot assets. Returns an error if it does not.
    pub fn clean_and_validate(
        &mut self,
        required_binance_positions: Vec<String>,
        required_binance_spot_assets: Vec<String>,
    ) -> StdResult<()> {
        // positions must contain only required binance positions
        self.positions
            .retain(|p| required_binance_positions.contains(&p.symbol));
        self.positions.sort_by(|a, b| a.symbol.cmp(&b.symbol));

        if self.positions.len() != required_binance_positions.len() {
            return Err(StdError::generic_err(
                "Binance positions do not match required positions",
            ));
        }

        // spot_balances must contain only required binance spot assets
        self.spot_balances
            .retain(|p| required_binance_spot_assets.contains(&p.asset));
        self.spot_balances.sort_by(|a, b| a.asset.cmp(&b.asset));

        if self.spot_balances.len() != required_binance_spot_assets.len() {
            return Err(StdError::generic_err(
                "Binance spot assets do not match required spot assets",
            ));
        }

        Ok(())
    }
}

impl ConsensusData for BinanceData {
    fn try_consensus(
        data: &[BinanceData],
        threshold: usize,
        delta_ppm: u64,
    ) -> Option<BinanceData> {
        // Early-exit if not enough reports
        if data.len() < threshold {
            return None;
        }

        // Consistency checks on vector lengths
        let positions_len = data[0].positions.len();
        let spot_len = data[0].spot_balances.len();
        if data
            .iter()
            .any(|d| d.positions.len() != positions_len || d.spot_balances.len() != spot_len)
        {
            return None; // length mismatch
        }

        // Consistency checks on string fields
        for i in 0..positions_len {
            let ref_symbol = &data[0].positions[i].symbol;
            if data.iter().any(|d| d.positions[i].symbol != *ref_symbol) {
                return None; // differing position symbol
            }
        }
        for i in 0..spot_len {
            let ref_asset = &data[0].spot_balances[i].asset;
            if data.iter().any(|d| d.spot_balances[i].asset != *ref_asset) {
                return None; // differing spot-balance asset
            }
        }

        // Scalar-field consensus
        let consensus_unimmr = consensus_on_field(data, |d| d.unimmr, threshold, delta_ppm)?;
        let consensus_um_balance_usdt =
            consensus_on_field(data, |d| d.um_balance_usdt, threshold, delta_ppm)?;
        let consensus_pm_equity =
            consensus_on_field(data, |d| d.pm_account_actual_equity, threshold, delta_ppm)?;
        let consensus_withdrawable_usdt =
            consensus_on_field(data, |d| d.withdrawable_usdt, threshold, delta_ppm)?;

        // Vec<Position> consensus (safe because of checks above)
        let mut consensus_positions = Vec::with_capacity(positions_len);
        for i in 0..positions_len {
            let amounts: Vec<SignedDecimal> = data.iter().map(|d| d.positions[i].amount).collect();
            let pnls: Vec<SignedDecimal> = data.iter().map(|d| d.positions[i].pnl).collect();

            consensus_positions.push(Position {
                symbol: data[0].positions[i].symbol.clone(), // all equal
                amount: consensus_on_items(&amounts, threshold, delta_ppm)?,
                pnl: consensus_on_items(&pnls, threshold, delta_ppm)?,
            });
        }

        // Vec<SpotBalance> consensus
        let mut consensus_spot = Vec::with_capacity(spot_len);
        for i in 0..spot_len {
            let amounts: Vec<SignedDecimal> =
                data.iter().map(|d| d.spot_balances[i].amount).collect();
            consensus_spot.push(SpotBalance {
                asset: data[0].spot_balances[i].asset.clone(), // all equal
                amount: consensus_on_items(&amounts, threshold, delta_ppm)?,
            });
        }

        // Assemble final object
        Some(BinanceData {
            unimmr: consensus_unimmr,
            positions: consensus_positions,
            um_balance_usdt: consensus_um_balance_usdt,
            spot_balances: consensus_spot,
            pm_account_actual_equity: consensus_pm_equity,
            withdrawable_usdt: consensus_withdrawable_usdt,
        })
    }
}

// Single field consensus
pub fn consensus_on_field<F>(
    data: &[BinanceData],
    extract: F,
    threshold: usize,
    delta_ppm: u64,
) -> Option<SignedDecimal>
where
    F: Fn(&BinanceData) -> SignedDecimal,
{
    let items: Vec<SignedDecimal> = data.iter().map(&extract).collect();
    consensus_on_items(&items, threshold, delta_ppm)
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const CONSENSUS_STATE: State<BinanceData> = State::default();
