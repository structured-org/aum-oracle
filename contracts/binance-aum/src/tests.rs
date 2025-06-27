use crate::contract::*;
use crate::msg::{ExecuteMsg, GetDataResponse, QueryMsg};
use crate::state::{BinanceData, Config, Position, SpotBalance, CONFIG};
use consensus::consensus::{Config as ConsensusConfig, ConsensusData, OracleData, Round, State};
use cosmwasm_std::{
    from_json,
    testing::{mock_dependencies, mock_env},
    Addr, Coin, Deps, DepsMut, Env, MessageInfo, SignedDecimal, StdError, Timestamp,
};

// Helper function to create a MessageInfo object for testing
fn message_info(sender: &str, funds: &[Coin]) -> MessageInfo {
    MessageInfo {
        sender: Addr::unchecked(sender),
        funds: funds.to_vec(),
    }
}

// Helper function to create a test consensus config
fn create_test_consensus_config() -> ConsensusConfig {
    ConsensusConfig {
        oracles: vec![
            Addr::unchecked("oracle1"),
            Addr::unchecked("oracle2"),
            Addr::unchecked("oracle3"),
        ],
        threshold: 2,
        data_delta_ppm: 10000, // 1%
        round_length: 3600,    // 1 hour
    }
}

// Helper function to create a test contract config
fn create_test_contract_config() -> Config {
    Config {
        admin: Addr::unchecked("admin"),
        valid_period: 100,
        required_binance_positions: vec!["BTCUSDT".to_string()],
        required_binance_spot_assets: vec!["BTC".to_string(), "USDT".to_string()],
    }
}

// Helper function to create a test BinanceData object
fn create_test_data(
    round: u64,
    timestamp: u64,
    unimmr: SignedDecimal,
    um_balance: SignedDecimal,
    pm_equity: SignedDecimal,
    withdrawable: SignedDecimal,
) -> OracleData<BinanceData> {
    OracleData {
        round,
        timestamp,
        data: BinanceData {
            unimmr,
            positions: vec![Position {
                symbol: "BTCUSDT".to_string(),
                amount: SignedDecimal::from_ratio(1, 1),
                pnl: SignedDecimal::from_ratio(100, 1),
            }],
            um_balance_usdt: um_balance,
            spot_balances: vec![
                SpotBalance {
                    asset: "BTC".to_string(),
                    amount: SignedDecimal::from_ratio(1, 1),
                },
                SpotBalance {
                    asset: "USDT".to_string(),
                    amount: SignedDecimal::from_ratio(10000, 1),
                },
            ],
            pm_account_actual_equity: pm_equity,
            withdrawable_usdt: withdrawable,
        },
    }
}

// Helper function to setup storage with config and current round
fn setup_test_state(
    deps: &mut DepsMut,
    config: &Config,
    consensus_config: &ConsensusConfig,
    round: u64,
    start_time: u64,
) {
    let state: State<BinanceData> = State::default();
    state.config.save(deps.storage, consensus_config).unwrap();
    state
        .pending_round
        .save(
            deps.storage,
            &Round {
                round,
                start: start_time,
            },
        )
        .unwrap();

    CONFIG.save(deps.storage, config).unwrap();
}

fn query_last_published_data(deps: Deps, env: Env) -> GetDataResponse {
    from_json(query(deps, env, QueryMsg::GetData {}).unwrap()).unwrap()
}

