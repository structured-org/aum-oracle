use cosmwasm_std::Int256;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {
    /// Owner of the contract
    pub owner: String,
    /// A list of AUM oracle instances from where the contract need to get individual AUMs and sum them up
    pub oracles: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// UpdateConfig updates the contract's configuration parameters.
    /// Only callable by the admin. All fields are optional, allowing partial updates.
    UpdateConfig { new_config: UpdateConfig },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UpdateConfig {
    /// New owner address
    pub owner: Option<String>,
    /// A new list of AUM oracle instances from where the contract need to get individual AUMs and sum them up
    pub oracles: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub enum QueryMsg {
    /// Returns contract's configuration
    GetConfig {},

    /// Returns the latest total AUM in Binance reported by oracles
    GetAum {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GetAumResponse {
    /// The total BTC AUM reported by oracles.
    /// The value is in micro-Bitcoin (uwBTC) = 1wBTC = 100000000 uwBTC
    pub aum_in_btc: Int256,
}
