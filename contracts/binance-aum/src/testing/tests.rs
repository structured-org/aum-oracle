use crate::contract::*;
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetAumResponse, GetDataResponse, QueryMsg};
use crate::state::{BinanceData, Config, Position, SpotBalance, CONFIG, CONSENSUS_STATE};
use crate::testing::mock::custom_mock_dependencies;
use crate::utils::CombinedPriceResponse;
use consensus::consensus::{Config as ConsensusConfig, ConsensusData, OracleData, Round, State};
use consensus::error::ConsensusError;
use cosmwasm_schema::schemars;
use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_std::{
    from_json,
    testing::{mock_dependencies, mock_env},
    to_json_binary, Addr, Coin, Deps, DepsMut, Env, Int256, MessageInfo, SignedDecimal256,
    Timestamp,
};
use neutron_std::types::neutron::util::precdec::PrecDec;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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
        owner: Addr::unchecked("admin"),
        price_max_blocks_old: 100,
        required_binance_positions: vec!["BTCUSDT".to_string()],
        required_binance_spot_assets: vec!["BTC".to_string(), "USDT".to_string()],
        price_oracle_contract: Addr::unchecked("price_oracle_contract"),
        consensus_data_valid_period: 100,
    }
}

// Helper function to create a test BinanceData object
fn create_test_data(
    unimmr: SignedDecimal256,
    um_balance: SignedDecimal256,
    pm_equity: SignedDecimal256,
    withdrawable: SignedDecimal256,
) -> BinanceData {
    BinanceData {
        unimmr,
        positions: vec![Position {
            symbol: "BTCUSDT".to_string(),
            amount: SignedDecimal256::from_ratio(1, 1),
            pnl: SignedDecimal256::from_ratio(100, 1),
        }],
        um_balance_usdt: um_balance,
        spot_balances: vec![
            SpotBalance {
                asset: "BTC".to_string(),
                amount: SignedDecimal256::from_ratio(1, 1),
            },
            SpotBalance {
                asset: "USDT".to_string(),
                amount: SignedDecimal256::from_ratio(10000, 1),
            },
        ],
        pm_account_actual_equity: pm_equity,
        withdrawable_usdt: withdrawable,
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
        SignedDecimal256::from_ratio(5, 10),
        SignedDecimal256::from_ratio(1000, 1),
        SignedDecimal256::from_ratio(2000, 1),
        SignedDecimal256::from_ratio(500, 1),
    );

    // Test 1: Unauthorized oracle rejection
    let unauthorized_info = message_info("unauthorized", &[]);
    let msg = ExecuteMsg::PublishData {
        new_data: test_data.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), unauthorized_info, msg);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), ContractError::Unauthorized {});

    // Test 2: Invalid positions
    let oracle_info = message_info("oracle1", &[]);
    let mut invalid_positions_data = create_test_data(
        SignedDecimal256::from_ratio(5, 10),
        SignedDecimal256::from_ratio(1000, 1),
        SignedDecimal256::from_ratio(2000, 1),
        SignedDecimal256::from_ratio(500, 1),
    );
    invalid_positions_data.positions = vec![
        Position {
            symbol: "WRONG_SYMBOL".to_string(),
            amount: SignedDecimal256::from_ratio(1, 1),
            pnl: SignedDecimal256::from_ratio(100, 1),
        },
        Position {
            symbol: "ANOTHER_WRONG_SYMBOL".to_string(),
            amount: SignedDecimal256::from_ratio(1, 1),
            pnl: SignedDecimal256::from_ratio(100, 1),
        },
    ];

    let msg = ExecuteMsg::PublishData {
        new_data: invalid_positions_data,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle_info.clone(), msg);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContractError::InvalidBinanceData { msg } => {
            assert_eq!(msg, "Binance positions do not match required positions")
        }
        _ => panic!("Unexpected error"),
    }

    // Test 2: Invalid spot balances
    let oracle_info = message_info("oracle1", &[]);
    let mut invalid_spot_balances_data = create_test_data(
        SignedDecimal256::from_ratio(5, 10),
        SignedDecimal256::from_ratio(1000, 1),
        SignedDecimal256::from_ratio(2000, 1),
        SignedDecimal256::from_ratio(500, 1),
    );
    invalid_spot_balances_data.spot_balances = vec![
        SpotBalance {
            asset: "WRONG_ASSET".to_string(),
            amount: SignedDecimal256::from_ratio(1, 1),
        },
        SpotBalance {
            asset: "ANOTHER_WRONG_ASSET".to_string(),
            amount: SignedDecimal256::from_ratio(10000, 1),
        },
    ];

    let msg = ExecuteMsg::PublishData {
        new_data: invalid_spot_balances_data,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle_info.clone(), msg);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContractError::InvalidBinanceData { msg } => {
            assert_eq!(msg, "Binance spot assets do not match required spot assets")
        }
        _ => panic!("Unexpected error"),
    }

    // Test 2: Valid submission acceptance
    let oracle_info = message_info("oracle1", &[]);
    let msg = ExecuteMsg::PublishData {
        new_data: test_data.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), oracle_info.clone(), msg);
    assert!(result.is_ok());

    // Test 3: Submit data from all oracles to reach consensus
    let oracle2_info = message_info("oracle2", &[]);
    let mut test_data2 = create_test_data(
        SignedDecimal256::from_ratio(505, 1000),
        SignedDecimal256::from_ratio(1005, 1),
        SignedDecimal256::from_ratio(2010, 1),
        SignedDecimal256::from_ratio(505, 1),
    );
    // BinanceData with additional wrong positions and spot balances must be accepted anyway, since
    // the data is being cleaned
    test_data2.positions.append(&mut vec![
        Position {
            symbol: "WRONG_SYMBOL".to_string(),
            amount: SignedDecimal256::from_ratio(1, 1),
            pnl: SignedDecimal256::from_ratio(100, 1),
        },
        Position {
            symbol: "ANOTHER_WRONG_SYMBOL".to_string(),
            amount: SignedDecimal256::from_ratio(1, 1),
            pnl: SignedDecimal256::from_ratio(100, 1),
        },
    ]);
    test_data2.spot_balances.append(&mut vec![
        SpotBalance {
            asset: "WRONG_ASSET".to_string(),
            amount: SignedDecimal256::from_ratio(1, 1),
        },
        SpotBalance {
            asset: "ANOTHER_WRONG_ASSET".to_string(),
            amount: SignedDecimal256::from_ratio(10000, 1),
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
        SignedDecimal256::from_ratio(503, 1000),
        SignedDecimal256::from_ratio(1003, 1),
        SignedDecimal256::from_ratio(2005, 1),
        SignedDecimal256::from_ratio(503, 1),
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

    // Test 4: Advance the round by time expiration
    // Update the block time to after the round expiration
    env.block.time = Timestamp::from_seconds(1000 + consensus_config.round_length + 1);

    // Submit data for the new round (which should be 2 now)
    let oracle2_info = message_info("oracle2", &[]);
    let new_round_data = create_test_data(
        SignedDecimal256::from_ratio(503, 1000),
        SignedDecimal256::from_ratio(1003, 1),
        SignedDecimal256::from_ratio(2005, 1),
        SignedDecimal256::from_ratio(503, 1),
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
        SignedDecimal256::from_ratio(5, 10),
        SignedDecimal256::from_ratio(1000, 1),
        SignedDecimal256::from_ratio(2000, 1),
        SignedDecimal256::from_ratio(500, 1),
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
        SignedDecimal256::from_ratio(5, 10),   // Identical to test_data1
        SignedDecimal256::from_ratio(1000, 1), // Identical to test_data1
        SignedDecimal256::from_ratio(2000, 1), // Identical to test_data1
        SignedDecimal256::from_ratio(500, 1),  // Identical to test_data1
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
        SignedDecimal256::from_ratio(5, 10), // Identical to test_data1 and test_data2
        SignedDecimal256::from_ratio(1000, 1), // Identical to test_data1 and test_data2
        SignedDecimal256::from_ratio(2000, 1), // Identical to test_data1 and test_data2
        SignedDecimal256::from_ratio(500, 1), // Identical to test_data1 and test_data2
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
        let single_data = vec![create_test_data(
            SignedDecimal256::from_ratio(5, 10),
            SignedDecimal256::from_ratio(1000, 1),
            SignedDecimal256::from_ratio(2000, 1),
            SignedDecimal256::from_ratio(500, 1),
        )];
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
                SignedDecimal256::from_ratio(5, 10),
                SignedDecimal256::from_ratio(1000, 1),
                SignedDecimal256::from_ratio(2000, 1),
                SignedDecimal256::from_ratio(500, 1),
            ),
            create_test_data(
                SignedDecimal256::from_ratio(505, 1000),
                SignedDecimal256::from_ratio(1005, 1),
                SignedDecimal256::from_ratio(2010, 1),
                SignedDecimal256::from_ratio(505, 1),
            ), // within 1% delta
        ];
        let consensus =
            BinanceData::try_consensus(&data, config.threshold as usize, config.data_delta_ppm);
        assert!(consensus.is_some());

        let result = consensus.unwrap();
        assert_eq!(result.unimmr, SignedDecimal256::from_ratio(5025, 10000)); // median of 0.5 and 0.505
        assert_eq!(
            result.um_balance_usdt,
            SignedDecimal256::from_ratio(10025, 10)
        ); // median of 1000 and 1005
        assert_eq!(
            result.pm_account_actual_equity,
            SignedDecimal256::from_ratio(2005, 1)
        ); // median of 2000 and 2010
        assert_eq!(
            result.withdrawable_usdt,
            SignedDecimal256::from_ratio(5025, 10)
        ); // median of 500 and 505
    }

    // Test 4: Data outside acceptable delta should not reach consensus
    {
        let mut config = create_test_consensus_config();
        config.data_delta_ppm = 1000; // 0.1% delta

        let data_divergent = vec![
            create_test_data(
                SignedDecimal256::from_ratio(5, 10),
                SignedDecimal256::from_ratio(1000, 1),
                SignedDecimal256::from_ratio(2000, 1),
                SignedDecimal256::from_ratio(500, 1),
            ),
            create_test_data(
                SignedDecimal256::from_ratio(51, 100),
                SignedDecimal256::from_ratio(1020, 1),
                SignedDecimal256::from_ratio(2050, 1),
                SignedDecimal256::from_ratio(510, 1),
            ), // outside 0.1% delta
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
                SignedDecimal256::from_ratio(5, 10),
                SignedDecimal256::from_ratio(1000, 1),
                SignedDecimal256::from_ratio(2000, 1),
                SignedDecimal256::from_ratio(500, 1),
            ),
            create_test_data(
                SignedDecimal256::from_ratio(505, 1000),
                SignedDecimal256::from_ratio(1005, 1),
                SignedDecimal256::from_ratio(2010, 1),
                SignedDecimal256::from_ratio(505, 1),
            ),
            create_test_data(
                SignedDecimal256::from_ratio(503, 1000),
                SignedDecimal256::from_ratio(1003, 1),
                SignedDecimal256::from_ratio(2005, 1),
                SignedDecimal256::from_ratio(503, 1),
            ),
        ];

        let consensus = BinanceData::try_consensus(
            &data_multiple,
            config.threshold as usize,
            config.data_delta_ppm,
        );
        assert!(consensus.is_some());

        let result = consensus.unwrap();
        assert_eq!(result.unimmr, SignedDecimal256::from_ratio(503, 1000));
        assert_eq!(
            result.um_balance_usdt,
            SignedDecimal256::from_ratio(1003, 1)
        );
        assert_eq!(
            result.pm_account_actual_equity,
            SignedDecimal256::from_ratio(2005, 1)
        );
        assert_eq!(
            result.withdrawable_usdt,
            SignedDecimal256::from_ratio(503, 1)
        );
    }

    // Test 6: Consensus with some outliers
    {
        let config = create_test_consensus_config();
        let data_with_outliers = vec![
            create_test_data(
                SignedDecimal256::from_ratio(5, 10),
                SignedDecimal256::from_ratio(1000, 1),
                SignedDecimal256::from_ratio(2000, 1),
                SignedDecimal256::from_ratio(500, 1),
            ),
            create_test_data(
                SignedDecimal256::from_ratio(505, 1000),
                SignedDecimal256::from_ratio(1005, 1),
                SignedDecimal256::from_ratio(2010, 1),
                SignedDecimal256::from_ratio(505, 1),
            ),
            create_test_data(
                SignedDecimal256::from_ratio(6, 10),
                SignedDecimal256::from_ratio(1200, 1),
                SignedDecimal256::from_ratio(2500, 1),
                SignedDecimal256::from_ratio(600, 1),
            ), // outlier
        ];

        let consensus = BinanceData::try_consensus(
            &data_with_outliers,
            config.threshold as usize,
            config.data_delta_ppm,
        );
        assert!(consensus.is_some());

        // The outlier should be excluded from the consensus
        let result = consensus.unwrap();
        assert_eq!(result.unimmr, SignedDecimal256::from_ratio(5025, 10000)); // median of 0.5 and 0.505 (outlier excluded)
    }

    // Test 7: Consensus with multiple positions
    {
        let config = create_test_consensus_config();
        let data1 = create_test_data(
            SignedDecimal256::from_ratio(5, 10),
            SignedDecimal256::from_ratio(1000, 1),
            SignedDecimal256::from_ratio(2000, 1),
            SignedDecimal256::from_ratio(500, 1),
        );
        let data2 = create_test_data(
            SignedDecimal256::from_ratio(505, 1000),
            SignedDecimal256::from_ratio(1005, 1),
            SignedDecimal256::from_ratio(2010, 1),
            SignedDecimal256::from_ratio(505, 1),
        );
        //outlier
        let data3 = create_test_data(
            SignedDecimal256::from_ratio(6, 10),
            SignedDecimal256::from_ratio(1200, 1),
            SignedDecimal256::from_ratio(2500, 1),
            SignedDecimal256::from_ratio(600, 1),
        );
        let data_with_outliers = vec![data1, data2, data3];

        let consensus = BinanceData::try_consensus(
            &data_with_outliers,
            config.threshold as usize,
            config.data_delta_ppm,
        );
        assert!(consensus.is_some());

        // The outlier should be excluded from the consensus
        let result = consensus.unwrap();
        assert_eq!(result.unimmr, SignedDecimal256::from_ratio(5025, 10000)); // median of 0.5 and 0.505 (outlier excluded)
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
        SignedDecimal256::from_ratio(5, 10),
        SignedDecimal256::from_ratio(1000, 1),
        SignedDecimal256::from_ratio(2000, 1),
        SignedDecimal256::from_ratio(500, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data1,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle1_info, msg);
    assert!(result.is_ok());

    let oracle2_info = message_info("oracle2", &[]);
    let test_data2 = create_test_data(
        SignedDecimal256::from_ratio(505, 1000),
        SignedDecimal256::from_ratio(1005, 1),
        SignedDecimal256::from_ratio(2010, 1),
        SignedDecimal256::from_ratio(505, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data2,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle2_info, msg);
    assert!(result.is_ok());

    let oracle3_info = message_info("oracle3", &[]);
    let test_data3 = create_test_data(
        SignedDecimal256::from_ratio(503, 1000),
        SignedDecimal256::from_ratio(1003, 1),
        SignedDecimal256::from_ratio(2005, 1),
        SignedDecimal256::from_ratio(503, 1),
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
        SignedDecimal256::from_ratio(5, 10),
        SignedDecimal256::from_ratio(1000, 1),
        SignedDecimal256::from_ratio(2000, 1),
        SignedDecimal256::from_ratio(500, 1),
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
        SignedDecimal256::from_ratio(505, 1000),
        SignedDecimal256::from_ratio(1005, 1),
        SignedDecimal256::from_ratio(2010, 1),
        SignedDecimal256::from_ratio(505, 1),
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
        SignedDecimal256::from_ratio(503, 1000),
        SignedDecimal256::from_ratio(1003, 1),
        SignedDecimal256::from_ratio(2005, 1),
        SignedDecimal256::from_ratio(503, 1),
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
        SignedDecimal256::from_ratio(5, 10),
        SignedDecimal256::from_ratio(1000, 1),
        SignedDecimal256::from_ratio(2000, 1),
        SignedDecimal256::from_ratio(500, 1),
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

    // Submit data for the new round (round 2)
    let oracle2_info = message_info("oracle2", &[]);
    let test_data3 = create_test_data(
        SignedDecimal256::from_ratio(505, 1000),
        SignedDecimal256::from_ratio(1005, 1),
        SignedDecimal256::from_ratio(2010, 1),
        SignedDecimal256::from_ratio(505, 1),
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
        SignedDecimal256::from_ratio(5, 10),
        SignedDecimal256::from_ratio(1000, 1),
        SignedDecimal256::from_ratio(2000, 1),
        SignedDecimal256::from_ratio(500, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data1,
    };
    let result = execute(deps.as_mut(), env.clone(), oracle1_info, msg);
    assert!(result.is_ok());

    // Advance time by 3 rounds (skipping rounds 2 and 3, landing in round 4)
    env.block.time = Timestamp::from_seconds(start_time + (3 * consensus_config.round_length) + 1);

    // Submit data for the current round (round 4)
    let oracle2_info = message_info("oracle2", &[]);
    let test_data4 = create_test_data(
        SignedDecimal256::from_ratio(505, 1000),
        SignedDecimal256::from_ratio(1005, 1),
        SignedDecimal256::from_ratio(2010, 1),
        SignedDecimal256::from_ratio(505, 1),
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
        SignedDecimal256::from_ratio(5, 10),
        SignedDecimal256::from_ratio(1000, 1),
        SignedDecimal256::from_ratio(2000, 1),
        SignedDecimal256::from_ratio(500, 1),
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
        SignedDecimal256::from_ratio(505, 1000),
        SignedDecimal256::from_ratio(1005, 1),
        SignedDecimal256::from_ratio(2010, 1),
        SignedDecimal256::from_ratio(505, 1),
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
        SignedDecimal256::from_ratio(503, 1000),
        SignedDecimal256::from_ratio(1003, 1),
        SignedDecimal256::from_ratio(2005, 1),
        SignedDecimal256::from_ratio(503, 1),
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
        SignedDecimal256::from_ratio(505, 1000),
        SignedDecimal256::from_ratio(1005, 1),
        SignedDecimal256::from_ratio(2010, 1),
        SignedDecimal256::from_ratio(505, 1),
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
        SignedDecimal256::from_ratio(504, 1000),
        SignedDecimal256::from_ratio(1004, 1),
        SignedDecimal256::from_ratio(2008, 1),
        SignedDecimal256::from_ratio(504, 1),
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
        SignedDecimal256::from_ratio(504, 1000),
        SignedDecimal256::from_ratio(1004, 1),
        SignedDecimal256::from_ratio(2008, 1),
        SignedDecimal256::from_ratio(504, 1),
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
        SignedDecimal256::from_ratio(51, 100),
        SignedDecimal256::from_ratio(1010, 1),
        SignedDecimal256::from_ratio(2020, 1),
        SignedDecimal256::from_ratio(510, 1),
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
        SignedDecimal256::from_ratio(50, 100),
        SignedDecimal256::from_ratio(1000, 1),
        SignedDecimal256::from_ratio(2000, 1),
        SignedDecimal256::from_ratio(500, 1),
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
        SignedDecimal256::from_ratio(5, 10),
        SignedDecimal256::from_ratio(1000, 1),
        SignedDecimal256::from_ratio(2000, 1),
        SignedDecimal256::from_ratio(500, 1),
    );
    let msg = ExecuteMsg::PublishData {
        new_data: test_data1.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), oracle1_info.clone(), msg);
    assert!(result.is_ok(), "First submission should succeed");

    // Oracle 1 tries to submit data for round 1 again (should fail)
    let test_data1_again = create_test_data(
        SignedDecimal256::from_ratio(51, 100),
        SignedDecimal256::from_ratio(1010, 1),
        SignedDecimal256::from_ratio(2010, 1),
        SignedDecimal256::from_ratio(510, 1),
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
        ContractError::ConsensusError(ConsensusError::DoubleSubmission {}) => {}
        _ => panic!("Unexpected error"),
    }

    // Advance time to round 2
    env.block.time = Timestamp::from_seconds(start_time + consensus_config.round_length + 1);

    // Oracle 1 submits data for round 2 (should succeed)
    let test_data2 = create_test_data(
        SignedDecimal256::from_ratio(52, 100),
        SignedDecimal256::from_ratio(1020, 1),
        SignedDecimal256::from_ratio(2020, 1),
        SignedDecimal256::from_ratio(520, 1),
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
        SignedDecimal256::from_ratio(5, 10),
        SignedDecimal256::from_ratio(1000, 1),
        SignedDecimal256::from_ratio(2000, 1),
        SignedDecimal256::from_ratio(500, 1),
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
        SignedDecimal256::from_ratio(505, 1000),
        SignedDecimal256::from_ratio(1005, 1),
        SignedDecimal256::from_ratio(2010, 1),
        SignedDecimal256::from_ratio(505, 1),
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
        SignedDecimal256::from_ratio(503, 1000),
        SignedDecimal256::from_ratio(1003, 1),
        SignedDecimal256::from_ratio(2005, 1),
        SignedDecimal256::from_ratio(503, 1),
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
        SignedDecimal256::from_ratio(51, 100),
        SignedDecimal256::from_ratio(1010, 1),
        SignedDecimal256::from_ratio(2020, 1),
        SignedDecimal256::from_ratio(510, 1),
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

#[test]
fn test_query_get_aum_basic() {
    // Set up mock dependencies with price oracle responses
    let price_oracle_addr = "price_oracle";
    let price_oracle_responses = vec![
        // BTC/BTC price
        (
            price_oracle_addr.to_string(),
            "BTC".to_string(),
            "BTC".to_string(),
            to_json_binary(&CombinedPriceResponse {
                token_0_price: PrecDec::from_str("100000.0").unwrap(),
                token_1_price: PrecDec::from_str("100000.0").unwrap(),
                price_0_to_1: SignedDecimal256::from_str("1.0").unwrap(),
            })
            .unwrap(),
        ), // BTC/USD price
        (
            price_oracle_addr.to_string(),
            "BTC".to_string(),
            "USD".to_string(),
            to_json_binary(&CombinedPriceResponse {
                token_0_price: PrecDec::from_str("100000.0").unwrap(),
                token_1_price: PrecDec::from_str("1.0").unwrap(),
                price_0_to_1: SignedDecimal256::from_str("100000.0").unwrap(),
            })
            .unwrap(),
        ),
        // BTC/ETH price
        (
            price_oracle_addr.to_string(),
            "BTC".to_string(),
            "ETH".to_string(),
            to_json_binary(&CombinedPriceResponse {
                token_0_price: PrecDec::from_str("100000.0").unwrap(),
                token_1_price: PrecDec::from_str("2000.0").unwrap(),
                price_0_to_1: SignedDecimal256::from_str("50").unwrap(),
            })
            .unwrap(),
        ),
        // BTC/USDT price
        (
            price_oracle_addr.to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
            to_json_binary(&CombinedPriceResponse {
                token_0_price: PrecDec::from_str("100000").unwrap(),
                token_1_price: PrecDec::from_str("1.0").unwrap(),
                price_0_to_1: SignedDecimal256::from_str("100000").unwrap(),
            })
            .unwrap(),
        ),
    ];

    let mut deps = custom_mock_dependencies(price_oracle_responses);

    // Set up configuration
    let config = Config {
        owner: Addr::unchecked("admin"),
        price_oracle_contract: Addr::unchecked(price_oracle_addr),
        consensus_data_valid_period: 3600, // 1 hour
        price_max_blocks_old: 100,
        required_binance_positions: vec!["BTCUSDT".to_string()],
        required_binance_spot_assets: vec![
            "BTC".to_string(),
            "ETH".to_string(),
            "USDT".to_string(),
        ],
    };

    CONFIG.save(deps.as_mut().storage, &config).unwrap();

    // Set up consensus state data
    let binance_data = BinanceData {
        unimmr: SignedDecimal256::from_str("0.1").unwrap(),
        positions: vec![Position {
            symbol: "BTCUSDT".to_string(),
            amount: SignedDecimal256::from_str("1.5").unwrap(),
            pnl: SignedDecimal256::from_str("1000.0").unwrap(),
        }],
        um_balance_usdt: SignedDecimal256::from_str("5000.0").unwrap(),
        spot_balances: vec![
            SpotBalance {
                asset: "BTC".to_string(),
                amount: SignedDecimal256::from_str("2.5").unwrap(),
            },
            SpotBalance {
                asset: "ETH".to_string(),
                amount: SignedDecimal256::from_str("20.0").unwrap(),
            },
            SpotBalance {
                asset: "USDT".to_string(),
                amount: SignedDecimal256::from_str("10000.0").unwrap(),
            },
        ],
        pm_account_actual_equity: SignedDecimal256::from_str("80000.0").unwrap(),
        withdrawable_usdt: SignedDecimal256::from_str("3000.0").unwrap(),
    };

    let current_time = 1700000000;
    let oracle_data = OracleData {
        round: 1,
        timestamp: current_time,
        data: binance_data,
    };

    CONSENSUS_STATE
        .last_published_data
        .save(deps.as_mut().storage, &oracle_data)
        .unwrap();

    // Create an environment with a timestamp that's within the valid period
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(current_time + 1800); // 30 minutes after the data timestamp

    // Calculate expected AUM manually
    // 1. PM account equity in BTC: 80000 / 100000 = 0.8 BTC
    // 2. Spot balances in BTC:
    //    - 2.5 BTC = 2.5 BTC
    //    - 20 ETH = 20 / 50 = 0.4 BTC
    //    - 10000 USDT = 10000 / 100000 = 0.1 BTC
    // 3. Total AUM = 0.8 + 2.5 + 0.4 + 0.1 = 3.8 BTC = 380000000 uwBTC
    let expected_aum: Int256 = Int256::from_str("380000000").unwrap();

    // Execute query
    let response: GetAumResponse = query_get_aum(deps.as_ref(), env).unwrap();

    // Verify results
    assert_eq!(response.aum_in_btc, expected_aum);
}

#[test]
fn test_query_get_aum_with_expired_data() {
    // Set up mock dependencies
    let price_oracle_addr = "price_oracle";
    let price_oracle_responses = vec![];
    let mut deps = custom_mock_dependencies(price_oracle_responses);

    // Set up configuration
    let config = Config {
        owner: Addr::unchecked("admin"),
        price_oracle_contract: Addr::unchecked(price_oracle_addr),
        consensus_data_valid_period: 3600, // 1 hour
        price_max_blocks_old: 100,
        required_binance_positions: vec!["BTCUSDT".to_string()],
        required_binance_spot_assets: vec!["BTC".to_string()],
    };

    CONFIG.save(deps.as_mut().storage, &config).unwrap();

    // Set up consensus state data with an old timestamp
    let current_time = 1700000000;
    let binance_data = BinanceData {
        unimmr: SignedDecimal256::from_str("0.1").unwrap(),
        positions: vec![],
        um_balance_usdt: SignedDecimal256::from_str("5000.0").unwrap(),
        spot_balances: vec![],
        pm_account_actual_equity: SignedDecimal256::from_str("80000.0").unwrap(),
        withdrawable_usdt: SignedDecimal256::from_str("3000.0").unwrap(),
    };

    let oracle_data = OracleData {
        round: 1,
        timestamp: 0,
        data: binance_data,
    };

    CONSENSUS_STATE
        .last_published_data
        .save(deps.as_mut().storage, &oracle_data)
        .unwrap();

    // Create an environment with a timestamp that's beyond the valid period
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(current_time + 7200); // 2 hours after the data timestamp

    // Execute query - should fail because data is too old
    let result = query_get_aum(deps.as_ref(), env);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), ContractError::PublishedDataTooOld {},);
}

#[test]
fn test_query_get_aum_with_negative_equity() {
    // Set up mock dependencies with price oracle responses
    let price_oracle_addr = "price_oracle";
    let price_oracle_responses = vec![
        // BTC/USD price
        (
            price_oracle_addr.to_string(),
            "BTC".to_string(),
            "USD".to_string(),
            to_json_binary(&CombinedPriceResponse {
                token_0_price: PrecDec::from_str("40000.0").unwrap(),
                token_1_price: PrecDec::from_str("1.0").unwrap(),
                price_0_to_1: SignedDecimal256::from_str("40000.0").unwrap(),
            })
            .unwrap(),
        ),
        // BTC/ETH price
        (
            price_oracle_addr.to_string(),
            "BTC".to_string(),
            "ETH".to_string(),
            to_json_binary(&CombinedPriceResponse {
                token_0_price: PrecDec::from_str("40000.0").unwrap(),
                token_1_price: PrecDec::from_str("2000.0").unwrap(),
                price_0_to_1: SignedDecimal256::from_str("20").unwrap(),
            })
            .unwrap(),
        ),
    ];

    let mut deps = custom_mock_dependencies(price_oracle_responses);

    // Set up configuration
    let config = Config {
        owner: Addr::unchecked("admin"),
        price_oracle_contract: Addr::unchecked(price_oracle_addr),
        consensus_data_valid_period: 3600,
        price_max_blocks_old: 100,
        required_binance_positions: vec!["BTCUSDT".to_string()],
        required_binance_spot_assets: vec!["ETH".to_string()],
    };

    CONFIG.save(deps.as_mut().storage, &config).unwrap();

    // Set up consensus state data with negative equity
    let binance_data = BinanceData {
        unimmr: SignedDecimal256::from_str("0.1").unwrap(),
        positions: vec![Position {
            symbol: "BTCUSDT".to_string(),
            amount: SignedDecimal256::from_str("1.5").unwrap(),
            pnl: SignedDecimal256::from_str("-10000.0").unwrap(), // Negative PnL
        }],
        um_balance_usdt: SignedDecimal256::from_str("5000.0").unwrap(),
        spot_balances: vec![SpotBalance {
            asset: "ETH".to_string(),
            amount: SignedDecimal256::from_str("10.0").unwrap(),
        }],
        pm_account_actual_equity: SignedDecimal256::from_str("-20000.0").unwrap(), // Negative equity
        withdrawable_usdt: SignedDecimal256::from_str("0.0").unwrap(),
    };

    let current_time = 1700000000;
    let oracle_data = OracleData {
        round: 1,
        timestamp: current_time,
        data: binance_data,
    };

    CONSENSUS_STATE
        .last_published_data
        .save(deps.as_mut().storage, &oracle_data)
        .unwrap();

    // Create an environment with a timestamp that's within the valid period
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(current_time + 1800);

    // Calculate expected AUM manually
    // 1. PM account equity in BTC = -20000 USD / 40000 USD/BTC = -0.5 BTC
    // 2. Spot balances in BTC:
    //    - 10 ETH = 10 / 20 = 0.5 BTC
    // 3. Total AUM = -0.5 + 0.5 = 0 BTC
    let expected_aum = Int256::zero();

    // Execute query
    let response: GetAumResponse = query_get_aum(deps.as_ref(), env).unwrap();

    // Verify results
    assert_eq!(response.aum_in_btc, expected_aum);
}

#[test]
fn test_execute_update_config_admin_only() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    // Set up initial state
    let consensus_config = create_test_consensus_config();
    let contract_config = create_test_contract_config();
    setup_test_state(
        &mut deps.as_mut(),
        &contract_config,
        &consensus_config,
        1,
        1000,
    );

    // Test 1: Non-admin tries to update config (should fail)
    let non_admin_info = message_info("non_admin", &[]);
    let update_config = crate::msg::UpdateConfig {
        owner: Some("new_admin_attempt".to_string()),
        consensus_data_valid_period: None,
        price_data_valid_period: None,
        required_binance_positions: None,
        required_binance_spot_assets: None,
        price_oracle_contract: None,
        oracles: None,
        threshold: None,
        data_delta_ppm: None,
        round_length: None,
    };
    let msg = ExecuteMsg::UpdateConfig {
        new_config: update_config,
    };
    let result = execute(deps.as_mut(), env.clone(), non_admin_info, msg);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContractError::Unauthorized {} => {}
        _ => panic!("Unexpected error"),
    }

    let new_admin = deps.api.addr_make("new_admin");
    let oracle1 = deps.api.addr_make("oracle1");
    let oracle2 = deps.api.addr_make("oracle2");
    let price_oracle = deps.api.addr_make("price_oracle");

    // Test 2: Admin successfully updates config
    let admin_info = message_info("admin", &[]);
    let update_config = crate::msg::UpdateConfig {
        owner: Some(new_admin.to_string()),
        consensus_data_valid_period: Some(7200),
        price_data_valid_period: Some(200),
        required_binance_positions: Some(vec!["ETHUSDT".to_string()]),
        required_binance_spot_assets: Some(vec!["ETH".to_string(), "USDT".to_string()]),
        price_oracle_contract: Some(price_oracle.to_string()),
        oracles: Some(vec![oracle1.to_string(), oracle2.to_string()]),
        threshold: Some(1),
        data_delta_ppm: Some(5000),
        round_length: Some(1800),
    };
    let msg = ExecuteMsg::UpdateConfig {
        new_config: update_config,
    };
    let result = execute(deps.as_mut(), env.clone(), admin_info, msg);
    if result.is_err() {
        println!("Error: {:?}", result.as_ref().unwrap_err());
    }
    assert!(result.is_ok());

    // Verify contract config was updated
    let updated_contract_config = CONFIG.load(deps.as_ref().storage).unwrap();
    assert_eq!(updated_contract_config.owner, new_admin);
    assert_eq!(updated_contract_config.consensus_data_valid_period, 7200);
    assert_eq!(updated_contract_config.price_max_blocks_old, 200);
    assert_eq!(
        updated_contract_config.required_binance_positions,
        vec!["ETHUSDT".to_string()]
    );
    assert_eq!(
        updated_contract_config.required_binance_spot_assets,
        vec!["ETH".to_string(), "USDT".to_string()]
    );
    assert_eq!(updated_contract_config.price_oracle_contract, price_oracle);

    // Verify consensus config was updated
    let updated_consensus_config = CONSENSUS_STATE.config.load(deps.as_ref().storage).unwrap();
    assert_eq!(updated_consensus_config.oracles, vec![oracle1, oracle2]);
    assert_eq!(updated_consensus_config.threshold, 1);
    assert_eq!(updated_consensus_config.data_delta_ppm, 5000);
    assert_eq!(updated_consensus_config.round_length, 1800);
}

