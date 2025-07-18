use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetAumResponse, InstantiateMsg, QueryMsg, UpdateConfig};
use crate::state::{Config, ExchangeRateDataPoint, CONFIG, EXCHANGE_RATE_HISTORY};
use crate::testing::mock_querier::mock_dependencies;
use cosmwasm_std::testing::{message_info, mock_env, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    from_json, to_json_binary, Addr, ContractResult, Decimal, Empty, Env, Order, OwnedDeps,
    StdError, SystemResult, Timestamp, Uint128, WasmQuery,
};

#[test]
fn proper_initialization() {
    let deps = setup_contract();
    let owner = deps.api.addr_make("owner");
    let oracle1 = deps.api.addr_make("oracle1");

    let config = CONFIG.load(&deps.storage).unwrap();
    assert_config_equals(&config, &owner, &[oracle1], 86400);
}

#[test]
fn test_instantiate_with_invalid_owner() {
    let mut deps = mock_dependencies();
    let owner = deps.api.addr_make("owner");
    let oracle1 = deps.api.addr_make("oracle1");

    let msg = InstantiateMsg {
        owner: "invalid...address...".to_string(),
        oracles: vec![oracle1.to_string()],
        maxbtc_denom: "factory/neutron1/maxbtc".to_string(),
        twa_window_seconds: 86400,
    };
    let info = message_info(&owner, &[]);
    let err = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert!(matches!(err, StdError::GenericErr { .. }));
}

#[test]
fn test_instantiate_with_invalid_oracle() {
    let mut deps = mock_dependencies();
    let owner = deps.api.addr_make("owner");

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        oracles: vec!["invalid...oracle...".to_string()],
        maxbtc_denom: "factory/neutron1/maxbtc".to_string(),
        twa_window_seconds: 86400,
    };
    let info = message_info(&owner, &[]);
    let err = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert!(matches!(err, StdError::GenericErr { .. }));
}

#[test]
fn update_config_by_owner() {
    let mut deps = setup_contract();
    let owner = deps.api.addr_make("owner");
    let oracle1 = deps.api.addr_make("oracle1");
    let oracle2 = deps.api.addr_make("oracle2");
    let new_owner = deps.api.addr_make("new_owner");

    let msg = ExecuteMsg::UpdateConfig {
        new_config: UpdateConfig {
            owner: Some(new_owner.to_string()),
            oracles: Some(vec![oracle1.to_string(), oracle2.to_string()]),
            twa_window_seconds: Some(172800), // 48 hours
        },
    };

    let res = execute_msg(&mut deps, mock_env(), &owner, msg).unwrap();
    assert_eq!(0, res.messages.len());

    let config = CONFIG.load(&deps.storage).unwrap();
    assert_config_equals(&config, &new_owner, &[oracle1, oracle2], 172800);
}

