use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    from_json, Binary, ContractResult, OwnedDeps, Querier, QuerierResult, QueryRequest,
    SystemError, SystemResult, WasmQuery,
};
use std::marker::PhantomData;

// Custom MockQuerier that can respond to price oracle queries
pub struct CustomMockQuerier {
    base: MockQuerier,
    price_oracle_responses: Vec<(String, String, String, Binary)>,
}

impl Querier for CustomMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<cosmwasm_std::Empty> = match from_json(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };

        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                // Parse the message to see if it's a price query
                let parsed_msg: serde_json::Value = match from_json(msg) {
                    Ok(v) => v,
                    Err(_) => return self.base.raw_query(bin_request),
                };

                // Check if this is a get_prices query to the price oracle
                if let Some(get_prices) = parsed_msg.get("get_prices") {
                    if let (Some(token_a_obj), Some(token_b_obj)) =
                        (get_prices.get("token_a"), get_prices.get("token_b"))
                    {
                        if let (Some(token_a), Some(token_b)) = (
                            token_a_obj.get("denom").and_then(|v| v.as_str()),
                            token_b_obj.get("denom").and_then(|v| v.as_str()),
                        ) {
                            // Find the matching response
                            for (oracle_addr, response_token_a, response_token_b, price_response) in
                                &self.price_oracle_responses
                            {
                                if contract_addr == oracle_addr
                                    && token_a == response_token_a
                                    && token_b == response_token_b
                                {
                                    return SystemResult::Ok(ContractResult::Ok(
                                        price_response.clone(),
                                    ));
                                }
                            }
                        }
                    }

                    // If no matching response is found
                    return SystemResult::Err(SystemError::Unknown {});
                }

                // Pass other queries to the base querier
                self.base.raw_query(bin_request)
            }
            _ => self.base.raw_query(bin_request),
        }
    }
}

impl CustomMockQuerier {
    pub fn new(
        base: MockQuerier,
        price_oracle_responses: Vec<(String, String, String, Binary)>,
    ) -> Self {
        CustomMockQuerier {
            base,
            price_oracle_responses,
        }
    }
}

// Helper function to create mock dependencies with custom querier
pub fn custom_mock_dependencies(
    price_oracle_responses: Vec<(String, String, String, Binary)>,
) -> OwnedDeps<MockStorage, MockApi, CustomMockQuerier> {
    let base_querier = MockQuerier::new(&[]);
    let custom_querier = CustomMockQuerier::new(base_querier, price_oracle_responses);

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: PhantomData,
    }
}
