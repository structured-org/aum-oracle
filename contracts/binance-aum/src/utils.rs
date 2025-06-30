use crate::error::{ContractError, ContractResult};
use cosmwasm_schema::schemars;
use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_std::{Deps, SignedDecimal};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CombinedPriceResponse {
    pub token_0_price: SignedDecimal,
    pub token_1_price: SignedDecimal,
    pub price_0_to_1: SignedDecimal,
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
            &serde_json::json!(
                        {
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
