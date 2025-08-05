use consensus::consensus::State;
use cosmwasm_std::Int256;
use cw_storage_plus::Item;
use jupiter_aum_common::types::{Config, SolanaData};

/// CONFIG stores the contract's configuration parameters.
pub const CONFIG: Item<Config> = Item::new("config");

/// CONSENSUS_STATE manages the consensus process
pub const CONSENSUS_STATE: State<SolanaData, Config> = State::default();

pub const AUM_IN_WBTC: Item<Int256> = Item::new("aum_in_wbtc");
