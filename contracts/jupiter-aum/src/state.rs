use consensus::consensus::State;
use cw_storage_plus::Item;
use jupiter_aum_common::types::{Config, SolanaData};

/// CONFIG stores the contract's configuration parameters.
pub const CONFIG: Item<Config> = Item::new("config");

/// CONSENSUS_STATE manages the consensus process
pub const CONSENSUS_STATE: State<SolanaData> = State::default();
