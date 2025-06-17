use consensus::consensus::{consensus_on_items, ConsensusData, State};
use cosmwasm_std::{Addr, SignedDecimal};
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    pub admin: Addr,
    pub valid_period: u64,
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

pub const CONFIG: Item<Config> = Item::new("config");

pub const CONSENSUS_STATE: State<BinanceData> = State::new();

impl ConsensusData for BinanceData {
    fn try_consensus(
        data: &[BinanceData],
        threshold: usize,
        delta_ppm: u64,
    ) -> Option<BinanceData> {
        if data.len() < threshold {
            return None;
        }
        // Will fill with consensus values
        // For each field (flattened below)
        let consensus_unimmr = consensus_on_field(data, |d| d.unimmr, threshold, delta_ppm)?;
        let consensus_um_balance_usdt =
            consensus_on_field(data, |d| d.um_balance_usdt, threshold, delta_ppm)?;
        let consensus_pm_equity =
            consensus_on_field(data, |d| d.pm_account_actual_equity, threshold, delta_ppm)?;
        let consensus_withdrawable_usdt =
            consensus_on_field(data, |d| d.withdrawable_usdt, threshold, delta_ppm)?;

        // Positions (by symbol): consensus on all positions by symbol (must match number/order of symbols)
        let symbols: Vec<String> = data[0].positions.iter().map(|p| p.symbol.clone()).collect();
        let mut consensus_positions = Vec::new();
        for (i, symbol) in symbols.iter().enumerate() {
            let amounts: Vec<SignedDecimal> = data.iter().map(|d| d.positions[i].amount).collect();
            let pnls: Vec<SignedDecimal> = data.iter().map(|d| d.positions[i].pnl).collect();
            let amount = consensus_on_items(&amounts, threshold, delta_ppm)?;
            let pnl = consensus_on_items(&pnls, threshold, delta_ppm)?;
            consensus_positions.push(Position {
                symbol: symbol.clone(),
                amount,
                pnl,
            });
        }
        // Spot balances (by asset)
        let assets: Vec<String> = data[0]
            .spot_balances
            .iter()
            .map(|b| b.asset.clone())
            .collect();
        let mut consensus_spot = Vec::new();
        for (i, asset) in assets.iter().enumerate() {
            let amounts: Vec<SignedDecimal> =
                data.iter().map(|d| d.spot_balances[i].amount).collect();
            let amount = consensus_on_items(&amounts, threshold, delta_ppm)?;
            consensus_spot.push(SpotBalance {
                asset: asset.clone(),
                amount,
            });
        }

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
