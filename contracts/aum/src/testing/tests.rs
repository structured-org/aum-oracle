use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetAumResponse, InstantiateMsg, QueryMsg, UpdateConfig};
use crate::state::{Config, CONFIG};
use cosmwasm_std::testing::{
    message_info, mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{
    from_json, to_json_binary, Addr, ContractResult, Int256, OwnedDeps, StdError, SystemResult,
    WasmQuery,
};

fn setup_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let owner = mock_dependencies().api.addr_make("owner");
    let oracle1 = mock_dependencies().api.addr_make("oracle1");

    let mut deps = mock_dependencies();
    let msg = InstantiateMsg {
        owner: owner.to_string(),
        oracles: vec![oracle1.to_string()],
    };
    let info = message_info(&owner, &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    deps
}

#[test]
fn proper_initialization() {
    let deps = setup_contract();

    let owner = mock_dependencies().api.addr_make("owner");
    let oracle1 = mock_dependencies().api.addr_make("oracle1");

    // Check if config is stored correctly
    let config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(config.owner, Addr::unchecked(owner));
    assert_eq!(config.oracles, vec![Addr::unchecked(oracle1)]);
}

#[test]
fn test_instantiate_with_invalid_owner() {
    let mut deps = mock_dependencies();

    let owner = mock_dependencies().api.addr_make("owner");
    let oracle1 = mock_dependencies().api.addr_make("oracle1");

    let msg = InstantiateMsg {
        owner: "invalid...address...".to_string(),
        oracles: vec![oracle1.to_string()],
    };
    let info = message_info(&owner, &[]);
    let err = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert!(matches!(err, StdError::GenericErr { .. }));
}

#[test]
fn test_instantiate_with_invalid_oracle() {
    let mut deps = mock_dependencies();

    let owner = mock_dependencies().api.addr_make("owner");

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        oracles: vec!["invalid...oracle...".to_string()],
    };
    let info = message_info(&owner, &[]);
    let err = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert!(matches!(err, StdError::GenericErr { .. }));
}

