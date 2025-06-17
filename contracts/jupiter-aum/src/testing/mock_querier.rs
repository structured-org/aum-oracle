use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    from_json, Binary, ContractResult, Empty, GrpcQuery, OwnedDeps, Querier, QuerierResult,
    QueryRequest, SystemError, SystemResult,
};
use neutron_std::types::slinky::oracle::v1::QuotePrice;
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
    price: String,
}

impl WasmMockQuerier {
    pub(crate) fn with_price(&mut self, price: String) {
        self.price = price.to_string();
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
        match &request {
            // #[allow(deprecated)]
            QueryRequest::Grpc(GrpcQuery { data: _, path }) => match path.as_str() {
                neutron_std::types::slinky::oracle::v1::GetPriceRequest::PATH => {
                    let resp = neutron_std::types::slinky::oracle::v1::GetPriceResponse {
                        price: Some(QuotePrice {
                            price: self.price.to_string(),
                            block_timestamp: None,
                            block_height: 0,
                        }),
                        nonce: 0,
                        decimals: 6,
                        id: 0,
                    }
                    .to_proto_bytes();
                    SystemResult::Ok(ContractResult::Ok(Binary::new(resp.to_vec())))
                }
                _ => unimplemented!(),
            },
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    fn new(base: MockQuerier) -> WasmMockQuerier {
        WasmMockQuerier {
            base,
            price: Default::default(),
        }
    }
}
