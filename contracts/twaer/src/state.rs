use cosmwasm_std::{Decimal, Uint128};
use cw_storage_plus::{Item, Map};
use twaer_common::types::{Config, OldConfig, TwaAggregator};

pub const CONFIG: Item<Config> = Item::new("config");
pub const OLD_CONFIG: Item<OldConfig> = Item::new("config");

/// The "agreed upon" TWA exchange rate (paired with a timestamp) that is returned by GetTwaer
/// queries. This value is updated when PublishTwaer is explicitly called by the admin, providing
/// a stable, controlled value.
pub const TWAER: Item<(Decimal, u64)> = Item::new("twaer");

/// Incremental TWA calculation state. Updated every time RecordEr is called.
pub const TWA_AGGREGATOR: Item<TwaAggregator> = Item::new("twa_aggregator");

/// Historical exchange rate data points, keyed by timestamp.
pub const ER_HISTORY: Map<u64, Decimal> = Map::new("er_history");

/// The mocked maxBTC supply used instead of the real supply before the token is minted.
pub const MOCKED_MAXBTC_SUPPLY: Item<Uint128> = Item::new("mocked_maxbtc_supply");