#[test]
fn update_config_by_owner() {
    let mut deps = setup_contract();

    let owner = mock_dependencies().api.addr_make("owner");
    let oracle1 = mock_dependencies().api.addr_make("oracle1");
    let oracle2 = mock_dependencies().api.addr_make("oracle2");
    let new_owner = mock_dependencies().api.addr_make("new_owner");

    // Update owner and oracles
    let msg = ExecuteMsg::UpdateConfig {
        new_config: UpdateConfig {
            owner: Some(new_owner.to_string()),
            oracles: Some(vec![oracle1.to_string(), oracle2.to_string()]),
        },
    };
    let info = message_info(&owner, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // Verify config update
    let config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(config.owner, Addr::unchecked(new_owner));
    assert_eq!(
        config.oracles,
        vec![Addr::unchecked(oracle1), Addr::unchecked(oracle2)]
    );
}

#[test]
fn update_config_by_unauthorized() {
    let mut deps = setup_contract();

    let owner = mock_dependencies().api.addr_make("owner");
    let oracle1 = mock_dependencies().api.addr_make("oracle1");
    let new_owner = mock_dependencies().api.addr_make("new_owner");

    // Try to update config with unauthorized user
    let msg = ExecuteMsg::UpdateConfig {
        new_config: UpdateConfig {
            owner: Some(new_owner.to_string()),
            oracles: None,
        },
    };
    let info = message_info(&oracle1, &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // Verify config remains unchanged
    let config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(config.owner, Addr::unchecked(owner));
    assert_eq!(config.oracles, vec![Addr::unchecked(oracle1)]);
}

#[test]
fn update_config_partial() {
    let mut deps = setup_contract();

    let owner = mock_dependencies().api.addr_make("owner");
    let oracle1 = mock_dependencies().api.addr_make("oracle1");
    let new_owner = mock_dependencies().api.addr_make("new_owner");

    // Update only owner
    let msg = ExecuteMsg::UpdateConfig {
        new_config: UpdateConfig {
            owner: Some(new_owner.to_string()),
            oracles: None,
        },
    };
    let info = message_info(&owner, &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Verify only owner was updated
    let config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(config.owner, Addr::unchecked(new_owner));
    assert_eq!(config.oracles, vec![Addr::unchecked(oracle1)]);
}

#[test]
fn query_config() {
    let deps = setup_contract();

    let owner = mock_dependencies().api.addr_make("owner");
    let oracle1 = mock_dependencies().api.addr_make("oracle1");

    // Query config
    let msg = QueryMsg::GetConfig {};
    let bin = query(deps.as_ref(), mock_env(), msg).unwrap();
    let config: Config = from_json(bin).unwrap();

    assert_eq!(config.owner, Addr::unchecked(owner));
    assert_eq!(config.oracles, vec![Addr::unchecked(oracle1)]);
}

#[test]
fn query_aum_single_oracle() {
    let mut deps = setup_contract();

    // Mock oracle response
    deps.querier.update_wasm(|query| match query {
        WasmQuery::Smart { msg: _, .. } => {
            let res = GetAumResponse {
                aum_in_btc: Int256::from(1000000i128), // 0.01 BTC
            };
            let bin = to_json_binary(&res).unwrap();
            SystemResult::Ok(ContractResult::Ok(bin))
        }
        _ => panic!("unexpected query"),
    });

    // Query AUM
    let msg = QueryMsg::GetAum {};
    let bin = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: GetAumResponse = from_json(bin).unwrap();

    assert_eq!(res.aum_in_btc, Int256::from(1000000i128));
}

#[test]
fn query_aum_multiple_oracles() {
    let mut deps = mock_dependencies();

    let owner = mock_dependencies().api.addr_make("owner");
    let oracle1 = mock_dependencies().api.addr_make("oracle1");
    let oracle2 = mock_dependencies().api.addr_make("oracle2");

    // Setup contract with two oracles
    let msg = InstantiateMsg {
        owner: owner.to_string(),
        oracles: vec![oracle1.to_string(), oracle2.to_string()],
    };
    let info = message_info(&owner, &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Mock oracle responses
    deps.querier.update_wasm(move |query| match query {
        WasmQuery::Smart {
            msg: _,
            contract_addr,
        } => {
            let res = if contract_addr.as_str() == oracle1.as_str() {
                GetAumResponse {
                    aum_in_btc: Int256::from(1000000i128), // 0.01 BTC
                }
            } else if contract_addr.as_str() == oracle2.as_str() {
                GetAumResponse {
                    aum_in_btc: Int256::from(2000000i128), // 0.02 BTC
                }
            } else {
                unreachable!()
            };

            let bin = to_json_binary(&res).unwrap();
            SystemResult::Ok(ContractResult::Ok(bin))
        }
        _ => panic!("unexpected query"),
    });

    // Query AUM
    let msg = QueryMsg::GetAum {};
    let bin = query(deps.as_ref(), mock_env(), msg).unwrap();
    let res: GetAumResponse = from_json(bin).unwrap();

    // Total should be 0.03 BTC (3000000 uwBTC)
    assert_eq!(res.aum_in_btc, Int256::from(3000000i128));
}

#[test]
fn query_aum_with_oracle_error() {
    let mut deps = setup_contract();

    // Mock oracle error response
    deps.querier.update_wasm(|query| match query {
        WasmQuery::Smart { msg: _, .. } => {
            SystemResult::Ok(ContractResult::Err("oracle error".to_string()))
        }
        _ => panic!("unexpected query"),
    });

    // Query AUM should fail
    let msg = QueryMsg::GetAum {};
    let err = query(deps.as_ref(), mock_env(), msg).unwrap_err();
    assert!(matches!(
        err,
        ContractError::Std(StdError::GenericErr { .. })
    ));
}
