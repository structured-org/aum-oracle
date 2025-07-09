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
    price_block_height: u64,
}

impl WasmMockQuerier {
    fn new(base: MockQuerier) -> WasmMockQuerier {
        WasmMockQuerier {
            base,
            price: Default::default(),
            price_block_height: 0,
        }
    }
    pub(crate) fn with_price_and_height(&mut self, price: impl Into<String>, height: u64) {
        self.price = price.into();
        self.price_block_height = height;
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
            QueryRequest::Grpc(GrpcQuery { data: _, path }) => match path.as_str() {
                neutron_std::types::slinky::oracle::v1::GetPriceRequest::PATH => {
                    let resp = neutron_std::types::slinky::oracle::v1::GetPriceResponse {
                        price: Some(QuotePrice {
                            price: self.price.clone(),
                            block_timestamp: None,
                            block_height: self.price_block_height,
                        }),
                        nonce: 0,
                        decimals: 6,
                        id: 0,
                    }
                    .to_proto_bytes();
                    SystemResult::Ok(ContractResult::Ok(Binary::from(resp)))
                }
                _ => unimplemented!(),
            },
            _ => self.base.handle_query(request),
        }
    }
}
