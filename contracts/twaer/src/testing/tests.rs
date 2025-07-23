use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, GetAumResponse, GetTwaerResponse, InstantiateMsg, QueryMsg, UpdateConfig,
};
use crate::state::{Config, CONFIG, ER_HISTORY};
use crate::testing::mock_querier::mock_dependencies;
use cosmwasm_std::testing::{message_info, mock_env, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    from_json, to_json_binary, Addr, ContractResult, Decimal, Empty, Env, Order, OwnedDeps,
    StdError, SystemResult, Timestamp, Uint128, WasmQuery,
};

#[test]
fn proper_initialization() {
    let deps = setup_contract();

    let config = CONFIG.load(&deps.storage).unwrap();
    let expected_config = Config {
        owner: deps.api.addr_make("owner"),
        aum_oracles: vec![deps.api.addr_make("oracle1")],
        maxbtc_denom: "factory/neutron1/maxbtc".to_string(),
        twa_window_seconds: 86400,
    };
    assert_config_equals(&config, &expected_config);
}

#[test]
fn test_instantiate_with_invalid_owner() {
    let mut deps = mock_dependencies();
    let owner = deps.api.addr_make("owner");
    let oracle1 = deps.api.addr_make("oracle1");

    let msg = InstantiateMsg {
        owner: "invalid...address...".to_string(),
        aum_oracles: vec![oracle1.to_string()],
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
        aum_oracles: vec!["invalid...oracle...".to_string()],
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
            aum_oracles: Some(vec![oracle1.to_string(), oracle2.to_string()]),
            maxbtc_denom: Some("factory/neutron1/maxbtc".to_string()),
            twa_window_seconds: Some(172800), // 48 hours
        },
    };

    let res = execute_msg(&mut deps, mock_env(), &owner, msg).unwrap();
    assert_eq!(0, res.messages.len());

    let config = CONFIG.load(&deps.storage).unwrap();
    let expected_config = Config {
        owner: new_owner,
        aum_oracles: vec![oracle1, oracle2],
        maxbtc_denom: "factory/neutron1/maxbtc".to_string(),
        twa_window_seconds: 172800,
    };
    assert_config_equals(&config, &expected_config);
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
            aum_oracles: None,
            maxbtc_denom: None,
            twa_window_seconds: None,
        },
    };

    let err = execute_msg(&mut deps, mock_env(), &oracle1, msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    let config = CONFIG.load(&deps.storage).unwrap();
    let expected_config = Config {
        owner,
        aum_oracles: vec![oracle1],
        maxbtc_denom: "factory/neutron1/maxbtc".to_string(),
        twa_window_seconds: 86400,
    };
    assert_config_equals(&config, &expected_config);
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
            aum_oracles: None,
            maxbtc_denom: Some("factory/neutron1/minbtc".to_string()),
            twa_window_seconds: None,
        },
    };

    execute_msg(&mut deps, mock_env(), &owner, msg).unwrap();

    let config = CONFIG.load(&deps.storage).unwrap();
    let expected_config = Config {
        owner: new_owner,
        aum_oracles: vec![oracle1],
        maxbtc_denom: "factory/neutron1/minbtc".to_string(),
        twa_window_seconds: 86400,
    };
    assert_config_equals(&config, &expected_config);
}

