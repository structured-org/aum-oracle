use crate::error::{ContractError, ContractResult};
use consensus::consensus::{consensus_on_field, consensus_on_items_dec256, ConsensusData, State};
use consensus::error::ConsensusError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, Int256, SignedDecimal256};
use cw_storage_plus::Item;

pub const CONFIG: Item<Config> = Item::new("config");

pub const CONSENSUS_STATE: State<BinanceData, Config> = State::default();

pub const AUM_IN_WBTC: Item<AumInWBTC> = Item::new("aum_in_wbtc");

#[cw_serde]
pub struct Config {
    /// owner of the contract
    pub owner: Addr,
    /// address of price oracle contract
    pub price_oracle_contract: Addr,
    /// Validity period for data (that reached consensus) in the contract in seconds. If the data is too old,
    /// Binance AUM contract cannot rely on it in AUM calculations,
    /// and in that case, the contract just doesn't calculate AUM and returns an error in the corresponding query.
    pub consensus_data_valid_period: u64,
    /// Validity period for prices from the oracle contract in blocks. If the prices are too old,
    /// Binance AUM contract cannot rely on them in AUM calculations,
    /// and in that case, the contract just doesn't calculate AUM and returns an error in the corresponding query.
    pub price_data_valid_period: u64,
    /// required binance positions that messengers must provide
    pub required_binance_positions: Vec<String>,
    /// required binance spot assets that messengers must provide
    pub required_binance_spot_assets: Vec<String>,
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

#[cw_serde]
pub struct Position {
    pub symbol: String,
    pub amount: SignedDecimal256,
    pub pnl: SignedDecimal256,
}

#[cw_serde]
pub struct SpotBalance {
    pub asset: String,
    pub amount: SignedDecimal256,
}

#[cw_serde]
pub struct BinanceData {
    pub unimmr: SignedDecimal256,
    pub positions: Vec<Position>,
    pub um_balance_usdt: SignedDecimal256,
    pub spot_balances: Vec<SpotBalance>,
    pub pm_account_actual_equity: SignedDecimal256,
    pub withdrawable_usdt: SignedDecimal256,
}

impl ConsensusData<Config> for BinanceData {
    fn prepublish_cleanup(&mut self, options: Config) -> Result<(), ConsensusError> {
        // clean and validate published data
        // positions must contain only required binance positions
        self.positions
            .retain(|p| options.required_binance_positions.contains(&p.symbol));
        self.positions.sort_by(|a, b| a.symbol.cmp(&b.symbol));

        if self.positions.len() != options.required_binance_positions.len() {
            return Err(ConsensusError::PrepublishError {
                msg: "Binance positions do not match required positions".into(),
            });
        }

        // spot_balances must contain only required binance spot assets
        self.spot_balances
            .retain(|p| options.required_binance_spot_assets.contains(&p.asset));
        self.spot_balances.sort_by(|a, b| a.asset.cmp(&b.asset));

        if self.spot_balances.len() != options.required_binance_spot_assets.len() {
            return Err(ConsensusError::PrepublishError {
                msg: "Binance spot assets do not match required spot assets".into(),
            });
        }

        Ok(())
    }

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
            let amounts: Vec<SignedDecimal256> =
                data.iter().map(|d| d.positions[i].amount).collect();
            let pnls: Vec<SignedDecimal256> = data.iter().map(|d| d.positions[i].pnl).collect();

            consensus_positions.push(Position {
                symbol: data[0].positions[i].symbol.clone(), // all equal
                amount: consensus_on_items_dec256(&amounts, threshold, delta_ppm)?,
                pnl: consensus_on_items_dec256(&pnls, threshold, delta_ppm)?,
            });
        }

        // Vec<SpotBalance> consensus
        let mut consensus_spot = Vec::with_capacity(spot_len);
        for i in 0..spot_len {
            let amounts: Vec<SignedDecimal256> =
                data.iter().map(|d| d.spot_balances[i].amount).collect();
            consensus_spot.push(SpotBalance {
                asset: data[0].spot_balances[i].asset.clone(), // all equal
                amount: consensus_on_items_dec256(&amounts, threshold, delta_ppm)?,
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

impl Config {
    /// Updates the contract configuration with new values, keeping existing values for None options
    pub fn update_config(
        &mut self,
        deps: Deps,
        new_config: &crate::msg::UpdateConfig,
    ) -> ContractResult<()> {
        if let Some(ref owner) = new_config.owner {
            self.owner = deps.api.addr_validate(owner)?;
        }
        if let Some(consensus_data_valid_period) = new_config.consensus_data_valid_period {
            self.consensus_data_valid_period = consensus_data_valid_period;
        }
        if let Some(price_data_valid_period) = new_config.price_data_valid_period {
            self.price_data_valid_period = price_data_valid_period;
        }
        if let Some(ref required_binance_positions) = new_config.required_binance_positions {
            self.required_binance_positions = required_binance_positions.clone();
        }
        if let Some(ref required_binance_spot_assets) = new_config.required_binance_spot_assets {
            self.required_binance_spot_assets = required_binance_spot_assets.clone();
        }
        if let Some(ref price_oracle_contract) = new_config.price_oracle_contract {
            self.price_oracle_contract = deps.api.addr_validate(price_oracle_contract)?;
        }

        self.validate()?;

        Ok(())
    }
}

#[cw_serde]
pub struct AumInWBTC {
    /// Amount of aum in uWBTC
    pub amount: Int256,
    /// Timestamp when aum was calculated
    pub timestamp: u64,
}
