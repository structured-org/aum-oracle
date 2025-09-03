use binance_aum_common::types::{AumInWBTC, BinanceData, Config};
use consensus::consensus::State;
use cw_storage_plus::Item;

pub const CONFIG: Item<Config> = Item::new("config");

pub const CONSENSUS_STATE: State<BinanceData, Config> = State::default();

pub const AUM_IN_WBTC: Item<AumInWBTC> = Item::new("aum_in_wbtc");