#[test]
fn query_config() {
    let deps = setup_contract();
    let owner = deps.api.addr_make("owner");
    let oracle1 = deps.api.addr_make("oracle1");

    let bin = query_msg(&deps, mock_env(), QueryMsg::GetConfig {}).unwrap();
    let config: Config = from_json(bin).unwrap();

    let expected_config = Config {
        owner,
        aum_oracles: vec![oracle1],
        maxbtc_denom: "factory/neutron1/maxbtc".to_string(),
        twa_window_seconds: 86400,
    };
    assert_config_equals(&config, &expected_config);
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
fn test_record_er() {
    let mut deps = setup_contract_with_supply(500000u128, None);
    let owner = deps.api.addr_make("owner");

    deps.querier.update_wasm(mock_oracle_response(1000000u128));

    let env = test_env_with_time(1000000, 100);
    record_er(&mut deps, env.clone(), &owner).unwrap();

    let count = query_data_point_count(&deps).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_query_twaer_no_data() {
    let deps = setup_contract();

    let err = query_twaer(&deps, mock_env()).unwrap_err();
    assert!(matches!(err, ContractError::TwaerNotCalculated));
}

#[test]
fn test_record_er_cleanup() {
    let mut deps = setup_contract_with_supply(500000u128, None);
    let owner = deps.api.addr_make("owner");

    deps.querier.update_wasm(mock_oracle_response(1000000u128));

    // Store data point that will be outside the window
    let env1 = test_env_with_time(1000000, 100);
    record_er(&mut deps, env1, &owner).unwrap();

    // Move time forward beyond the TWA window (86400 seconds = 24 hours)
    let env2 = test_env_with_time(1000000 + 86401, 101);
    record_er(&mut deps, env2.clone(), &owner).unwrap();

    // Should only have 1 data point now (the old one was cleaned up)
    let count = query_data_point_count(&deps).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_twaer_long_time_period() {
    let mut deps = setup_contract_with_supply(1000000u128, Some(3600)); // 1 hour window
    let owner = deps.api.addr_make("owner");

    let base_time = 1000000u64;

    // === Data Point 1: t=0, Supply=1M, AUM=2M, Rate=2.0 ===
    let env1 = test_env_with_time(base_time, 100);
    deps.querier.update_wasm(mock_oracle_response(2000000u128));
    record_er(&mut deps, env1.clone(), &owner).unwrap();

    // Check TWA after Point 1: Only one point, so TWA = 2.0
    let twa_after_1 = query_predict_twaer(&deps, env1.clone()).unwrap();
    assert_eq!(twa_after_1, Decimal::from_ratio(2u128, 1u128)); // 2.0

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
    record_er(&mut deps, env2.clone(), &owner).unwrap();

    // Check TWA after Point 2: Rate 2.0 active for 900s, then Rate 1.6 at t=900
    // At t=900: TWA = (2.0*900) / 900 = 2.0 (Point 2 just added, no duration yet)
    let twa_after_2 = query_predict_twaer(&deps, env2.clone()).unwrap();
    assert_eq!(twa_after_2, Decimal::from_ratio(2u128, 1u128)); // 2.0

    // Check TWA at t=1350 (22.5 min): Rate 2.0 for 900s, Rate 1.6 for 450s
    // TWA = (2.0*900 + 1.6*450) / 1350 = (1800 + 720) / 1350 = 2520/1350 = 1.866...
    let env_mid_2 = test_env_with_time(base_time + 1350, 101);
    let twa_mid_2 = query_predict_twaer(&deps, env_mid_2).unwrap();
    assert_eq!(twa_mid_2, Decimal::from_ratio(2520u128, 1350u128)); // ≈ 1.866666...

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
    record_er(&mut deps, env3.clone(), &owner).unwrap();

    // Check TWA after Point 3: Rate 2.0 for 900s, Rate 1.6 for 900s, Rate 1.0 at t=1800
    // At t=1800: TWA = (2.0*900 + 1.6*900) / 1800 = (1800 + 1440) / 1800 = 3240/1800 = 1.8
    let twa_after_3 = query_predict_twaer(&deps, env3.clone()).unwrap();
    assert_eq!(twa_after_3, Decimal::from_ratio(18u128, 10u128)); // 1.8

    // Check TWA at t=2250 (37.5 min): Rate 2.0 for 900s, Rate 1.6 for 900s, Rate 1.0 for 450s
    // TWA = (2.0*900 + 1.6*900 + 1.0*450) / 2250 = (1800 + 1440 + 450) / 2250 = 3690/2250 = 1.64
    let env_mid_3 = test_env_with_time(base_time + 2250, 102);
    let twa_mid_3 = query_predict_twaer(&deps, env_mid_3).unwrap();
    assert_eq!(twa_mid_3, Decimal::from_ratio(3690u128, 2250u128)); // 1.64

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
    record_er(&mut deps, env4.clone(), &owner).unwrap();

    // Check TWA after Point 4: Rate 2.0 for 900s, Rate 1.6 for 900s, Rate 1.0 for 900s, Rate 1.0 at t=2700
    // At t=2700: TWA = (2.0*900 + 1.6*900 + 1.0*900) / 2700 = (1800 + 1440 + 900) / 2700 = 4140/2700 ≈ 1.533
    let twa_after_4 = query_predict_twaer(&deps, env4.clone()).unwrap();
    assert_eq!(twa_after_4, Decimal::from_ratio(4140u128, 2700u128)); // ≈ 1.533333...

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

    let predicted_twa = query_predict_twaer(&deps, env_final.clone()).unwrap();
    assert_eq!(predicted_twa, Decimal::from_ratio(14u128, 10u128));

    // Update and verify stored TWA matches prediction
    execute_msg(
        &mut deps,
        env_final.clone(),
        &owner,
        ExecuteMsg::PublishTwaer {},
    )
    .unwrap();
    let stored_twa = query_twaer(&deps, env_final.clone()).unwrap();
    assert_eq!(predicted_twa, stored_twa.twaer);

    // === Test historical data query ===
    let history_rates: Vec<Decimal> = ER_HISTORY
        .range(deps.as_ref().storage, None, None, Order::Ascending)
        .map(|item| item.unwrap().1)
        .collect();

    assert_eq!(history_rates.len(), 4);

    // Verify the rates are correct
    assert_eq!(history_rates[0], Decimal::from_ratio(2u128, 1u128)); // 2.0
    assert_eq!(history_rates[1], Decimal::from_ratio(8u128, 5u128)); // 1.6
    assert_eq!(history_rates[2], Decimal::from_ratio(1u128, 1u128)); // 1.0
    assert_eq!(history_rates[3], Decimal::from_ratio(1u128, 1u128)); // 1.0

    // === Test data cleanup - move beyond window ===
    let env_cleanup = test_env_with_time(base_time + 7200, 105); // 2 hours later
    record_er(&mut deps, env_cleanup.clone(), &owner).unwrap();

    // Should only have 1 data point now (old ones cleaned up)
    let count = query_data_point_count(&deps).unwrap();
    assert_eq!(count, 1);

    // Check TWA after cleanup: Only one data point remains at t=7200s with Rate=1.0
    // Since there's only one point with no duration history, TWA = 1.0
    let predicted_twa = query_predict_twaer(&deps, env_cleanup.clone()).unwrap();
    assert_eq!(predicted_twa, Decimal::one());
}

#[test]
fn test_twaer_complex_intertwining_expiration() {
    let mut deps = setup_contract_with_supply(1000000u128, Some(1800)); // 30 minute window
    let owner = deps.api.addr_make("owner");

    let base_time = 2000000u64;

    // === Phase 1: Build up initial data points ===

    // Point 1: t=0, Rate=3.0
    let env1 = test_env_with_time(base_time, 100);
    deps.querier.update_wasm(mock_oracle_response(3000000u128));
    record_er(&mut deps, env1.clone(), &owner).unwrap();

    // Check TWA after Point 1: Only one point, so TWA = 3.0
    let twa_after_1 = query_predict_twaer(&deps, env1.clone()).unwrap();
    assert_eq!(twa_after_1, Decimal::from_ratio(3u128, 1u128)); // 3.0

    // Point 2: t=300s (5 min), Rate=2.5
    let env2 = test_env_with_time(base_time + 300, 101);
    deps.querier = cosmwasm_std::testing::MockQuerier::<Empty>::new(&[(
        "",
        &[cosmwasm_std::Coin {
            denom: "factory/neutron1/maxbtc".to_string(),
            amount: Uint128::from(1200000u128),
        }],
    )]);
    deps.querier.update_wasm(mock_oracle_response(3000000u128));
    record_er(&mut deps, env2.clone(), &owner).unwrap();

    // Check TWA after Point 2: Rate 3.0 active for 300s, then Rate 2.5 at t=300
    // At t=300: TWA = (3.0*300) / 300 = 3.0 (Point 2 just added, no duration yet)
    let twa_after_2 = query_predict_twaer(&deps, env2.clone()).unwrap();
    assert_eq!(twa_after_2, Decimal::from_ratio(3u128, 1u128)); // 3.0

    // Point 3: t=600s (10 min), Rate=2.0
    let env3 = test_env_with_time(base_time + 600, 102);
    deps.querier = cosmwasm_std::testing::MockQuerier::<Empty>::new(&[(
        "",
        &[cosmwasm_std::Coin {
            denom: "factory/neutron1/maxbtc".to_string(),
            amount: Uint128::from(1500000u128),
        }],
    )]);
    deps.querier.update_wasm(mock_oracle_response(3000000u128));
    record_er(&mut deps, env3.clone(), &owner).unwrap();

    // Check TWA after Point 3: Rate 3.0 for 300s, Rate 2.5 for 300s, Rate 2.0 at t=600
    // At t=600: TWA = (3.0*300 + 2.5*300) / 600 = (900 + 750) / 600 = 1650/600 = 2.75
    let twa_after_3 = query_predict_twaer(&deps, env3.clone()).unwrap();
    assert_eq!(twa_after_3, Decimal::from_ratio(275u128, 100u128)); // 2.75

    // Verify we have 3 points
    assert_eq!(query_data_point_count(&deps).unwrap(), 3);

    // === Phase 2: Add points while first ones start expiring ===

    // Point 4: t=1200s (20 min), Rate=1.5 - still all points in window
    let env4 = test_env_with_time(base_time + 1200, 103);
    deps.querier = cosmwasm_std::testing::MockQuerier::<Empty>::new(&[(
        "",
        &[cosmwasm_std::Coin {
            denom: "factory/neutron1/maxbtc".to_string(),
            amount: Uint128::from(2000000u128),
        }],
    )]);
    deps.querier.update_wasm(mock_oracle_response(3000000u128));
    record_er(&mut deps, env4.clone(), &owner).unwrap();

    // Check TWA after Point 4: Rate 3.0 for 300s, Rate 2.5 for 300s, Rate 2.0 for 600s, Rate 1.5 at t=1200
    // At t=1200: TWA = (3.0*300 + 2.5*300 + 2.0*600) / 1200 = (900 + 750 + 1200) / 1200 = 2850/1200 = 2.375
    let twa_after_4 = query_predict_twaer(&deps, env4.clone()).unwrap();
    assert_eq!(twa_after_4, Decimal::from_ratio(2375u128, 1000u128)); // 2.375

    assert_eq!(query_data_point_count(&deps).unwrap(), 4);

    // Point 5: t=1900s (31.67 min), Rate=1.0 - Point 1 should expire (beyond 30 min window)
    let env5 = test_env_with_time(base_time + 1900, 104);
    deps.querier = cosmwasm_std::testing::MockQuerier::<Empty>::new(&[(
        "",
        &[cosmwasm_std::Coin {
            denom: "factory/neutron1/maxbtc".to_string(),
            amount: Uint128::from(3000000u128),
        }],
    )]);
    deps.querier.update_wasm(mock_oracle_response(3000000u128));
    record_er(&mut deps, env5.clone(), &owner).unwrap();

    // Check TWA after Point 5: Window is [100s, 1900s], Point 1 expired
    // Remaining: Point 2 (t=300) for 300s, Point 3 (t=600) for 600s, Point 4 (t=1200) for 700s, Point 5 (t=1900) at end
    // TWA = (2.5*300 + 2.0*600 + 1.5*700) / 1600 = (750 + 1200 + 1050) / 1600 = 3000/1600 = 1.875
    let twa_after_5 = query_predict_twaer(&deps, env5.clone()).unwrap();
    assert_eq!(twa_after_5, Decimal::from_ratio(1875u128, 1000u128)); // 1.875

    assert_eq!(query_data_point_count(&deps).unwrap(), 4);

    // === Phase 3: Rapid expiration phase ===

    // Point 6: t=2200s (36.67 min), Rate=0.8 - Points 2 should expire
    let env6 = test_env_with_time(base_time + 2200, 105);
    deps.querier = cosmwasm_std::testing::MockQuerier::<Empty>::new(&[(
        "",
        &[cosmwasm_std::Coin {
            denom: "factory/neutron1/maxbtc".to_string(),
            amount: Uint128::from(3750000u128),
        }],
    )]);
    deps.querier.update_wasm(mock_oracle_response(3000000u128));
    record_er(&mut deps, env6.clone(), &owner).unwrap();

    // Check TWA after Point 6: Window is [400s, 2200s], Point 2 expired
    // Remaining: Point 3 (t=600) for 600s, Point 4 (t=1200) for 700s, Point 5 (t=1900) for 300s, Point 6 (t=2200) at end
    // TWA = (2.0*600 + 1.5*700 + 1.0*300) / 1600 = (1200 + 1050 + 300) / 1600 = 2550/1600 = 1.59375
    let twa_after_6 = query_predict_twaer(&deps, env6.clone()).unwrap();
    assert_eq!(twa_after_6, Decimal::from_ratio(159375u128, 100000u128)); // 1.59375

    assert_eq!(query_data_point_count(&deps).unwrap(), 4);

    // Point 7: t=2500s (41.67 min), Rate=0.6 - Point 3 should expire
    let env7 = test_env_with_time(base_time + 2500, 106);
    deps.querier = cosmwasm_std::testing::MockQuerier::<Empty>::new(&[(
        "",
        &[cosmwasm_std::Coin {
            denom: "factory/neutron1/maxbtc".to_string(),
            amount: Uint128::from(5000000u128),
        }],
    )]);
    deps.querier.update_wasm(mock_oracle_response(3000000u128));
    record_er(&mut deps, env7.clone(), &owner).unwrap();

    // Check TWA after Point 7: Window is [700s, 2500s], Point 3 expired
    // Remaining: Point 4 (t=1200) for 700s, Point 5 (t=1900) for 300s, Point 6 (t=2200) for 300s, Point 7 (t=2500) at end
    // TWA = (1.5*700 + 1.0*300 + 0.8*300) / 1300 = (1050 + 300 + 240) / 1300 = 1590/1300 ≈ 1.223
    let twa_after_7 = query_predict_twaer(&deps, env7.clone()).unwrap();
    assert_eq!(twa_after_7, Decimal::from_ratio(1590u128, 1300u128)); // ≈ 1.223076923

    assert_eq!(query_data_point_count(&deps).unwrap(), 4);

    // === Phase 4: Test TWA calculation during complex transitions ===

    // Calculate TWA at t=2500s with current window [700s, 2500s]
    // Expected points in window:
    // - Point 4: t=1200s, Rate=1.5, active for 700s (1200 to 1900)
    // - Point 5: t=1900s, Rate=1.0, active for 300s (1900 to 2200)
    // - Point 6: t=2200s, Rate=0.8, active for 300s (2200 to 2500)
    // - Point 7: t=2500s, Rate=0.6, active for 0s (just added)
    // TWA = (1.5*700 + 1.0*300 + 0.8*300) / 1300 = (1050 + 300 + 240) / 1300 = 1590/1300 ≈ 1.223

    let predicted_twa = query_predict_twaer(&deps, env7.clone()).unwrap();
    let expected_twa = Decimal::from_ratio(1590u128, 1300u128); // ≈ 1.223076923
    assert_eq!(predicted_twa, expected_twa);

    // === Phase 5: Edge case - Multiple points expire at once ===

    // Jump far ahead to cause multiple expirations
    // Point 8: t=4000s (66.67 min), Rate=0.4 - Points 4,5,6 should all expire at once
    let env8 = test_env_with_time(base_time + 4000, 107);
    deps.querier = cosmwasm_std::testing::MockQuerier::<Empty>::new(&[(
        "",
        &[cosmwasm_std::Coin {
            denom: "factory/neutron1/maxbtc".to_string(),
            amount: Uint128::from(7500000u128),
        }],
    )]);
    deps.querier.update_wasm(mock_oracle_response(3000000u128));
    record_er(&mut deps, env8.clone(), &owner).unwrap();

    // Check TWA after Point 8: Window is [2200s, 4000s], Points 4,5 expired, Point 6 remains
    // Remaining: Point 6 (t=2200) for 300s, Point 7 (t=2500) for 1500s, Point 8 (t=4000) at end
    // TWA = (0.8*300 + 0.6*1500) / 1800 = (240 + 900) / 1800 = 1140/1800 ≈ 0.6333
    let twa_after_8 = query_predict_twaer(&deps, env8.clone()).unwrap();
    assert_eq!(twa_after_8, Decimal::from_ratio(1140u128, 1800u128)); // ≈ 0.6333

    // Should have 3 points now (Point 6, Point 7, and Point 8)
    assert_eq!(query_data_point_count(&deps).unwrap(), 3);

    // === Phase 6: Edge case - Empty window recovery ===

    // Jump very far ahead to expire everything
    let env9 = test_env_with_time(base_time + 6000, 108);
    deps.querier = cosmwasm_std::testing::MockQuerier::<Empty>::new(&[(
        "",
        &[cosmwasm_std::Coin {
            denom: "factory/neutron1/maxbtc".to_string(),
            amount: Uint128::from(5000000u128),
        }],
    )]);
    deps.querier.update_wasm(mock_oracle_response(2500000u128));
    record_er(&mut deps, env9.clone(), &owner).unwrap();

    // Check TWA after Point 9: Only one data point remains at t=6000s with Rate=0.5
    // Since there's only one point with no duration history, TWA = 0.5
    let twa_after_9 = query_predict_twaer(&deps, env9.clone()).unwrap();
    assert_eq!(twa_after_9, Decimal::from_ratio(1u128, 2u128)); // 0.5

    // Should have only 1 point (everything else expired)
    assert_eq!(query_data_point_count(&deps).unwrap(), 1);
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
    let config = ContractSetup::default();
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
        aum_oracles: oracle_addresses,
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
        aum_oracles: oracle_addresses,
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
        aum_oracles: vec![oracle1.to_string()],
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
fn assert_config_equals(config: &Config, expected_config: &Config) {
    assert_eq!(config.owner, expected_config.owner);
    assert_eq!(config.aum_oracles, expected_config.aum_oracles);
    assert_eq!(config.maxbtc_denom, expected_config.maxbtc_denom);
    assert_eq!(
        config.twa_window_seconds,
        expected_config.twa_window_seconds
    );
}

/// Create test environment with specific timestamp
fn test_env_with_time(timestamp: u64, block_height: u64) -> Env {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(timestamp);
    env.block.height = block_height;
    env
}

/// Record exchange rate helper
fn record_er<T>(
    deps: &mut OwnedDeps<MockStorage, MockApi, T>,
    env: Env,
    owner: &Addr,
) -> Result<cosmwasm_std::Response, ContractError>
where
    T: cosmwasm_std::Querier,
{
    execute_msg(deps, env, owner, ExecuteMsg::RecordEr {})
}

/// Query data point count helper - reads directly from state
fn query_data_point_count<T>(
    deps: &OwnedDeps<MockStorage, MockApi, T>,
) -> Result<u32, ContractError>
where
    T: cosmwasm_std::Querier,
{
    let count = ER_HISTORY
        .range(&deps.storage, None, None, Order::Ascending)
        .count() as u32;
    Ok(count)
}

/// Query TWA exchange rate helper
fn query_twaer<T>(
    deps: &OwnedDeps<MockStorage, MockApi, T>,
    env: Env,
) -> Result<GetTwaerResponse, ContractError>
where
    T: cosmwasm_std::Querier,
{
    let bin = query_msg(deps, env, QueryMsg::GetTwaer {})?;
    Ok(from_json(bin)?)
}

/// Query predict TWA exchange rate helper
fn query_predict_twaer<T>(
    deps: &OwnedDeps<MockStorage, MockApi, T>,
    env: Env,
) -> Result<Decimal, ContractError>
where
    T: cosmwasm_std::Querier,
{
    let bin = query_msg(deps, env, QueryMsg::PredictTwaer {})?;
    Ok(from_json(bin)?)
}
