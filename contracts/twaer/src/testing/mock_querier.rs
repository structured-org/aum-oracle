use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    from_json, Binary, ContractResult, Empty, OwnedDeps, Querier, QuerierResult, QueryRequest,
    SystemError, SystemResult, WasmQuery,
};
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

/// A custom mock querier that can handle WASM queries.
pub struct WasmMockQuerier {
    base: MockQuerier,
    // Map from contract_addr => response binary for WASM queries
    wasm_responses: HashMap<String, Binary>,
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier {
            base,
            wasm_responses: HashMap::new(),
        }
    }

    fn handle_wasm_query(&self, wasm_query: &WasmQuery) -> SystemResult<ContractResult<Binary>> {
        match wasm_query {
            WasmQuery::Smart { contract_addr, .. } => {
                if let Some(response) = self.wasm_responses.get(contract_addr) {
                    SystemResult::Ok(ContractResult::Ok(response.clone()))
                } else {
                    SystemResult::Err(SystemError::UnsupportedRequest {
                        kind: format!("No mock response for contract: {}", contract_addr),
                    })
                }
            }
            _ => SystemResult::Err(SystemError::UnsupportedRequest {
                kind: "Unsupported WASM query".to_string(),
            }),
        }
    }

    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(wasm_query) => self.handle_wasm_query(wasm_query),
            _ => self.base.handle_query(request),
        }
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