#[test]
fn update_config_by_unauthorized() {
    let mut deps = setup_contract();
    let owner = deps.api.addr_make("owner");
    let oracle1 = deps.api.addr_make("oracle1");
    let new_owner = deps.api.addr_make("new_owner");

    let msg = ExecuteMsg::UpdateConfig {
        new_config: UpdateConfig {
            owner: Some(new_owner.to_string()),
            oracles: None,
            twa_window_seconds: None,
        },
    };

    let err = execute_msg(&mut deps, mock_env(), &oracle1, msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    let config = CONFIG.load(&deps.storage).unwrap();
    assert_config_equals(&config, &owner, &[oracle1], 86400);
}

#[test]
fn update_config_partial() {
    let mut deps = setup_contract();
    let owner = deps.api.addr_make("owner");
    let oracle1 = deps.api.addr_make("oracle1");
    let new_owner = deps.api.addr_make("new_owner");

    let msg = ExecuteMsg::UpdateConfig {
        new_config: UpdateConfig {
            owner: Some(new_owner.to_string()),
            oracles: None,
            twa_window_seconds: None,
        },
    };

    execute_msg(&mut deps, mock_env(), &owner, msg).unwrap();

    let config = CONFIG.load(&deps.storage).unwrap();
    assert_config_equals(&config, &new_owner, &[oracle1], 86400);
}

#[test]
fn query_config() {
    let deps = setup_contract();
    let owner = deps.api.addr_make("owner");
    let oracle1 = deps.api.addr_make("oracle1");

    let bin = query_msg(&deps, mock_env(), QueryMsg::GetConfig {}).unwrap();
    let config: Config = from_json(bin).unwrap();

    assert_config_equals(&config, &owner, &[oracle1], 86400);
}

#[test]
fn query_aum_single_oracle() {
    let mut deps = setup_contract_with_standard_querier();
    deps.querier.update_wasm(mock_oracle_response(1000000u128));

    let bin = query_msg(&deps, mock_env(), QueryMsg::GetAum {}).unwrap();
    let res: GetAumResponse = from_json(bin).unwrap();

    assert_eq!(res.aum_in_btc, Uint128::from(1000000u128));
}

#[test]
fn query_aum_multiple_oracles() {
    let config = ContractSetup {
        oracles: vec!["oracle1".to_string(), "oracle2".to_string()],
        ..Default::default()
    };
    let mut deps = setup_contract_with_standard_querier_and_config(config);
    let oracle1 = deps.api.addr_make("oracle1");
    let oracle2 = deps.api.addr_make("oracle2");

    deps.querier.update_wasm(move |query| match query {
        WasmQuery::Smart {
            msg: _,
            contract_addr,
        } => {
            let res = if contract_addr.as_str() == oracle1.as_str() {
                GetAumResponse {
                    aum_in_btc: Uint128::from(1000000u128), // 0.01 BTC
                }
            } else if contract_addr.as_str() == oracle2.as_str() {
                GetAumResponse {
                    aum_in_btc: Uint128::from(2000000u128), // 0.02 BTC
                }
            } else {
                unreachable!()
            };
            let bin = to_json_binary(&res).unwrap();
            SystemResult::Ok(ContractResult::Ok(bin))
        }
        _ => panic!("unexpected query"),
    });

    let bin = query_msg(&deps, mock_env(), QueryMsg::GetAum {}).unwrap();
    let res: GetAumResponse = from_json(bin).unwrap();

    assert_eq!(res.aum_in_btc, Uint128::from(3000000u128)); // 0.03 BTC total
}

#[test]
fn query_aum_with_oracle_error() {
    let mut deps = setup_contract_with_standard_querier();
    deps.querier.update_wasm(mock_oracle_error());

    let err = query_msg(&deps, mock_env(), QueryMsg::GetAum {}).unwrap_err();
    assert!(matches!(
        err,
        ContractError::Std(StdError::GenericErr { .. })
    ));
}

#[test]
fn test_store_instant_exchange_rate() {
    let mut deps = setup_contract_with_supply(500000u128, None);
    let owner = deps.api.addr_make("owner");

    deps.querier.update_wasm(mock_oracle_response(1000000u128));

    let env = test_env_with_time(1000000, 100);
    store_instant_rate(&mut deps, env.clone(), &owner).unwrap();

    let count = query_data_point_count(&deps).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_twa_exchange_rate_no_data() {
    let deps = setup_contract();

    let err = query_twa_rate(&deps, mock_env()).unwrap_err();
    assert!(matches!(
        err,
        ContractError::Std(StdError::GenericErr { .. })
    ));
}

#[test]
fn test_twa_data_cleanup() {
    let mut deps = setup_contract_with_supply(500000u128, None);
    let owner = deps.api.addr_make("owner");

    deps.querier.update_wasm(mock_oracle_response(1000000u128));

    // Store data point that will be outside the window
    let env1 = test_env_with_time(1000000, 100);
    store_instant_rate(&mut deps, env1, &owner).unwrap();

    // Move time forward beyond the TWA window (86400 seconds = 24 hours)
    let env2 = test_env_with_time(1000000 + 86401, 101);
    store_instant_rate(&mut deps, env2.clone(), &owner).unwrap();

    // Should only have 1 data point now (the old one was cleaned up)
    let count = query_data_point_count(&deps).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_twa_exchange_rate_long_time_period() {
    let mut deps = setup_contract_with_supply(1000000u128, Some(3600)); // 1 hour window
    let owner = deps.api.addr_make("owner");

    let base_time = 1000000u64;

    // === Data Point 1: t=0, Supply=1M, AUM=2M, Rate=2.0 ===
    let env1 = test_env_with_time(base_time, 100);
    deps.querier.update_wasm(mock_oracle_response(2000000u128));
    store_instant_rate(&mut deps, env1, &owner).unwrap();

    // === Data Point 2: t=900s (15 min), Supply=1.5M, AUM=2.4M, Rate=1.6 ===
    let env2 = test_env_with_time(base_time + 900, 101);
    deps.querier = cosmwasm_std::testing::MockQuerier::<Empty>::new(&[(
        "",
        &[cosmwasm_std::Coin {
            denom: "factory/neutron1/maxbtc".to_string(),
            amount: Uint128::from(1500000u128),
        }],
    )]);
    deps.querier.update_wasm(mock_oracle_response(2400000u128));
    store_instant_rate(&mut deps, env2, &owner).unwrap();

    // === Data Point 3: t=1800s (30 min), Supply=2M, AUM=2M, Rate=1.0 ===
    let env3 = test_env_with_time(base_time + 1800, 102);
    deps.querier = cosmwasm_std::testing::MockQuerier::<Empty>::new(&[(
        "",
        &[cosmwasm_std::Coin {
            denom: "factory/neutron1/maxbtc".to_string(),
            amount: Uint128::from(2000000u128),
        }],
    )]);
    deps.querier.update_wasm(mock_oracle_response(2000000u128));
    store_instant_rate(&mut deps, env3, &owner).unwrap();

    // === Data Point 4: t=2700s (45 min), Supply=1.8M, AUM=1.8M, Rate=1.0 ===
    let env4 = test_env_with_time(base_time + 2700, 103);
    deps.querier = cosmwasm_std::testing::MockQuerier::<Empty>::new(&[(
        "",
        &[cosmwasm_std::Coin {
            denom: "factory/neutron1/maxbtc".to_string(),
            amount: Uint128::from(1800000u128),
        }],
    )]);
    deps.querier.update_wasm(mock_oracle_response(1800000u128));
    store_instant_rate(&mut deps, env4, &owner).unwrap();

    // === Calculate TWA at t=3600s (1 hour) ===
    let env_final = test_env_with_time(base_time + 3600, 104);

    // Verify we have 4 data points
    let count = query_data_point_count(&deps).unwrap();
    assert_eq!(count, 4);

    // Calculate expected TWA:
    // Rate 2.0 active for 900s (0 to 900)
    // Rate 1.6 active for 900s (900 to 1800)
    // Rate 1.0 active for 900s (1800 to 2700)
    // Rate 1.0 active for 900s (2700 to 3600)
    // TWA = (2.0*900 + 1.6*900 + 1.0*900 + 1.0*900) / 3600 = 1.4

    let predicted_twa = query_predicted_twa_rate(&deps, env_final.clone()).unwrap();
    assert_eq!(predicted_twa, Decimal::from_ratio(14u128, 10u128));

    // Update and verify stored TWA matches prediction
    execute_msg(
        &mut deps,
        env_final.clone(),
        &owner,
        ExecuteMsg::UpdateTwaExchangeRate {},
    )
    .unwrap();
    let stored_twa = query_twa_rate(&deps, env_final.clone()).unwrap();
    assert_eq!(predicted_twa, stored_twa);

    // === Test historical data query ===
    let data_points: Vec<ExchangeRateDataPoint> = EXCHANGE_RATE_HISTORY
        .range(deps.as_ref().storage, None, None, Order::Ascending)
        .map(|item| item.unwrap().1)
        .collect();

    assert_eq!(data_points.len(), 4);

    // Verify the rates are correct
    assert_eq!(data_points[0].rate, Decimal::from_ratio(2u128, 1u128)); // 2.0
    assert_eq!(data_points[1].rate, Decimal::from_ratio(8u128, 5u128)); // 1.6
    assert_eq!(data_points[2].rate, Decimal::from_ratio(1u128, 1u128)); // 1.0
    assert_eq!(data_points[3].rate, Decimal::from_ratio(1u128, 1u128)); // 1.0

    // === Test data cleanup - move beyond window ===
    let env_cleanup = test_env_with_time(base_time + 7200, 105); // 2 hours later
    store_instant_rate(&mut deps, env_cleanup.clone(), &owner).unwrap();

    // Should only have 1 data point now (old ones cleaned up)
    let count = query_data_point_count(&deps).unwrap();
    assert_eq!(count, 1);
}

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Contract setup configuration
struct ContractSetup {
    owner: String,
    oracles: Vec<String>,
    maxbtc_denom: String,
    twa_window_seconds: u64,
}

impl Default for ContractSetup {
    fn default() -> Self {
        Self {
            owner: "owner".to_string(),
            oracles: vec!["oracle1".to_string()],
            maxbtc_denom: "factory/neutron1/maxbtc".to_string(),
            twa_window_seconds: 86400, // 24 hours
        }
    }
}

/// Creates a contract with custom mock querier
fn setup_contract() -> OwnedDeps<MockStorage, MockApi, crate::testing::mock_querier::WasmMockQuerier>
{
    setup_contract_with_config(ContractSetup::default())
}

/// Creates a contract with custom configuration and custom mock querier
fn setup_contract_with_config(
    config: ContractSetup,
) -> OwnedDeps<MockStorage, MockApi, crate::testing::mock_querier::WasmMockQuerier> {
    let mut deps = mock_dependencies();
    let owner = deps.api.addr_make("owner");

    // Convert oracle names to addresses
    let oracle_addresses: Vec<String> = config
        .oracles
        .iter()
        .map(|name| deps.api.addr_make(name).to_string())
        .collect();

    let msg = InstantiateMsg {
        owner: if config.owner == "owner" {
            owner.to_string()
        } else {
            config.owner
        },
        oracles: oracle_addresses,
        maxbtc_denom: config.maxbtc_denom,
        twa_window_seconds: config.twa_window_seconds,
    };
    let info = message_info(&owner, &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    deps
}

/// Creates a contract with standard querier
fn setup_contract_with_standard_querier() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    setup_contract_with_standard_querier_and_config(ContractSetup::default())
}

/// Creates a contract with standard querier and custom configuration
fn setup_contract_with_standard_querier_and_config(
    config: ContractSetup,
) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = cosmwasm_std::testing::mock_dependencies();
    let owner = deps.api.addr_make("owner");

    // Convert oracle names to addresses
    let oracle_addresses: Vec<String> = config
        .oracles
        .iter()
        .map(|name| deps.api.addr_make(name).to_string())
        .collect();

    let msg = InstantiateMsg {
        owner: if config.owner == "owner" {
            owner.to_string()
        } else {
            config.owner
        },
        oracles: oracle_addresses,
        maxbtc_denom: config.maxbtc_denom,
        twa_window_seconds: config.twa_window_seconds,
    };
    let info = message_info(&owner, &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    deps
}

/// Creates a contract with supply data for exchange rate calculations
fn setup_contract_with_supply(
    supply_amount: u128,
    twa_window_seconds: Option<u64>,
) -> OwnedDeps<MockStorage, MockApi, MockQuerier<Empty>> {
    let mut deps = OwnedDeps {
        storage: cosmwasm_std::testing::MockStorage::default(),
        api: cosmwasm_std::testing::MockApi::default(),
        querier: cosmwasm_std::testing::MockQuerier::<Empty>::new(&[(
            "",
            &[cosmwasm_std::Coin {
                denom: "factory/neutron1/maxbtc".to_string(),
                amount: Uint128::from(supply_amount),
            }],
        )]),
        custom_query_type: std::marker::PhantomData,
    };

    let owner = deps.api.addr_make("owner");
    let oracle1 = deps.api.addr_make("oracle1");
    let msg = InstantiateMsg {
        owner: owner.to_string(),
        oracles: vec![oracle1.to_string()],
        maxbtc_denom: "factory/neutron1/maxbtc".to_string(),
        twa_window_seconds: twa_window_seconds.unwrap_or(86400),
    };
    let info = message_info(&owner, &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    deps
}

/// Mock oracle response helper
fn mock_oracle_response(
    aum_in_btc: u128,
) -> impl Fn(&WasmQuery) -> SystemResult<ContractResult<cosmwasm_std::Binary>> {
    move |query| match query {
        WasmQuery::Smart { msg: _, .. } => {
            let res = GetAumResponse {
                aum_in_btc: Uint128::from(aum_in_btc),
            };
            let bin = to_json_binary(&res).unwrap();
            SystemResult::Ok(ContractResult::Ok(bin))
        }
        _ => panic!("unexpected query"),
    }
}

/// Mock oracle error response helper
fn mock_oracle_error() -> impl Fn(&WasmQuery) -> SystemResult<ContractResult<cosmwasm_std::Binary>>
{
    |query| match query {
        WasmQuery::Smart { msg: _, .. } => {
            SystemResult::Ok(ContractResult::Err("oracle error".to_string()))
        }
        _ => panic!("unexpected query"),
    }
}

/// Execute message helper
fn execute_msg<T>(
    deps: &mut OwnedDeps<MockStorage, MockApi, T>,
    env: Env,
    sender: &Addr,
    msg: ExecuteMsg,
) -> Result<cosmwasm_std::Response, ContractError>
where
    T: cosmwasm_std::Querier,
{
    let info = message_info(sender, &[]);
    execute(deps.as_mut(), env, info, msg)
}

/// Query helper
fn query_msg<T>(
    deps: &OwnedDeps<MockStorage, MockApi, T>,
    env: Env,
    msg: QueryMsg,
) -> Result<cosmwasm_std::Binary, ContractError>
where
    T: cosmwasm_std::Querier,
{
    query(deps.as_ref(), env, msg)
}

/// Config verification helper
fn assert_config_equals(
    config: &Config,
    expected_owner: &Addr,
    expected_oracles: &[Addr],
    expected_twa_window: u64,
) {
    assert_eq!(config.owner, *expected_owner);
    assert_eq!(config.oracles, expected_oracles);
    assert_eq!(config.twa_window_seconds, expected_twa_window);
}

/// Create test environment with specific timestamp
fn test_env_with_time(timestamp: u64, block_height: u64) -> Env {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(timestamp);
    env.block.height = block_height;
    env
}

/// Store instant exchange rate helper
fn store_instant_rate<T>(
    deps: &mut OwnedDeps<MockStorage, MockApi, T>,
    env: Env,
    owner: &Addr,
) -> Result<cosmwasm_std::Response, ContractError>
where
    T: cosmwasm_std::Querier,
{
    execute_msg(deps, env, owner, ExecuteMsg::StoreInstantExchangeRate {})
}

/// Query data point count helper - reads directly from state
fn query_data_point_count<T>(
    deps: &OwnedDeps<MockStorage, MockApi, T>,
) -> Result<u32, ContractError>
where
    T: cosmwasm_std::Querier,
{
    let count = EXCHANGE_RATE_HISTORY
        .range(&deps.storage, None, None, Order::Ascending)
        .count() as u32;
    Ok(count)
}

/// Query TWA exchange rate helper
fn query_twa_rate<T>(
    deps: &OwnedDeps<MockStorage, MockApi, T>,
    env: Env,
) -> Result<Decimal, ContractError>
where
    T: cosmwasm_std::Querier,
{
    let bin = query_msg(deps, env, QueryMsg::GetTwaExchangeRate {})?;
    Ok(from_json(bin)?)
}

/// Query predicted TWA exchange rate helper
fn query_predicted_twa_rate<T>(
    deps: &OwnedDeps<MockStorage, MockApi, T>,
    env: Env,
) -> Result<Decimal, ContractError>
where
    T: cosmwasm_std::Querier,
{
    let bin = query_msg(deps, env, QueryMsg::PredictTwaExchangeRate {})?;
    Ok(from_json(bin)?)
}