#[test]
fn test_execute_publish_data() {
    // Set up test environment
    let mut deps = mock_dependencies();
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(1000);

    // Set up initial state
    let mut consensus_config = create_test_consensus_config();
    let contract_config = create_test_contract_config();
    // Modify the config to use threshold instead of all oracles for consensus
    consensus_config.threshold = 2;
    let round = Round {
        round: 1,
        start: 1000,
    };
    setup_test_state(
        &mut deps.as_mut(),
        &contract_config,
        &consensus_config,
        round.round,
        round.start,
    );

    // Create test data
    let test_data = create_test_data(
        1,
        1000,
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(1000, 1),
        SignedDecimal::from_ratio(2000, 1),
        SignedDecimal::from_ratio(500, 1),
    );

    // Test 1: Unauthorized oracle rejection
    let unauthorized_info = message_info("unauthorized", &[]);
    let msg = ExecuteMsg::PublishData {
        new_data: test_data.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), unauthorized_info, msg);
    assert!(result.is_err());
    match result.unwrap_err() {
        StdError::GenericErr { msg, .. } => assert_eq!(msg, "Unauthorized oracle"),
        _ => panic!("Unexpected error"),
    }

    // Test 2: Invalid positions
    let oracle_info = message_info("oracle1", &[]);
    let mut invalid_positions_data = create_test_data(
        2, // Wrong round
        1000,
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(1000, 1),
        SignedDecimal::from_ratio(2000, 1),
        SignedDecimal::from_ratio(500, 1),
    );
    invalid_positions_data.data.positions = vec![
        Position {
            symbol: "WRONG_SYMBOL".to_string(),
            amount: SignedDecimal::from_ratio(1, 1),
            pnl: SignedDecimal::from_ratio(100, 1),
        },
        Position {
            symbol: "ANOTHER_WRONG_SYMBOL".to_string(),
            amount: SignedDecimal::from_ratio(1, 1),
            pnl: SignedDecimal::from_ratio(100, 1),
        },
    ];

    let msg = ExecuteMsg::PublishData {
        new_data: invalid_positions_data,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle_info.clone(), msg);
    assert!(result.is_err());
    match result.unwrap_err() {
        StdError::GenericErr { msg, .. } => {
            assert_eq!(msg, "Binance positions do not match required positions")
        }
        _ => panic!("Unexpected error"),
    }

    // Test 2: Invalid spot balances
    let oracle_info = message_info("oracle1", &[]);
    let mut invalid_spot_balances_data = create_test_data(
        2, // Wrong round
        1000,
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(1000, 1),
        SignedDecimal::from_ratio(2000, 1),
        SignedDecimal::from_ratio(500, 1),
    );
    invalid_spot_balances_data.data.spot_balances = vec![
        SpotBalance {
            asset: "WRONG_ASSET".to_string(),
            amount: SignedDecimal::from_ratio(1, 1),
        },
        SpotBalance {
            asset: "ANOTHER_WRONG_ASSET".to_string(),
            amount: SignedDecimal::from_ratio(10000, 1),
        },
    ];

    let msg = ExecuteMsg::PublishData {
        new_data: invalid_spot_balances_data,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle_info.clone(), msg);
    assert!(result.is_err());
    match result.unwrap_err() {
        StdError::GenericErr { msg, .. } => {
            assert_eq!(msg, "Binance spot assets do not match required spot assets")
        }
        _ => panic!("Unexpected error"),
    }

    // Test 2: Invalid round rejection
    let oracle_info = message_info("oracle1", &[]);
    let invalid_round_data = create_test_data(
        2, // Wrong round
        1000,
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(1000, 1),
        SignedDecimal::from_ratio(2000, 1),
        SignedDecimal::from_ratio(500, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: invalid_round_data,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle_info.clone(), msg);
    assert!(result.is_err());
    match result.unwrap_err() {
        StdError::GenericErr { msg, .. } => assert_eq!(msg, "Invalid round"),
        _ => panic!("Unexpected error"),
    }

    // Test 3: Valid submission acceptance
    let msg = ExecuteMsg::PublishData {
        new_data: test_data.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), oracle_info.clone(), msg);
    assert!(result.is_ok());

    // Test 4: Submit data from all oracles to reach consensus
    let oracle2_info = message_info("oracle2", &[]);
    let mut test_data2 = create_test_data(
        1,
        1001,
        SignedDecimal::from_ratio(505, 1000),
        SignedDecimal::from_ratio(1005, 1),
        SignedDecimal::from_ratio(2010, 1),
        SignedDecimal::from_ratio(505, 1),
    );
    // BinanceData with additional wrong positions and spot balances must be accepted anyway, since
    // the data is being cleaned
    test_data2.data.positions.append(&mut vec![
        Position {
            symbol: "WRONG_SYMBOL".to_string(),
            amount: SignedDecimal::from_ratio(1, 1),
            pnl: SignedDecimal::from_ratio(100, 1),
        },
        Position {
            symbol: "ANOTHER_WRONG_SYMBOL".to_string(),
            amount: SignedDecimal::from_ratio(1, 1),
            pnl: SignedDecimal::from_ratio(100, 1),
        },
    ]);
    test_data2.data.spot_balances.append(&mut vec![
        SpotBalance {
            asset: "WRONG_ASSET".to_string(),
            amount: SignedDecimal::from_ratio(1, 1),
        },
        SpotBalance {
            asset: "ANOTHER_WRONG_ASSET".to_string(),
            amount: SignedDecimal::from_ratio(10000, 1),
        },
    ]);

    let msg = ExecuteMsg::PublishData {
        new_data: test_data2,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle2_info, msg);
    assert!(result.is_ok());

    // Submit data from the third oracle to reach consensus (all oracles)
    let oracle3_info = message_info("oracle3", &[]);
    let test_data3 = create_test_data(
        1,
        1002,
        SignedDecimal::from_ratio(503, 1000),
        SignedDecimal::from_ratio(1003, 1),
        SignedDecimal::from_ratio(2005, 1),
        SignedDecimal::from_ratio(503, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data3,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle3_info, msg);
    assert!(result.is_ok());

    // Verify consensus was reached, but round is still the same
    let response = result.unwrap();
    let publish_consensus_attr = response
        .attributes
        .iter()
        .find(|attr| attr.key == "action" && attr.value == "publish_consensus");
    assert!(publish_consensus_attr.is_some());

    let round_attr = response.attributes.iter().find(|attr| attr.key == "round");
    assert!(round_attr.is_some());
    assert_eq!(round_attr.unwrap().value, "1");

    // Test 5: Verify that oracles can't publish data for already finalized rounds
    // After consensus is reached, attempting to submit data for the same round should fail
    let oracle1_info = message_info("oracle1", &[]);
    let new_data = create_test_data(
        1, // Same round
        1003,
        SignedDecimal::from_ratio(503, 1000),
        SignedDecimal::from_ratio(1003, 1),
        SignedDecimal::from_ratio(2005, 1),
        SignedDecimal::from_ratio(503, 1),
    );
    let msg = ExecuteMsg::PublishData { new_data };
    let result = execute(deps.as_mut(), env.clone(), oracle1_info, msg);
    assert!(result.is_err());
    match result.unwrap_err() {
        StdError::GenericErr { msg, .. } => assert_eq!(msg, "Invalid round"),
        _ => panic!("Unexpected error"),
    };

    // Test 6: Advance the round by time expiration
    // Update the block time to after the round expiration
    env.block.time = Timestamp::from_seconds(1000 + consensus_config.round_length + 1);

    // Submit data for the new round (which should be 2 now)
    let oracle2_info = message_info("oracle2", &[]);
    let new_round_data = create_test_data(
        2, // New round
        1000 + consensus_config.round_length + 1,
        SignedDecimal::from_ratio(503, 1000),
        SignedDecimal::from_ratio(1003, 1),
        SignedDecimal::from_ratio(2005, 1),
        SignedDecimal::from_ratio(503, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: new_round_data,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle2_info, msg);
    assert!(result.is_ok());

    // Verify the round has advanced
    let response = result.unwrap();
    let round_attr = response.attributes.iter().find(|attr| attr.key == "round");
    assert!(round_attr.is_some());
    assert_eq!(round_attr.unwrap().value, "2");
}

#[test]
fn test_execute_publish_data_time_based_consensus() {
    // Set up test environment
    let mut deps = mock_dependencies();
    let mut env = mock_env();
    let start_time = 1000;
    env.block.time = Timestamp::from_seconds(start_time);

    // Set up initial state with a custom config
    let mut consensus_config = create_test_consensus_config();
    let contract_config = create_test_contract_config();
    // Modify the config to use threshold instead of all oracles for consensus
    consensus_config.threshold = 2;
    let round = Round {
        round: 1,
        start: start_time,
    };
    setup_test_state(
        &mut deps.as_mut(),
        &contract_config,
        &consensus_config,
        round.round,
        round.start,
    );

    // Create test data
    let test_data1 = create_test_data(
        1,
        start_time,
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(1000, 1),
        SignedDecimal::from_ratio(2000, 1),
        SignedDecimal::from_ratio(500, 1),
    );

    // Test 1: Submit data from one oracle
    let oracle1_info = message_info("oracle1", &[]);
    let msg = ExecuteMsg::PublishData {
        new_data: test_data1,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle1_info, msg);
    assert!(result.is_ok());

    // Test 2: No consensus yet (only one oracle submitted)
    let response = result.unwrap();
    // Check that no consensus was reached (no publish_consensus action)
    let publish_consensus_attr = response
        .attributes
        .iter()
        .find(|attr| attr.key == "action" && attr.value == "publish_consensus");
    assert!(publish_consensus_attr.is_none());

    // Test 3: Submit data from another oracle with identical data
    let test_data2 = create_test_data(
        1, // Same round
        start_time + 1,
        SignedDecimal::from_ratio(5, 10), // Identical to test_data1
        SignedDecimal::from_ratio(1000, 1), // Identical to test_data1
        SignedDecimal::from_ratio(2000, 1), // Identical to test_data1
        SignedDecimal::from_ratio(500, 1), // Identical to test_data1
    );
    let oracle2_info = message_info("oracle2", &[]);
    let msg = ExecuteMsg::PublishData {
        new_data: test_data2,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle2_info, msg);
    assert!(result.is_ok());

    // Test 4: No consensus yet (only two oracles submitted)
    let response = result.unwrap();

    let publish_consensus_attr = response
        .attributes
        .iter()
        .find(|attr| attr.key == "action" && attr.value == "publish_consensus");
    assert!(publish_consensus_attr.is_none());

    // Test 5: Submit data from the third oracle with identical data
    let test_data3 = create_test_data(
        1, // Same round
        start_time + 2,
        SignedDecimal::from_ratio(5, 10), // Identical to test_data1 and test_data2
        SignedDecimal::from_ratio(1000, 1), // Identical to test_data1 and test_data2
        SignedDecimal::from_ratio(2000, 1), // Identical to test_data1 and test_data2
        SignedDecimal::from_ratio(500, 1), // Identical to test_data1 and test_data2
    );
    let oracle3_info = message_info("oracle3", &[]);
    let msg = ExecuteMsg::PublishData {
        new_data: test_data3,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle3_info, msg);
    assert!(result.is_ok());

    // Test 6: Consensus should be reached (all oracles submitted identical data)
    let response = result.unwrap();

    let publish_consensus_attr = response
        .attributes
        .iter()
        .find(|attr| attr.key == "action" && attr.value == "publish_consensus");
    assert!(publish_consensus_attr.is_some());
}

#[test]
fn test_try_consensus() {
    // Test 1: Empty data array should return None
    {
        let config = create_test_consensus_config();
        let empty_data: Vec<BinanceData> = vec![];
        assert!(BinanceData::try_consensus(
            &empty_data,
            config.threshold as usize,
            config.data_delta_ppm
        )
        .is_none());
    }

    // Test 2: Data array with fewer elements than threshold should return None
    {
        let config = create_test_consensus_config();
        let single_data = vec![
            create_test_data(
                1,
                1000,
                SignedDecimal::from_ratio(5, 10),
                SignedDecimal::from_ratio(1000, 1),
                SignedDecimal::from_ratio(2000, 1),
                SignedDecimal::from_ratio(500, 1),
            )
            .data,
        ];
        assert!(BinanceData::try_consensus(
            &single_data,
            config.threshold as usize,
            config.data_delta_ppm
        )
        .is_none());
    }

    // Test 3: Data within acceptable delta should reach consensus
    {
        let config = create_test_consensus_config();
        let data = vec![
            create_test_data(
                1,
                1000,
                SignedDecimal::from_ratio(5, 10),
                SignedDecimal::from_ratio(1000, 1),
                SignedDecimal::from_ratio(2000, 1),
                SignedDecimal::from_ratio(500, 1),
            )
            .data,
            create_test_data(
                1,
                1001,
                SignedDecimal::from_ratio(505, 1000),
                SignedDecimal::from_ratio(1005, 1),
                SignedDecimal::from_ratio(2010, 1),
                SignedDecimal::from_ratio(505, 1),
            )
            .data, // within 1% delta
        ];
        let consensus =
            BinanceData::try_consensus(&data, config.threshold as usize, config.data_delta_ppm);
        assert!(consensus.is_some());

        let result = consensus.unwrap();
        assert_eq!(result.unimmr, SignedDecimal::from_ratio(5025, 10000)); // median of 0.5 and 0.505
        assert_eq!(result.um_balance_usdt, SignedDecimal::from_ratio(10025, 10)); // median of 1000 and 1005
        assert_eq!(
            result.pm_account_actual_equity,
            SignedDecimal::from_ratio(2005, 1)
        ); // median of 2000 and 2010
        assert_eq!(
            result.withdrawable_usdt,
            SignedDecimal::from_ratio(5025, 10)
        ); // median of 500 and 505
    }

    // Test 4: Data outside acceptable delta should not reach consensus
    {
        let mut config = create_test_consensus_config();
        config.data_delta_ppm = 1000; // 0.1% delta

        let data_divergent = vec![
            create_test_data(
                1,
                1000,
                SignedDecimal::from_ratio(5, 10),
                SignedDecimal::from_ratio(1000, 1),
                SignedDecimal::from_ratio(2000, 1),
                SignedDecimal::from_ratio(500, 1),
            )
            .data,
            create_test_data(
                1,
                1001,
                SignedDecimal::from_ratio(51, 100),
                SignedDecimal::from_ratio(1020, 1),
                SignedDecimal::from_ratio(2050, 1),
                SignedDecimal::from_ratio(510, 1),
            )
            .data, // outside 0.1% delta
        ];

        assert!(BinanceData::try_consensus(
            &data_divergent,
            config.threshold as usize,
            config.data_delta_ppm
        )
        .is_none());
    }

    // Test 5: Consensus with more than threshold oracles
    {
        let config = create_test_consensus_config();
        let data_multiple = vec![
            create_test_data(
                1,
                1000,
                SignedDecimal::from_ratio(5, 10),
                SignedDecimal::from_ratio(1000, 1),
                SignedDecimal::from_ratio(2000, 1),
                SignedDecimal::from_ratio(500, 1),
            )
            .data,
            create_test_data(
                1,
                1001,
                SignedDecimal::from_ratio(505, 1000),
                SignedDecimal::from_ratio(1005, 1),
                SignedDecimal::from_ratio(2010, 1),
                SignedDecimal::from_ratio(505, 1),
            )
            .data,
            create_test_data(
                1,
                1002,
                SignedDecimal::from_ratio(503, 1000),
                SignedDecimal::from_ratio(1003, 1),
                SignedDecimal::from_ratio(2005, 1),
                SignedDecimal::from_ratio(503, 1),
            )
            .data,
        ];

        let consensus = BinanceData::try_consensus(
            &data_multiple,
            config.threshold as usize,
            config.data_delta_ppm,
        );
        assert!(consensus.is_some());

        let result = consensus.unwrap();
        assert_eq!(result.unimmr, SignedDecimal::from_ratio(503, 1000));
        assert_eq!(result.um_balance_usdt, SignedDecimal::from_ratio(1003, 1));
        assert_eq!(
            result.pm_account_actual_equity,
            SignedDecimal::from_ratio(2005, 1)
        );
        assert_eq!(result.withdrawable_usdt, SignedDecimal::from_ratio(503, 1));
    }

    // Test 6: Consensus with some outliers
    {
        let config = create_test_consensus_config();
        let data_with_outliers = vec![
            create_test_data(
                1,
                1000,
                SignedDecimal::from_ratio(5, 10),
                SignedDecimal::from_ratio(1000, 1),
                SignedDecimal::from_ratio(2000, 1),
                SignedDecimal::from_ratio(500, 1),
            )
            .data,
            create_test_data(
                1,
                1001,
                SignedDecimal::from_ratio(505, 1000),
                SignedDecimal::from_ratio(1005, 1),
                SignedDecimal::from_ratio(2010, 1),
                SignedDecimal::from_ratio(505, 1),
            )
            .data,
            create_test_data(
                1,
                1002,
                SignedDecimal::from_ratio(6, 10),
                SignedDecimal::from_ratio(1200, 1),
                SignedDecimal::from_ratio(2500, 1),
                SignedDecimal::from_ratio(600, 1),
            )
            .data, // outlier
        ];

        let consensus = BinanceData::try_consensus(
            &data_with_outliers,
            config.threshold as usize,
            config.data_delta_ppm,
        );
        assert!(consensus.is_some());

        // The outlier should be excluded from the consensus
        let result = consensus.unwrap();
        assert_eq!(result.unimmr, SignedDecimal::from_ratio(5025, 10000)); // median of 0.5 and 0.505 (outlier excluded)
    }

    // Test 7: Consensus with multiple positions
    {
        let config = create_test_consensus_config();
        let mut data1 = create_test_data(
            1,
            1000,
            SignedDecimal::from_ratio(5, 10),
            SignedDecimal::from_ratio(1000, 1),
            SignedDecimal::from_ratio(2000, 1),
            SignedDecimal::from_ratio(500, 1),
        );
        let mut data2 = create_test_data(
            1,
            1001,
            SignedDecimal::from_ratio(505, 1000),
            SignedDecimal::from_ratio(1005, 1),
            SignedDecimal::from_ratio(2010, 1),
            SignedDecimal::from_ratio(505, 1),
        );
        //outlier
        let mut data3 = create_test_data(
            1,
            1002,
            SignedDecimal::from_ratio(6, 10),
            SignedDecimal::from_ratio(1200, 1),
            SignedDecimal::from_ratio(2500, 1),
            SignedDecimal::from_ratio(600, 1),
        );
        let data_with_outliers = vec![data1.data, data2.data, data3.data];

        let consensus = BinanceData::try_consensus(
            &data_with_outliers,
            config.threshold as usize,
            config.data_delta_ppm,
        );
        assert!(consensus.is_some());

        // The outlier should be excluded from the consensus
        let result = consensus.unwrap();
        assert_eq!(result.unimmr, SignedDecimal::from_ratio(5025, 10000)); // median of 0.5 and 0.505 (outlier excluded)
    }
}

#[test]
fn test_all_oracles_consensus_round_not_increased() {
    // Set up test environment
    let mut deps = mock_dependencies();
    let mut env = mock_env();
    let start_time = 1000;
    env.block.time = Timestamp::from_seconds(start_time);

    // Set up initial state
    let consensus_config = create_test_consensus_config();
    let contract_config = create_test_contract_config();
    let round = Round {
        round: 1,
        start: start_time,
    };
    setup_test_state(
        &mut deps.as_mut(),
        &contract_config,
        &consensus_config,
        round.round,
        round.start,
    );

    // Submit data from all oracles
    let oracle1_info = message_info("oracle1", &[]);
    let test_data1 = create_test_data(
        1,
        start_time,
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(1000, 1),
        SignedDecimal::from_ratio(2000, 1),
        SignedDecimal::from_ratio(500, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data1,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle1_info, msg);
    assert!(result.is_ok());

    let oracle2_info = message_info("oracle2", &[]);
    let test_data2 = create_test_data(
        1,
        start_time + 1,
        SignedDecimal::from_ratio(505, 1000),
        SignedDecimal::from_ratio(1005, 1),
        SignedDecimal::from_ratio(2010, 1),
        SignedDecimal::from_ratio(505, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data2,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle2_info, msg);
    assert!(result.is_ok());

    let oracle3_info = message_info("oracle3", &[]);
    let test_data3 = create_test_data(
        1,
        start_time + 2,
        SignedDecimal::from_ratio(503, 1000),
        SignedDecimal::from_ratio(1003, 1),
        SignedDecimal::from_ratio(2005, 1),
        SignedDecimal::from_ratio(503, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data3,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle3_info, msg);
    assert!(result.is_ok());

    // Verify consensus was reached
    let response = result.unwrap();
    let publish_consensus_attr = response
        .attributes
        .iter()
        .find(|attr| attr.key == "action" && attr.value == "publish_consensus");
    assert!(
        publish_consensus_attr.is_some(),
        "Consensus should be reached when all oracles submit data"
    );

    // Verify round is still the same (not increased)
    let round_attr = response.attributes.iter().find(|attr| attr.key == "round");
    assert!(round_attr.is_some());
    assert_eq!(
        round_attr.unwrap().value,
        "1",
        "Round should not increase until time passes"
    );

    // Verify next round is correctly set
    let next_round_attr = response
        .attributes
        .iter()
        .find(|attr| attr.key == "next_round");
    assert!(next_round_attr.is_some());
    assert_eq!(
        next_round_attr.unwrap().value,
        "2",
        "Next round should be set to 2 (current round + 1)"
    );
}

#[test]
fn test_partial_oracles_consensus_round_not_increased() {
    // Set up test environment
    let mut deps = mock_dependencies();
    let mut env = mock_env();
    let start_time = 1000;
    env.block.time = Timestamp::from_seconds(start_time);

    // Set up initial state with threshold = 2 (out of 3 oracles)
    let mut consensus_config = create_test_consensus_config();
    let contract_config = create_test_contract_config();
    consensus_config.threshold = 2; // Only need 2 out of 3 oracles for consensus
    let round = Round {
        round: 1,
        start: start_time,
    };
    setup_test_state(
        &mut deps.as_mut(),
        &contract_config,
        &consensus_config,
        round.round,
        round.start,
    );

    // Submit data from first oracle
    let oracle1_info = message_info("oracle1", &[]);
    let test_data1 = create_test_data(
        1,
        start_time,
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(1000, 1),
        SignedDecimal::from_ratio(2000, 1),
        SignedDecimal::from_ratio(500, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data1,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle1_info, msg);
    assert!(result.is_ok());

    // Verify no consensus yet (only one oracle)
    let response = result.unwrap();
    let publish_consensus_attr = response
        .attributes
        .iter()
        .find(|attr| attr.key == "action" && attr.value == "publish_consensus");
    assert!(
        publish_consensus_attr.is_none(),
        "No consensus should be reached with only one oracle"
    );

    // Submit data from second oracle (should reach threshold)
    let oracle2_info = message_info("oracle2", &[]);
    let test_data2 = create_test_data(
        1,
        start_time + 1,
        SignedDecimal::from_ratio(505, 1000),
        SignedDecimal::from_ratio(1005, 1),
        SignedDecimal::from_ratio(2010, 1),
        SignedDecimal::from_ratio(505, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data2,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle2_info, msg);
    assert!(result.is_ok());

    // Verify no consensus yet (need all oracles or round to pass)
    let response = result.unwrap();
    let publish_consensus_attr = response
        .attributes
        .iter()
        .find(|attr| attr.key == "action" && attr.value == "publish_consensus");
    assert!(publish_consensus_attr.is_none(), "No consensus should be reached with only threshold oracles (need all oracles or round to pass)");

    // Submit data from third oracle (all oracles now)
    let oracle3_info = message_info("oracle3", &[]);
    let test_data3 = create_test_data(
        1,
        start_time + 2,
        SignedDecimal::from_ratio(503, 1000),
        SignedDecimal::from_ratio(1003, 1),
        SignedDecimal::from_ratio(2005, 1),
        SignedDecimal::from_ratio(503, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data3,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle3_info, msg);
    assert!(result.is_ok());

    // Verify consensus was reached with all oracles
    let response = result.unwrap();
    let publish_consensus_attr = response
        .attributes
        .iter()
        .find(|attr| attr.key == "action" && attr.value == "publish_consensus");
    assert!(
        publish_consensus_attr.is_some(),
        "Consensus should be reached when all oracles submit data"
    );

    // Verify round is still the same (not increased)
    let round_attr = response.attributes.iter().find(|attr| attr.key == "round");
    assert!(round_attr.is_some());
    assert_eq!(
        round_attr.unwrap().value,
        "1",
        "Round should not increase until time passes"
    );

    // Verify next round is correctly set
    let next_round_attr = response
        .attributes
        .iter()
        .find(|attr| attr.key == "next_round");
    assert!(next_round_attr.is_some());
    assert_eq!(
        next_round_attr.unwrap().value,
        "2",
        "Next round should be set to 2 (current round + 1)"
    );
}

#[test]
fn test_no_consensus_when_threshold_not_met_and_round_passed() {
    // Set up test environment
    let mut deps = mock_dependencies();
    let mut env = mock_env();
    let start_time = 1000;
    env.block.time = Timestamp::from_seconds(start_time);

    // Set up initial state with threshold = 2 (out of 3 oracles)
    let mut consensus_config = create_test_consensus_config();
    let contract_config = create_test_contract_config();
    consensus_config.threshold = 2; // Need 2 out of 3 oracles for consensus
    let round = Round {
        round: 1,
        start: start_time,
    };
    setup_test_state(
        &mut deps.as_mut(),
        &contract_config,
        &consensus_config,
        round.round,
        round.start,
    );

    // Submit data from only one oracle (below threshold)
    let oracle1_info = message_info("oracle1", &[]);
    let test_data1 = create_test_data(
        1,
        start_time,
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(1000, 1),
        SignedDecimal::from_ratio(2000, 1),
        SignedDecimal::from_ratio(500, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data1,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle1_info, msg);
    assert!(result.is_ok());

    // Verify no consensus yet (only one oracle, below threshold)
    let response = result.unwrap();
    let publish_consensus_attr = response
        .attributes
        .iter()
        .find(|attr| attr.key == "action" && attr.value == "publish_consensus");
    assert!(
        publish_consensus_attr.is_none(),
        "No consensus should be reached with only one oracle (below threshold)"
    );

    // Advance time past round length to trigger round change
    env.block.time = Timestamp::from_seconds(start_time + consensus_config.round_length + 1);

    // Last oracle tries to submit data for the previous round (which is now passed)
    let oracle2_info = message_info("oracle2", &[]);
    let test_data2 = create_test_data(
        1, // Still trying to submit for round 1, which is now passed
        start_time + consensus_config.round_length + 1,
        SignedDecimal::from_ratio(505, 1000),
        SignedDecimal::from_ratio(1005, 1),
        SignedDecimal::from_ratio(2010, 1),
        SignedDecimal::from_ratio(505, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data2,
    };
    let result = execute(
        deps.as_mut(),
        env.clone(),
        oracle2_info.clone(),
        msg.clone(),
    );

    // Should fail because round 1 is now invalid (we're in round 2)
    assert!(result.is_err());
    match result.unwrap_err() {
        StdError::GenericErr { msg, .. } => {
            assert_eq!(msg, "Invalid round", "Should reject data for passed round")
        }
        _ => panic!("Unexpected error"),
    }

    // Submit data for the new round (round 2)
    let test_data3 = create_test_data(
        2, // New round
        start_time + consensus_config.round_length + 1,
        SignedDecimal::from_ratio(505, 1000),
        SignedDecimal::from_ratio(1005, 1),
        SignedDecimal::from_ratio(2010, 1),
        SignedDecimal::from_ratio(505, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data3,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle2_info, msg);
    assert!(result.is_ok());

    // Verify round has increased
    let response = result.unwrap();
    let round_attr = response.attributes.iter().find(|attr| attr.key == "round");
    assert!(round_attr.is_some());
    assert_eq!(
        round_attr.unwrap().value,
        "2",
        "Round should increase after time passes"
    );

    // Verify no consensus was reached for the previous round
    // This is implicit since we couldn't even submit for the previous round
    // and there was only one oracle's data (below threshold)

    // We can also check that no consensus is reached yet for the new round
    let publish_consensus_attr = response
        .attributes
        .iter()
        .find(|attr| attr.key == "action" && attr.value == "publish_consensus");
    assert!(
        publish_consensus_attr.is_none(),
        "No consensus should be reached yet for the new round"
    );
}

#[test]
fn test_multiple_rounds_passing() {
    // Set up test environment
    let mut deps = mock_dependencies();
    let mut env = mock_env();
    let start_time = 1000;
    env.block.time = Timestamp::from_seconds(start_time);

    // Set up initial state with threshold = 2 (out of 3 oracles)
    let mut consensus_config = create_test_consensus_config();
    let contract_config = create_test_contract_config();
    consensus_config.threshold = 2; // Need 2 out of 3 oracles for consensus
    let round = Round {
        round: 1,
        start: start_time,
    };
    setup_test_state(
        &mut deps.as_mut(),
        &contract_config,
        &consensus_config,
        round.round,
        round.start,
    );

    // Submit data from one oracle for round 1
    let oracle1_info = message_info("oracle1", &[]);
    let test_data1 = create_test_data(
        1,
        start_time,
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(1000, 1),
        SignedDecimal::from_ratio(2000, 1),
        SignedDecimal::from_ratio(500, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data1,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle1_info, msg);
    assert!(result.is_ok());

    // Advance time by 3 rounds (skipping rounds 2 and 3, landing in round 4)
    env.block.time = Timestamp::from_seconds(start_time + (3 * consensus_config.round_length) + 1);

    // Oracle tries to submit data for round 2 (which is now passed)
    let oracle2_info = message_info("oracle2", &[]);
    let test_data2 = create_test_data(
        2, // Trying to submit for round 2, which is now passed
        start_time + (3 * consensus_config.round_length) + 1,
        SignedDecimal::from_ratio(505, 1000),
        SignedDecimal::from_ratio(1005, 1),
        SignedDecimal::from_ratio(2010, 1),
        SignedDecimal::from_ratio(505, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data2,
    };
    let result = execute(
        deps.as_mut(),
        env.clone(),
        oracle2_info.clone(),
        msg.clone(),
    );

    // Should fail because round 2 is now invalid (we're in round 4)
    assert!(result.is_err());
    match result.unwrap_err() {
        StdError::GenericErr { msg, .. } => {
            assert_eq!(msg, "Invalid round", "Should reject data for passed round")
        }
        _ => panic!("Unexpected error"),
    }

    // Submit data for the current round (round 4)
    let test_data4 = create_test_data(
        4, // Current round after 3 rounds passed
        start_time + (3 * consensus_config.round_length) + 1,
        SignedDecimal::from_ratio(505, 1000),
        SignedDecimal::from_ratio(1005, 1),
        SignedDecimal::from_ratio(2010, 1),
        SignedDecimal::from_ratio(505, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data4,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle2_info, msg);
    assert!(result.is_ok());

    // Verify round has increased to 4 (skipping 2 and 3)
    let response = result.unwrap();
    let round_attr = response.attributes.iter().find(|attr| attr.key == "round");
    assert!(round_attr.is_some());
    assert_eq!(
        round_attr.unwrap().value,
        "4",
        "Round should be 4 after 3 rounds have passed"
    );

    // Verify next round is correctly set to 5
    let next_round_attr = response
        .attributes
        .iter()
        .find(|attr| attr.key == "next_round");
    assert!(next_round_attr.is_some());
    assert_eq!(
        next_round_attr.unwrap().value,
        "5",
        "Next round should be set to 5"
    );
}

#[test]
fn test_oracles_submitting_across_multiple_rounds() {
    // Set up test environment
    let mut deps = mock_dependencies();
    let mut env = mock_env();
    let start_time = 1000;
    env.block.time = Timestamp::from_seconds(start_time);

    // Set up initial state with threshold = 2 (out of 3 oracles)
    let mut consensus_config = create_test_consensus_config();
    let contract_config = create_test_contract_config();
    consensus_config.threshold = 2;
    let round = Round {
        round: 1,
        start: start_time,
    };
    setup_test_state(
        &mut deps.as_mut(),
        &contract_config,
        &consensus_config,
        round.round,
        round.start,
    );

    // Get initial data - should be None since nothing is published yet
    let initial_data = query_last_published_data(deps.as_ref(), env.clone());
    assert!(
        initial_data.last_published_data.is_none(),
        "No data should be published initially"
    );

    // Oracle 1 submits data for round 1
    let oracle1_info = message_info("oracle1", &[]);
    let test_data1 = create_test_data(
        1,
        start_time,
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(1000, 1),
        SignedDecimal::from_ratio(2000, 1),
        SignedDecimal::from_ratio(500, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data1.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), oracle1_info.clone(), msg);
    assert!(result.is_ok());

    // Check data after one oracle submission - should still be None (below threshold)
    let data_after_one_oracle = query_last_published_data(deps.as_ref(), env.clone());
    assert!(
        data_after_one_oracle.last_published_data.is_none(),
        "No data should be published with only one oracle (below threshold)"
    );

    // Oracle 2 submits data for round 1 (reaching threshold)
    let oracle2_info = message_info("oracle2", &[]);
    let test_data1_2 = create_test_data(
        1,
        start_time + 1,
        SignedDecimal::from_ratio(505, 1000),
        SignedDecimal::from_ratio(1005, 1),
        SignedDecimal::from_ratio(2010, 1),
        SignedDecimal::from_ratio(505, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data1_2.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), oracle2_info.clone(), msg);
    assert!(result.is_ok());

    // Check data after threshold reached but round not passed - should still be None
    let data_after_threshold = query_last_published_data(deps.as_ref(), env.clone());
    assert!(
        data_after_threshold.last_published_data.is_none(),
        "No data should be published with only threshold oracles (round not passed)"
    );

    // Oracle 3 submits data for round 1 (all oracles)
    let oracle3_info = message_info("oracle3", &[]);
    let test_data1_3 = create_test_data(
        1,
        start_time + 2,
        SignedDecimal::from_ratio(503, 1000),
        SignedDecimal::from_ratio(1003, 1),
        SignedDecimal::from_ratio(2005, 1),
        SignedDecimal::from_ratio(503, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data1_3.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), oracle3_info.clone(), msg);
    assert!(result.is_ok());

    // Scenario 1: All oracles submitted data but round is not passed yet
    // Check data after all oracles submitted - should be updated
    let data_after_all_oracles = query_last_published_data(deps.as_ref(), env.clone());
    assert!(
        data_after_all_oracles.last_published_data.is_some(),
        "Data should be published when all oracles submit"
    );
    assert_eq!(
        data_after_all_oracles.last_published_data.unwrap().round,
        1,
        "Published data should be for round 1"
    );

    // Advance time to round 2
    env.block.time = Timestamp::from_seconds(start_time + consensus_config.round_length + 1);

    // Oracle 2 submits data for round 2
    let test_data2 = create_test_data(
        2, // Round 2
        start_time + consensus_config.round_length + 1,
        SignedDecimal::from_ratio(505, 1000),
        SignedDecimal::from_ratio(1005, 1),
        SignedDecimal::from_ratio(2010, 1),
        SignedDecimal::from_ratio(505, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data2.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), oracle2_info.clone(), msg);
    assert!(result.is_ok());

    // Verify we're in round 2
    let response = result.unwrap();
    let round_attr = response.attributes.iter().find(|attr| attr.key == "round");
    assert!(round_attr.is_some());
    assert_eq!(round_attr.unwrap().value, "2", "Round should be 2");

    // Advance time to round 3
    env.block.time = Timestamp::from_seconds(start_time + (2 * consensus_config.round_length) + 1);

    // Oracle 1 submits data for round 3
    let test_data3 = create_test_data(
        3, // Round 3
        start_time + (2 * consensus_config.round_length) + 1,
        SignedDecimal::from_ratio(504, 1000),
        SignedDecimal::from_ratio(1004, 1),
        SignedDecimal::from_ratio(2008, 1),
        SignedDecimal::from_ratio(504, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data3.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), oracle1_info, msg);
    assert!(result.is_ok());

    // Verify we're in round 3
    let response = result.unwrap();
    let round_attr = response.attributes.iter().find(|attr| attr.key == "round");
    assert!(round_attr.is_some());
    assert_eq!(round_attr.unwrap().value, "3", "Round should be 3");

    // Oracle 2 also submits data for round 3 (reaching threshold)
    let test_data3_2 = create_test_data(
        3, // Round 3
        start_time + (2 * consensus_config.round_length) + 2,
        SignedDecimal::from_ratio(504, 1000),
        SignedDecimal::from_ratio(1004, 1),
        SignedDecimal::from_ratio(2008, 1),
        SignedDecimal::from_ratio(504, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data3_2.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), oracle2_info, msg);
    assert!(result.is_ok());

    // Scenario 2: Part of oracles (more than threshold) submitted data but round not passed yet
    // Check data after threshold reached but round not passed - should still show round 1 data
    let data_after_threshold_round3 = query_last_published_data(deps.as_ref(), env.clone());
    assert!(
        data_after_threshold_round3.last_published_data.is_some(),
        "Data should still be available"
    );
    assert_eq!(
        data_after_threshold_round3
            .last_published_data
            .unwrap()
            .round,
        1,
        "Published data should still be from round 1"
    );

    // Advance time to round 4 (passing round 3)
    env.block.time = Timestamp::from_seconds(start_time + (3 * consensus_config.round_length) + 1);

    // Scenario 2 (continued): Round is passed with threshold oracles having submitted
    // Check data after round passed with threshold met - should be updated to round 3 data
    let data_after_round_passed = query_last_published_data(deps.as_ref(), env.clone());
    assert!(
        data_after_round_passed.last_published_data.is_some(),
        "Data should be published when round passes with threshold met"
    );
    assert_eq!(
        data_after_round_passed.last_published_data.unwrap().round,
        3,
        "Published data should be updated to round 3"
    );

    // Oracle 3 submits data for round 4
    let test_data4 = create_test_data(
        4, // Round 4
        start_time + (3 * consensus_config.round_length) + 1,
        SignedDecimal::from_ratio(51, 100),
        SignedDecimal::from_ratio(1010, 1),
        SignedDecimal::from_ratio(2020, 1),
        SignedDecimal::from_ratio(510, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data4.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), oracle3_info.clone(), msg);
    assert!(result.is_ok());

    // Verify we're in round 4
    let response = result.unwrap();
    let round_attr = response.attributes.iter().find(|attr| attr.key == "round");
    assert!(round_attr.is_some());
    assert_eq!(
        round_attr.unwrap().value,
        "4",
        "Round should be 4 after round 3 has passed"
    );

    // Advance time to round 5 (skipping round 4)
    env.block.time = Timestamp::from_seconds(start_time + (4 * consensus_config.round_length) + 1);

    // Scenario 3: Less than threshold oracles submitted data and round is passed
    // Check data after round passed with below threshold - should still show round 3 data
    let data_after_round_passed_below_threshold =
        query_last_published_data(deps.as_ref(), env.clone());
    assert!(
        data_after_round_passed_below_threshold
            .last_published_data
            .is_some(),
        "Data should still be available"
    );
    assert_eq!(
        data_after_round_passed_below_threshold
            .last_published_data
            .unwrap()
            .round,
        3,
        "Published data should still be from round 3 (round 4 had below threshold submissions)"
    );

    // Oracle 3 submits data for round 5
    let test_data5 = create_test_data(
        5, // Round 5
        start_time + (4 * consensus_config.round_length) + 1,
        SignedDecimal::from_ratio(50, 100),
        SignedDecimal::from_ratio(1000, 1),
        SignedDecimal::from_ratio(2000, 1),
        SignedDecimal::from_ratio(500, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data5.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), oracle3_info, msg);
    assert!(result.is_ok());

    // Verify we're in round 5
    let response = result.unwrap();
    let round_attr = response.attributes.iter().find(|attr| attr.key == "round");
    assert!(round_attr.is_some());
    assert_eq!(
        round_attr.unwrap().value,
        "5",
        "Round should be 5 after skipping round 4"
    );

    // This test demonstrates:
    // 1. When all oracles submit data but round is not passed yet, query_get_data shows the latest published data
    // 2. When only part of oracles (but more than threshold) submit data and round is not passed,
    //    the last published data is not updated. But when the round is passed, it gets updated properly.
    // 3. When less than threshold oracles submit data and round is passed, last published data is not changed
}

#[test]
fn test_oracle_cannot_publish_twice_for_same_round() {
    // Set up test environment
    let mut deps = mock_dependencies();
    let mut env = mock_env();
    let start_time = 1000;
    env.block.time = Timestamp::from_seconds(start_time);

    // Set up initial state
    let consensus_config = create_test_consensus_config();
    let contract_config = create_test_contract_config();
    let round = Round {
        round: 1,
        start: start_time,
    };
    setup_test_state(
        &mut deps.as_mut(),
        &contract_config,
        &consensus_config,
        round.round,
        round.start,
    );

    // Oracle 1 submits data for round 1
    let oracle1_info = message_info("oracle1", &[]);
    let test_data1 = create_test_data(
        1,
        start_time,
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(1000, 1),
        SignedDecimal::from_ratio(2000, 1),
        SignedDecimal::from_ratio(500, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data1.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), oracle1_info.clone(), msg);
    assert!(result.is_ok(), "First submission should succeed");

    // Oracle 1 tries to submit data for round 1 again (should fail)
    let test_data1_again = create_test_data(
        1,
        start_time + 1,
        SignedDecimal::from_ratio(51, 100),
        SignedDecimal::from_ratio(1010, 1),
        SignedDecimal::from_ratio(2010, 1),
        SignedDecimal::from_ratio(510, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data1_again,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle1_info.clone(), msg);
    assert!(
        result.is_err(),
        "Second submission for the same round should fail"
    );
    match result.unwrap_err() {
        StdError::GenericErr { msg, .. } => assert_eq!(
            msg, "Oracle has already submitted data for this round",
            "Error message should indicate duplicate submission"
        ),
        _ => panic!("Unexpected error"),
    }

    // Advance time to round 2
    env.block.time = Timestamp::from_seconds(start_time + consensus_config.round_length + 1);

    // Oracle 1 submits data for round 2 (should succeed)
    let test_data2 = create_test_data(
        2,
        start_time + consensus_config.round_length + 1,
        SignedDecimal::from_ratio(52, 100),
        SignedDecimal::from_ratio(1020, 1),
        SignedDecimal::from_ratio(2020, 1),
        SignedDecimal::from_ratio(520, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data2,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle1_info, msg);
    assert!(result.is_ok(), "Submission for a new round should succeed");

    // Verify we're in round 2
    let response = result.unwrap();
    let round_attr = response.attributes.iter().find(|attr| attr.key == "round");
    assert!(round_attr.is_some());
    assert_eq!(round_attr.unwrap().value, "2", "Round should be 2");
}

#[test]
fn test_delayed_oracle_submissions_within_round() {
    // Set up test environment
    let mut deps = mock_dependencies();
    let mut env = mock_env();
    let start_time = 1000;
    env.block.time = Timestamp::from_seconds(start_time);

    // Set up initial state with threshold = 2 (out of 3 oracles)
    let mut consensus_config = create_test_consensus_config();
    let contract_config = create_test_contract_config();
    consensus_config.threshold = 2; // Need 2 out of 3 oracles for consensus
    let round = Round {
        round: 1,
        start: start_time,
    };
    setup_test_state(
        &mut deps.as_mut(),
        &contract_config,
        &consensus_config,
        round.round,
        round.start,
    );

    // Oracle 1 submits data at the beginning of round 1
    let oracle1_info = message_info("oracle1", &[]);
    let test_data1 = create_test_data(
        1,
        start_time,
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(1000, 1),
        SignedDecimal::from_ratio(2000, 1),
        SignedDecimal::from_ratio(500, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data1,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle1_info, msg);
    assert!(result.is_ok());

    // Verify no consensus yet (only one oracle)
    let response = result.unwrap();
    let publish_consensus_attr = response
        .attributes
        .iter()
        .find(|attr| attr.key == "action" && attr.value == "publish_consensus");
    assert!(
        publish_consensus_attr.is_none(),
        "No consensus should be reached with only one oracle"
    );

    // Advance time within the same round (but not past round length)
    env.block.time = Timestamp::from_seconds(start_time + consensus_config.round_length / 2);

    // Oracle 2 submits data in the middle of round 1
    let oracle2_info = message_info("oracle2", &[]);
    let test_data2 = create_test_data(
        1, // Still round 1
        start_time + consensus_config.round_length / 2,
        SignedDecimal::from_ratio(505, 1000),
        SignedDecimal::from_ratio(1005, 1),
        SignedDecimal::from_ratio(2010, 1),
        SignedDecimal::from_ratio(505, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data2,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle2_info, msg);
    assert!(result.is_ok());

    // Verify we're still in round 1
    let response = result.unwrap();
    let round_attr = response.attributes.iter().find(|attr| attr.key == "round");
    assert!(round_attr.is_some());
    assert_eq!(round_attr.unwrap().value, "1", "Round should still be 1");

    // Advance time to just before the end of round 1
    env.block.time = Timestamp::from_seconds(start_time + consensus_config.round_length - 10);

    // Oracle 3 submits data near the end of round 1
    let oracle3_info = message_info("oracle3", &[]);
    let test_data3 = create_test_data(
        1, // Still round 1
        start_time + consensus_config.round_length - 10,
        SignedDecimal::from_ratio(503, 1000),
        SignedDecimal::from_ratio(1003, 1),
        SignedDecimal::from_ratio(2005, 1),
        SignedDecimal::from_ratio(503, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data3,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle3_info.clone(), msg);
    assert!(result.is_ok());

    // Verify consensus was reached for round 1 (all oracles submitted)
    let response = result.unwrap();
    let publish_consensus_attr = response
        .attributes
        .iter()
        .find(|attr| attr.key == "action" && attr.value == "publish_consensus");
    assert!(
        publish_consensus_attr.is_some(),
        "Consensus should be reached when all oracles submit data"
    );

    // Verify we're still in round 1
    let round_attr = response.attributes.iter().find(|attr| attr.key == "round");
    assert!(round_attr.is_some());
    assert_eq!(round_attr.unwrap().value, "1", "Round should still be 1");

    // Advance time to round 2
    env.block.time = Timestamp::from_seconds(start_time + consensus_config.round_length + 1);

    // Oracle 3 submits data for round 2
    let test_data4 = create_test_data(
        2, // Round 2
        start_time + consensus_config.round_length + 1,
        SignedDecimal::from_ratio(51, 100),
        SignedDecimal::from_ratio(1010, 1),
        SignedDecimal::from_ratio(2020, 1),
        SignedDecimal::from_ratio(510, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data4,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle3_info, msg);
    assert!(result.is_ok());

    // Verify we're now in round 2
    let response = result.unwrap();
    let round_attr = response.attributes.iter().find(|attr| attr.key == "round");
    assert!(round_attr.is_some());
    assert_eq!(round_attr.unwrap().value, "2", "Round should now be 2");
}
