use binance_aum_common::error::{ContractError, ContractResult};
use binance_aum_common::types::SpotBalance;
use cosmwasm_schema::schemars;
use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_std::{Deps, Int256, SignedDecimal256};
use neutron_std::types::neutron::util::precdec::PrecDec;
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

/// Converts PrecDec to Decimal256
fn prec_dec_to_decimal256(p: PrecDec) -> ContractResult<SignedDecimal256> {
    let atomic = Int256::from_str(&p.atomics().to_string())?;

    Ok(SignedDecimal256::from_atomics(
        atomic,
        PrecDec::DECIMAL_PLACES,
    )?)
}

/// Deserialize PrecDec string value as SignedDecimal256
fn prec_dec_deserializer<'de, D>(deserializer: D) -> Result<SignedDecimal256, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let prec_dec = PrecDec::from_str(&s).map_err(serde::de::Error::custom)?;

    prec_dec_to_decimal256(prec_dec).map_err(serde::de::Error::custom)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CombinedPriceResponse {
    pub token_0_price: PrecDec,
    pub token_1_price: PrecDec,
    // CombinedPriceResponse in the oracle contract has price_0_to_1 as PrecDec.
    // The PrecDec has 10^27 decimal precision, and SignedDecimal256 has 10^18,
    // meaning SignedDecimal256 can't deserialize big precision numbers like 42.508858735053196497504943037
    // For such cases we need to use a custom deserializer that can successfully parse large precision numbers
    // but the precision got less (from 10^27 to 10^18).
    // And that's ok, we don't need such perfect precision anyway.
    #[serde(deserialize_with = "prec_dec_deserializer")]
    pub price_0_to_1: SignedDecimal256,
}

// a helper to get spot balance in BTC using oracle contract
pub fn spot_balance_asset_in_btc(
    deps: Deps,
    price_oracle_contract: String,
    max_blocks_old: u64,
    sb: &SpotBalance,
) -> ContractResult<SignedDecimal256> {
    let price_in_btc = get_prices(
        deps,
        price_oracle_contract,
        "BTC".to_string(),
        sb.asset.clone(),
        max_blocks_old,
    )?;

    Ok(sb.amount.checked_div(price_in_btc.price_0_to_1)?)
}

// a helper to get prices from the price of token A in token B using oracle contract
// https://github.com/neutron-org/slinky-vault/blob/545118ff29b361cc5b7af6b8fcaf083e5c4bd61c/contracts/slinky-oracle/src/msg.rs#L9
pub fn get_prices(
    deps: Deps,
    price_oracle_contract: String,
    token_a: String,
    token_b: String,
    max_blocks_old: u64,
) -> ContractResult<CombinedPriceResponse> {
    let prices: CombinedPriceResponse = deps
        .querier
        .query_wasm_smart(
            price_oracle_contract,
            &serde_json::json!({
              "get_prices": {
                "token_a": {
                  "denom": token_a.clone(), // we don't care about denoms
                  "decimals": 0, // and we don't care about decimals
                  "pair": {
                    "base": token_a,
                    "quote": "USD" // both tokens must be in USD
                  },
                  "max_blocks_old": max_blocks_old
                },
                "token_b": {
                  "denom": token_b.clone(),
                  "decimals": 0,
                  "pair": {
                    "base": token_b.clone(),
                    "quote": "USD"
                  },
                  "max_blocks_old": max_blocks_old
                }
              }
            }
                          ),
        )
        .map_err(|e| ContractError::PriceOracleError {
            msg: format!("Failed to query oracle: {}", e),
        })?;

    Ok(prices)
}
