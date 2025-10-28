use cw_storage_plus::Item;

use waitasaurus_common::types::{Config, State};

pub const CONFIG: Item<Config> = Item::new("config");

pub const STATE: Item<State> = Item::new("state");