#[test]
fn test_execute_update_config_partial_updates() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    // Set up initial state
    let consensus_config = create_test_consensus_config();
    let contract_config = create_test_contract_config();
    setup_test_state(
        &mut deps.as_mut(),
        &contract_config,
        &consensus_config,
        1,
        1000,
    );

    // Test partial update - only contract config fields
    let admin_info = message_info("admin", &[]);
    let update_config = crate::msg::UpdateConfig {
        owner: None,
        consensus_data_valid_period: Some(3600),
        price_data_valid_period: Some(150),
        required_binance_positions: None,
        required_binance_spot_assets: None,
        price_oracle_contract: None,
        oracles: None,
        threshold: None,
        data_delta_ppm: None,
        round_length: None,
    };
    let msg = ExecuteMsg::UpdateConfig {
        new_config: update_config,
    };
    let result = execute(deps.as_mut(), env.clone(), admin_info.clone(), msg);
    assert!(result.is_ok());

    // Verify only specified fields were updated
    let updated_contract_config = CONFIG.load(deps.as_ref().storage).unwrap();
    assert_eq!(updated_contract_config.owner, Addr::unchecked("admin")); // unchanged
    assert_eq!(updated_contract_config.consensus_data_valid_period, 3600); // updated
    assert_eq!(updated_contract_config.price_max_blocks_old, 150); // updated
    assert_eq!(
        updated_contract_config.required_binance_positions,
        vec!["BTCUSDT".to_string()]
    ); // unchanged
    assert_eq!(
        updated_contract_config.required_binance_spot_assets,
        vec!["BTC".to_string(), "USDT".to_string()]
    ); // unchanged

    // Verify consensus config was not updated
    let consensus_config_after = CONSENSUS_STATE.config.load(deps.as_ref().storage).unwrap();
    assert_eq!(consensus_config_after.oracles, consensus_config.oracles);
    assert_eq!(consensus_config_after.threshold, consensus_config.threshold);
    assert_eq!(
        consensus_config_after.data_delta_ppm,
        consensus_config.data_delta_ppm
    );
    assert_eq!(
        consensus_config_after.round_length,
        consensus_config.round_length
    );

    let oracle_a = deps.api.addr_make("oracle_a");
    let oracle_b = deps.api.addr_make("oracle_b");

    // Test partial update - only consensus config fields
    let update_config = crate::msg::UpdateConfig {
        owner: None,
        consensus_data_valid_period: None,
        price_data_valid_period: None,
        required_binance_positions: None,
        required_binance_spot_assets: None,
        price_oracle_contract: None,
        oracles: Some(vec![oracle_a.to_string(), oracle_b.to_string()]),
        threshold: Some(1),
        data_delta_ppm: None,
        round_length: None,
    };
    let msg = ExecuteMsg::UpdateConfig {
        new_config: update_config,
    };
    let result = execute(deps.as_mut(), env.clone(), admin_info, msg);
    assert!(result.is_ok());

    // Verify only specified consensus fields were updated
    let updated_consensus_config = CONSENSUS_STATE.config.load(deps.as_ref().storage).unwrap();
    assert_eq!(updated_consensus_config.oracles, vec![oracle_a, oracle_b]); // updated
    assert_eq!(updated_consensus_config.threshold, 1); // updated
    assert_eq!(
        updated_consensus_config.data_delta_ppm,
        consensus_config.data_delta_ppm
    ); // unchanged
    assert_eq!(
        updated_consensus_config.round_length,
        consensus_config.round_length
    ); // unchanged

    // Verify contract config was not changed from previous update
    let contract_config_after = CONFIG.load(deps.as_ref().storage).unwrap();
    assert_eq!(contract_config_after.consensus_data_valid_period, 3600); // still updated value
    assert_eq!(contract_config_after.price_max_blocks_old, 150); // still updated value
}

