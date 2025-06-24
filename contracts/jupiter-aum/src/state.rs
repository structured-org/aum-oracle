use cw_storage_plus::{Item, Map};
use jupiter_aum_common::types::{Config, PendingData, PublishedData};

/// CONFIG stores the contract's configuration parameters.
pub const CONFIG: Item<Config> = Item::new("config");

/// LAST_PUBLISHED_DATA stores the most recent Solana data that achieved consensus.
pub const LAST_PUBLISHED_DATA: Item<PublishedData> = Item::new("last_published_data");

/// PENDING_DATA stores currently pending data from Solana from each oracle, grouped by slot and data hash.
/// The key is a tuple: (solana_slot, solana_data_hash).
/// The value is the list of pending data submitted for this specific slot and data.
pub const PENDING_DATA: Map<(u64, String), Vec<PendingData>> = Map::new("pending_data");
