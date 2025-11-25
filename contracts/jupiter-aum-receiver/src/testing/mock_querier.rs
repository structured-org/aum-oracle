use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    from_json, Binary, ContractResult, Empty, GrpcQuery, OwnedDeps, Querier, QuerierResult,
    QueryRequest, SystemError, SystemResult,
};
use neutron_std::types::slinky::oracle::v1::{GetPriceRequest, QuotePrice};
use std::collections::HashMap;
use std::marker::PhantomData;

pub fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_storage = MockStorage::default();
    let custom_querier = WasmMockQuerier::new(MockQuerier::new(&[]));

    OwnedDeps {
        storage: custom_storage,
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: PhantomData,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier,
    prices: HashMap<String, (String, u64)>, // asset -> (price, block_height)
}

impl WasmMockQuerier {
    fn new(base: MockQuerier) -> WasmMockQuerier {
        WasmMockQuerier {
            base,
            prices: HashMap::new(),
        }
    }

    pub(crate) fn add_price(
        &mut self,
        asset: impl Into<String>,
        price: impl Into<String>,
        height: u64,
    ) {
        self.prices.insert(asset.into(), (price.into(), height));
    }
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<Empty> = match from_json(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return QuerierResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                });
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match request {
            QueryRequest::Grpc(GrpcQuery { data, path }) => match path.as_str() {
                GetPriceRequest::PATH => {
                    // Decode the protobuf data to get the CurrencyPair
                    let get_price_request = match GetPriceRequest::try_from(data.clone()) {
                        Ok(req) => req,
                        Err(_) => {
                            return QuerierResult::Err(SystemError::InvalidRequest {
                                error: "Failed to decode GetPriceRequest".to_string(),
                                request: data.clone(),
                            });
                        }
                    };

                    let asset = if let Some(currency_pair) = get_price_request.currency_pair {
                        // Use the base asset from the CurrencyPair
                        currency_pair.base
                    } else {
                        // Return empty price response if no currency pair is specified
                        let resp = neutron_std::types::slinky::oracle::v1::GetPriceResponse {
                            price: None, // No price available
                            nonce: 0,
                            decimals: 6,
                            id: 0,
                        }
                        .to_proto_bytes();
                        return SystemResult::Ok(ContractResult::Ok(Binary::from(resp)));
                    };

                    match self.prices.get(&asset) {
                        Some((price, height)) => {
                            let resp = neutron_std::types::slinky::oracle::v1::GetPriceResponse {
                                price: Some(QuotePrice {
                                    price: price.clone(),
                                    block_timestamp: None,
                                    block_height: *height,
                                }),
                                nonce: 0,
                                decimals: 6,
                                id: 0,
                            }
                            .to_proto_bytes();
                            SystemResult::Ok(ContractResult::Ok(Binary::from(resp)))
                        }
                        None => {
                            // Return empty price response to simulate missing price
                            let resp = neutron_std::types::slinky::oracle::v1::GetPriceResponse {
                                price: None, // No price available
                                nonce: 0,
                                decimals: 6,
                                id: 0,
                            }
                            .to_proto_bytes();
                            SystemResult::Ok(ContractResult::Ok(Binary::from(resp)))
                        }
                    }
                }
                _ => unimplemented!(),
            },
            _ => self.base.handle_query(request),
        }
    }
}