#[test]
fn test_execute_update_config_empty_update() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    // Set up initial state
    let consensus_config = create_test_consensus_config();
    let contract_config = create_test_contract_config();
    setup_test_state(
        &mut deps.as_mut(),
        &contract_config,
        &consensus_config,
        1,
        1000,
    );

    // Test empty update (all fields None)
    let admin_info = message_info("admin", &[]);
    let update_config = crate::msg::UpdateConfig {
        owner: None,
        consensus_data_valid_period: None,
        price_data_valid_period: None,
        required_binance_positions: None,
        required_binance_spot_assets: None,
        price_oracle_contract: None,
        oracles: None,
        threshold: None,
        data_delta_ppm: None,
        round_length: None,
    };
    let msg = ExecuteMsg::UpdateConfig {
        new_config: update_config,
    };
    let result = execute(deps.as_mut(), env.clone(), admin_info, msg);
    assert!(result.is_ok());

    // Verify nothing was changed
    let contract_config_after = CONFIG.load(deps.as_ref().storage).unwrap();
    assert_eq!(contract_config_after, contract_config);

    let consensus_config_after = CONSENSUS_STATE.config.load(deps.as_ref().storage).unwrap();
    assert_eq!(consensus_config_after, consensus_config);
}

#[test]
fn test_execute_update_config_admin_change() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    // Set up initial state
    let consensus_config = create_test_consensus_config();
    let contract_config = create_test_contract_config();
    setup_test_state(
        &mut deps.as_mut(),
        &contract_config,
        &consensus_config,
        1,
        1000,
    );

    let new_admin = deps.api.addr_make("new_admin");
    // Test admin change
    let admin_info = message_info("admin", &[]);
    let update_config = crate::msg::UpdateConfig {
        owner: Some(new_admin.to_string()),
        consensus_data_valid_period: None,
        price_data_valid_period: None,
        required_binance_positions: None,
        required_binance_spot_assets: None,
        price_oracle_contract: None,
        oracles: None,
        threshold: None,
        data_delta_ppm: None,
        round_length: None,
    };
    let msg = ExecuteMsg::UpdateConfig {
        new_config: update_config,
    };
    let result = execute(deps.as_mut(), env.clone(), admin_info, msg);
    assert!(result.is_ok());

    // Verify admin was changed
    let updated_contract_config = CONFIG.load(deps.as_ref().storage).unwrap();
    assert_eq!(updated_contract_config.owner, new_admin);

    // Test that old admin can no longer update config
    let old_admin_info = message_info("admin", &[]);
    let update_config = crate::msg::UpdateConfig {
        owner: Some("another_admin".to_string()),
        consensus_data_valid_period: None,
        price_data_valid_period: None,
        required_binance_positions: None,
        required_binance_spot_assets: None,
        price_oracle_contract: None,
        oracles: None,
        threshold: None,
        data_delta_ppm: None,
        round_length: None,
    };
    let msg = ExecuteMsg::UpdateConfig {
        new_config: update_config,
    };
    let result = execute(deps.as_mut(), env.clone(), old_admin_info, msg);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContractError::Unauthorized => {}
        _ => panic!("Unexpected error"),
    }

    // Test that new admin can update config
    let new_admin_info = message_info(new_admin.as_ref(), &[]);
    let update_config = crate::msg::UpdateConfig {
        owner: None,
        consensus_data_valid_period: Some(5000),
        price_data_valid_period: None,
        required_binance_positions: None,
        required_binance_spot_assets: None,
        price_oracle_contract: None,
        oracles: None,
        threshold: None,
        data_delta_ppm: None,
        round_length: None,
    };
    let msg = ExecuteMsg::UpdateConfig {
        new_config: update_config,
    };
    let result = execute(deps.as_mut(), env.clone(), new_admin_info, msg);
    assert!(result.is_ok());

    // Verify the update was successful
    let final_contract_config = CONFIG.load(deps.as_ref().storage).unwrap();
    assert_eq!(final_contract_config.consensus_data_valid_period, 5000);
}

