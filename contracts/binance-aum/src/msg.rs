use crate::state::BinanceData;
use consensus::consensus::{OracleData, Round};
use cosmwasm_std::SignedDecimal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {
    pub admin: String,
    pub oracles: Vec<String>,
    pub threshold: u32,
    pub data_delta_ppm: u64,
    pub round_length: u64,
    pub valid_period: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    PublishData { new_data: OracleData<BinanceData> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub enum QueryMsg {
    GetData {},
    GetAum {},
    GetRoundInfo {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RoundInfoResponse {
    pub pending_round: Round,
    pub next_round: Round,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GetDataResponse {
    pub last_published_data: Option<OracleData<BinanceData>>,
}

// AUM = (pm_account_actual_equity / btc_price_in_usd) + sum(spot_balances_in_btc)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GetAumResponse {
    pub aum_in_btc: SignedDecimal,
}
