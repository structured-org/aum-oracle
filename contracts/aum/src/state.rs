use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    /// owner of the contract
    pub owner: Addr,
    /// A list of AUM oracle instances from where the contract need to get individual AUMs and sum them up
    pub oracles: Vec<Addr>,
}

pub const CONFIG: Item<Config> = Item::new("config");