#[test]
fn test_query_get_aum_with_large_values() {
    // Set up mock dependencies with price oracle responses
    let price_oracle_addr = "price_oracle";
    let price_oracle_responses = vec![
        // BTC/BTC price
        (
            price_oracle_addr.to_string(),
            "BTC".to_string(),
            "BTC".to_string(),
            to_json_binary(&CombinedPriceResponse {
                token_0_price: PrecDec::from_str("100000.0").unwrap(),
                token_1_price: PrecDec::from_str("100000.0").unwrap(),
                price_0_to_1: SignedDecimal256::from_str("1.0").unwrap(),
            })
            .unwrap(),
        ),
        // BTC/USD price
        (
            price_oracle_addr.to_string(),
            "BTC".to_string(),
            "USD".to_string(),
            to_json_binary(&CombinedPriceResponse {
                token_0_price: PrecDec::from_str("100000.0").unwrap(),
                token_1_price: PrecDec::from_str("1.0").unwrap(),
                price_0_to_1: SignedDecimal256::from_str("100000.0").unwrap(),
            })
            .unwrap(),
        ),
        // BTC/ETH price
        (
            price_oracle_addr.to_string(),
            "BTC".to_string(),
            "ETH".to_string(),
            to_json_binary(&CombinedPriceResponse {
                token_0_price: PrecDec::from_str("100000").unwrap(),
                token_1_price: PrecDec::from_str("2500.0").unwrap(),
                price_0_to_1: SignedDecimal256::from_str("40").unwrap(),
            })
            .unwrap(),
        ),
    ];

    let mut deps = custom_mock_dependencies(price_oracle_responses);

    // Set up configuration
    let config = Config {
        owner: Addr::unchecked("admin"),
        price_oracle_contract: Addr::unchecked(price_oracle_addr),
        consensus_data_valid_period: 3600,
        price_max_blocks_old: 100,
        required_binance_positions: vec!["BTCUSDT".to_string()],
        required_binance_spot_assets: vec!["BTC".to_string(), "ETH".to_string()],
    };

    CONFIG.save(deps.as_mut().storage, &config).unwrap();

    // Set up consensus state data with large values
    let binance_data = BinanceData {
        unimmr: SignedDecimal256::from_str("0.1").unwrap(),
        positions: vec![],
        um_balance_usdt: SignedDecimal256::from_str("5000.0").unwrap(),
        spot_balances: vec![
            SpotBalance {
                asset: "BTC".to_string(),
                amount: SignedDecimal256::from_str("1000.0").unwrap(), // 1000 BTC
            },
            SpotBalance {
                asset: "ETH".to_string(),
                amount: SignedDecimal256::from_str("20000.0").unwrap(), // 20000 ETH
            },
        ],
        pm_account_actual_equity: SignedDecimal256::from_str("40000000.0").unwrap(), // 40M USD
        withdrawable_usdt: SignedDecimal256::from_str("1000000.0").unwrap(),
    };

    let current_time = 1700000000;
    let oracle_data = OracleData {
        round: 1,
        timestamp: current_time,
        data: binance_data,
    };

    CONSENSUS_STATE
        .last_published_data
        .save(deps.as_mut().storage, &oracle_data)
        .unwrap();

    // Create an environment with a timestamp that's within the valid period
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(current_time + 1800);

    // Calculate expected AUM manually
    // 1. PM account equity in BTC = 40000000 USD / 100000 USD/BTC = 400 BTC
    // 2. Spot balances in BTC:
    //    - 1000 BTC = 1000 BTC
    //    - 20000 ETH = 20000 / 40 = 500 BTC
    // 3. Total AUM = 400 + 1000 + 500 = 1900 BTC = 190000000000
    let expected_aum: Int256 = Int256::from_str("190000000000").unwrap();

    // Execute query
    let response: GetAumResponse = query_get_aum(deps.as_ref(), env).unwrap();

    // Verify results
    assert_eq!(response.aum_in_btc, expected_aum);
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CombinedPriceResponseWithPrecDec {
    pub token_0_price: PrecDec,
    pub token_1_price: PrecDec,
    pub price_0_to_1: PrecDec,
}

#[test]
fn test_query_aum_with_high_precision_prices_from_oracle() {
    // Set up mock dependencies with price oracle responses
    let price_oracle_addr = "price_oracle";
    let price_oracle_responses = vec![
        // BTC/BTC price
        (
            price_oracle_addr.to_string(),
            "BTC".to_string(),
            "BTC".to_string(),
            to_json_binary(&CombinedPriceResponseWithPrecDec {
                token_0_price: PrecDec::from_str("100000.0").unwrap(),
                token_1_price: PrecDec::from_str("100000.0").unwrap(),
                price_0_to_1: PrecDec::from_str("1.0").unwrap(),
            })
            .unwrap(),
        ), // BTC/USD price
        (
            price_oracle_addr.to_string(),
            "BTC".to_string(),
            "USD".to_string(),
            to_json_binary(&CombinedPriceResponseWithPrecDec {
                token_0_price: PrecDec::from_str("100000.0").unwrap(),
                token_1_price: PrecDec::from_str("1.0").unwrap(),
                price_0_to_1: PrecDec::from_str("100000.0").unwrap(),
            })
            .unwrap(),
        ),
        // BTC/ETH price
        (
            price_oracle_addr.to_string(),
            "BTC".to_string(),
            "ETH".to_string(),
            to_json_binary(&CombinedPriceResponseWithPrecDec {
                token_0_price: PrecDec::from_str("100000.0").unwrap(),
                token_1_price: PrecDec::from_str("2000.0").unwrap(),
                // this must be parsed successfully and the precision must be dropped just to 50.0
                price_0_to_1: PrecDec::from_str("50.00000000000000000000001").unwrap(),
            })
            .unwrap(),
        ),
        // BTC/USDT price
        (
            price_oracle_addr.to_string(),
            "BTC".to_string(),
            "USDT".to_string(),
            to_json_binary(&CombinedPriceResponseWithPrecDec {
                token_0_price: PrecDec::from_str("100000").unwrap(),
                token_1_price: PrecDec::from_str("1.0").unwrap(),
                price_0_to_1: PrecDec::from_str("100000").unwrap(),
            })
            .unwrap(),
        ),
    ];

    let mut deps = custom_mock_dependencies(price_oracle_responses);

    // Set up configuration
    let config = Config {
        owner: Addr::unchecked("admin"),
        price_oracle_contract: Addr::unchecked(price_oracle_addr),
        consensus_data_valid_period: 3600, // 1 hour
        price_max_blocks_old: 100,
        required_binance_positions: vec!["BTCUSDT".to_string()],
        required_binance_spot_assets: vec![
            "BTC".to_string(),
            "ETH".to_string(),
            "USDT".to_string(),
        ],
    };

    CONFIG.save(deps.as_mut().storage, &config).unwrap();

    // Set up consensus state data
    let binance_data = BinanceData {
        unimmr: SignedDecimal256::from_str("0.1").unwrap(),
        positions: vec![Position {
            symbol: "BTCUSDT".to_string(),
            amount: SignedDecimal256::from_str("1.5").unwrap(),
            pnl: SignedDecimal256::from_str("1000.0").unwrap(),
        }],
        um_balance_usdt: SignedDecimal256::from_str("5000.0").unwrap(),
        spot_balances: vec![
            SpotBalance {
                asset: "BTC".to_string(),
                amount: SignedDecimal256::from_str("2.5").unwrap(),
            },
            SpotBalance {
                asset: "ETH".to_string(),
                amount: SignedDecimal256::from_str("20.0").unwrap(),
            },
            SpotBalance {
                asset: "USDT".to_string(),
                amount: SignedDecimal256::from_str("10000.0").unwrap(),
            },
        ],
        pm_account_actual_equity: SignedDecimal256::from_str("80000.0").unwrap(),
        withdrawable_usdt: SignedDecimal256::from_str("3000.0").unwrap(),
    };

    let current_time = 1700000000;
    let oracle_data = OracleData {
        round: 1,
        timestamp: current_time,
        data: binance_data,
    };

    CONSENSUS_STATE
        .last_published_data
        .save(deps.as_mut().storage, &oracle_data)
        .unwrap();

    // Create an environment with a timestamp that's within the valid period
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(current_time + 1800); // 30 minutes after the data timestamp

    // Calculate expected AUM manually
    // 1. PM account equity in BTC: 80000 / 100000 = 0.8 BTC
    // 2. Spot balances in BTC:
    //    - 2.5 BTC = 2.5 BTC
    //    - 20 ETH = 20 / 50 = 0.4 BTC
    //    - 10000 USDT = 10000 / 100000 = 0.1 BTC
    // 3. Total AUM = 0.8 + 2.5 + 0.4 + 0.1 = 3.8 BTC = 380000000 uwBTC
    let expected_aum = Int256::from_str("380000000").unwrap();

    // Execute query
    let response: GetAumResponse = query_get_aum(deps.as_ref(), env).unwrap();

    // Verify results
    assert_eq!(response.aum_in_btc, expected_aum);
}
