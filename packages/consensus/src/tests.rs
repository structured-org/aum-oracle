use crate::consensus::{consensus_on_items, Config, ConsensusData, Round, State};
use crate::error::ConsensusError;
use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{Addr, SignedDecimal256, Timestamp};
use serde::{Deserialize, Serialize};

// Mock data structure for testing OracleData<T>
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct MockData {
    pub value1: SignedDecimal256,
    pub value2: SignedDecimal256,
    pub value3: SignedDecimal256,
    pub value4: SignedDecimal256,
}

impl ConsensusData for MockData {
    // Consensus function for MockData
    fn try_consensus(data: &[MockData], threshold: usize, delta_ppm: u64) -> Option<MockData> {
        if data.len() < threshold {
            return None;
        }

        // Try to reach consensus on each field
        let value1 = consensus_on_items(
            &data.iter().map(|d| d.value1).collect::<Vec<_>>(),
            threshold,
            delta_ppm,
        );
        let value2 = consensus_on_items(
            &data.iter().map(|d| d.value2).collect::<Vec<_>>(),
            threshold,
            delta_ppm,
        );
        let value3 = consensus_on_items(
            &data.iter().map(|d| d.value3).collect::<Vec<_>>(),
            threshold,
            delta_ppm,
        );
        let value4 = consensus_on_items(
            &data.iter().map(|d| d.value4).collect::<Vec<_>>(),
            threshold,
            delta_ppm,
        );

        // If any field fails to reach consensus, the whole consensus fails
        if value1.is_none() || value2.is_none() || value3.is_none() || value4.is_none() {
            return None;
        }

        Some(MockData {
            value1: value1.unwrap(),
            value2: value2.unwrap(),
            value3: value3.unwrap(),
            value4: value4.unwrap(),
        })
    }
}

// Helper function to create a test config
fn create_test_config() -> Config {
    Config {
        messengers: vec![
            Addr::unchecked("oracle1"),
            Addr::unchecked("oracle2"),
            Addr::unchecked("oracle3"),
        ],
        threshold: 2,
        data_delta_ppm: 10000, // 1%
        round_length: 3600,    // 1 hour
    }
}

// Helper function to create a test MockData object
fn create_test_data(
    value1: SignedDecimal256,
    value2: SignedDecimal256,
    value3: SignedDecimal256,
    value4: SignedDecimal256,
) -> MockData {
    MockData {
        value1,
        value2,
        value3,
        value4,
    }
}

#[test]
fn test_try_consensus() {
    // Test 1: Empty data array should return None
    {
        let config = create_test_config();
        let empty_data: Vec<MockData> = vec![];
        assert!(MockData::try_consensus(
            &empty_data,
            config.threshold as usize,
            config.data_delta_ppm
        )
        .is_none());
    }

    // Test 2: Data array with fewer elements than threshold should return None
    {
        let config = create_test_config();
        let single_data = vec![create_test_data(
            SignedDecimal256::from_ratio(5, 10),
            SignedDecimal256::from_ratio(1000, 1),
            SignedDecimal256::from_ratio(2000, 1),
            SignedDecimal256::from_ratio(500, 1),
        )];
        assert!(MockData::try_consensus(
            &single_data,
            config.threshold as usize,
            config.data_delta_ppm
        )
        .is_none());
    }

    // Test 3: Data within acceptable delta should reach consensus
    {
        let config = create_test_config();
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
            MockData::try_consensus(&data, config.threshold as usize, config.data_delta_ppm);
        assert!(consensus.is_some());

        let result = consensus.unwrap();
        assert_eq!(result.value1, SignedDecimal256::from_ratio(5025, 10000)); // median of 0.5 and 0.505
        assert_eq!(result.value2, SignedDecimal256::from_ratio(10025, 10)); // median of 1000 and 1005
        assert_eq!(result.value3, SignedDecimal256::from_ratio(2005, 1)); // median of 2000 and 2010
        assert_eq!(result.value4, SignedDecimal256::from_ratio(5025, 10)); // median of 500 and 505
    }

    // Test 4: Data outside acceptable delta should not reach consensus
    {
        let mut config = create_test_config();
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

        assert!(MockData::try_consensus(
            &data_divergent,
            config.threshold as usize,
            config.data_delta_ppm
        )
        .is_none());
    }

    // Test 5: Consensus with more than threshold oracles
    {
        let config = create_test_config();
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

        let consensus = MockData::try_consensus(
            &data_multiple,
            config.threshold as usize,
            config.data_delta_ppm,
        );
        assert!(consensus.is_some());

        let result = consensus.unwrap();
        assert_eq!(result.value1, SignedDecimal256::from_ratio(503, 1000));
        assert_eq!(result.value2, SignedDecimal256::from_ratio(1003, 1));
        assert_eq!(result.value3, SignedDecimal256::from_ratio(2005, 1));
        assert_eq!(result.value4, SignedDecimal256::from_ratio(503, 1));
    }

    // Test 6: Consensus with some outliers
    {
        let config = create_test_config();
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

        let consensus = MockData::try_consensus(
            &data_with_outliers,
            config.threshold as usize,
            config.data_delta_ppm,
        );
        assert!(consensus.is_some());

        // The outlier should be excluded from the consensus
        let result = consensus.unwrap();
        assert_eq!(result.value1, SignedDecimal256::from_ratio(5025, 10000)); // median of 0.5 and 0.505 (outlier excluded)
    }
}

#[test]
fn test_round_operations() {
    let mut env = mock_env();
    let start_time = 1000;
    env.block.time = Timestamp::from_seconds(start_time);

    let round_length = 3600; // 1 hour

    // Test Round::is_passed
    let round = Round {
        round: 1,
        start: start_time,
    };
    assert!(
        !round.is_passed(&env, round_length),
        "Round should not be passed at start time"
    );

    env.block.time = Timestamp::from_seconds(start_time + round_length - 1);
    assert!(
        !round.is_passed(&env, round_length),
        "Round should not be passed just before end time"
    );

    env.block.time = Timestamp::from_seconds(start_time + round_length);
    assert!(
        round.is_passed(&env, round_length),
        "Round should be passed at end time"
    );

    env.block.time = Timestamp::from_seconds(start_time + round_length + 1);
    assert!(
        round.is_passed(&env, round_length),
        "Round should be passed after end time"
    );

    // Test Round::rounds_passed
    env.block.time = Timestamp::from_seconds(start_time);
    assert_eq!(
        round.rounds_passed(&env, round_length),
        0,
        "No rounds should have passed at start time"
    );

    env.block.time = Timestamp::from_seconds(start_time + round_length - 1);
    assert_eq!(
        round.rounds_passed(&env, round_length),
        0,
        "No rounds should have passed just before end time"
    );

    env.block.time = Timestamp::from_seconds(start_time + round_length);
    assert_eq!(
        round.rounds_passed(&env, round_length),
        1,
        "One round should have passed at end time"
    );

    env.block.time = Timestamp::from_seconds(start_time + 3 * round_length + 1);
    assert_eq!(
        round.rounds_passed(&env, round_length),
        3,
        "Three rounds should have passed"
    );

    // Test Round::add_rounds
    let new_round = round.add_rounds(round_length, 2);
    assert_eq!(new_round.round, 3, "Round number should be increased by 2");
    assert_eq!(
        new_round.start,
        start_time + 2 * round_length,
        "Start time should be increased by 2 round lengths"
    );

    // Test Round::next_round
    let next_round = round.next_round(round_length);
    assert_eq!(next_round.round, 2, "Round number should be increased by 1");
    assert_eq!(
        next_round.start,
        start_time + round_length,
        "Start time should be increased by 1 round length"
    );
}

#[test]
fn test_consensus_on_items() {
    struct TestCase {
        name: &'static str,
        items: Vec<SignedDecimal256>,
        threshold: usize,
        delta: u64,
        expected: Option<SignedDecimal256>,
    }

    let test_cases = vec![
        TestCase {
            name: "Empty array should return None",
            items: vec![],
            threshold: 1,
            delta: 10000,
            expected: None,
        },
        TestCase {
            name: "Array smaller than threshold should return None",
            items: vec![SignedDecimal256::from_ratio(5, 10)],
            threshold: 2,
            delta: 0,
            expected: None,
        },
        TestCase {
            name: "Values within delta should reach consensus",
            items: vec![
                SignedDecimal256::from_ratio(5, 10),
                SignedDecimal256::from_ratio(505, 1000),
                SignedDecimal256::from_ratio(51, 100),
            ],
            threshold: 2,
            delta: 10000,
            expected: Some(SignedDecimal256::from_ratio(5025, 10000)),
        },
        TestCase {
            name: "Values outside delta should not reach consensus",
            items: vec![
                SignedDecimal256::from_ratio(5, 10),
                SignedDecimal256::from_ratio(6, 10), // 20% difference
                SignedDecimal256::from_ratio(45, 100),
            ],
            threshold: 2,
            delta: 10000,
            expected: None,
        },
        TestCase {
            name: "Values that are all within delta but with zero as corner value",
            items: vec![
                SignedDecimal256::from_ratio(0, 1),
                SignedDecimal256::from_ratio(-9, 1),
                SignedDecimal256::from_ratio(-8, 1),
                SignedDecimal256::from_ratio(-7, 1),
                SignedDecimal256::from_ratio(-6, 1),
                SignedDecimal256::from_ratio(-5, 1),
                SignedDecimal256::from_ratio(-4, 1),
                SignedDecimal256::from_ratio(-3, 1),
                SignedDecimal256::from_ratio(-2, 1),
                SignedDecimal256::from_ratio(-1, 1),
            ],
            threshold: 7,
            delta: 1000000,
            expected: Some(SignedDecimal256::from_ratio(-45, 10)),
        },
        TestCase {
            name: "No consensus with 10% delta and incremented numbers",
            items: vec![
                SignedDecimal256::from_ratio(1, 1),
                SignedDecimal256::from_ratio(2, 1),
                SignedDecimal256::from_ratio(3, 1),
                SignedDecimal256::from_ratio(4, 1),
                SignedDecimal256::from_ratio(5, 1),
                SignedDecimal256::from_ratio(6, 1),
                SignedDecimal256::from_ratio(7, 1),
                SignedDecimal256::from_ratio(8, 1),
                SignedDecimal256::from_ratio(9, 1),
                SignedDecimal256::from_ratio(10, 1),
            ],
            threshold: 7,
            delta: 100000,
            expected: None,
        },
        TestCase {
            name: "Consensus with a couple of large wrong numbers (threshold is reached)",
            items: vec![
                SignedDecimal256::from_ratio(100000, 1),
                SignedDecimal256::from_ratio(100000, 1),
                SignedDecimal256::from_ratio(100000, 1),
                SignedDecimal256::from_ratio(1, 1),
                SignedDecimal256::from_ratio(101, 100),
                SignedDecimal256::from_ratio(102, 100),
                SignedDecimal256::from_ratio(103, 100),
                SignedDecimal256::from_ratio(104, 100),
                SignedDecimal256::from_ratio(105, 100),
                SignedDecimal256::from_ratio(106, 100),
            ],
            threshold: 7,
            delta: 100000,
            expected: Some(SignedDecimal256::from_ratio(103, 100)),
        },
        TestCase {
            name: "No consensus with a couple of large wrong numbers (threshold is reached not)",
            items: vec![
                SignedDecimal256::from_ratio(100000, 1),
                SignedDecimal256::from_ratio(100000, 1),
                SignedDecimal256::from_ratio(100000, 1),
                SignedDecimal256::from_ratio(100000, 1),
                SignedDecimal256::from_ratio(1, 1),
                SignedDecimal256::from_ratio(101, 100),
                SignedDecimal256::from_ratio(102, 100),
                SignedDecimal256::from_ratio(103, 100),
                SignedDecimal256::from_ratio(104, 100),
                SignedDecimal256::from_ratio(105, 100),
            ],
            threshold: 7,
            delta: 100000,
            expected: None,
        },
    ];

    for tc in test_cases {
        println!("Running test case: {}", tc.name);
        assert_eq!(
            consensus_on_items(&tc.items, tc.threshold, tc.delta),
            tc.expected,
        )
    }
}

// Helper function to setup storage with config and current round
fn setup_test_state(
    deps: &mut cosmwasm_std::testing::MockStorage,
    config: &Config,
    round: u64,
    start_time: u64,
) -> State<MockData> {
    let state = State::default();
    state.config.save(deps, config).unwrap();
    state
        .pending_round
        .save(
            deps,
            &Round {
                round,
                start: start_time,
            },
        )
        .unwrap();
    state
}

#[test]
fn test_state_publish_data() {
    // Set up test environment
    let mut deps = cosmwasm_std::testing::MockStorage::new();
    let mut env = mock_env();
    let start_time = 1000;
    env.block.time = Timestamp::from_seconds(start_time);

    // Set up initial state
    let config = create_test_config();
    let round = Round {
        round: 1,
        start: start_time,
    };
    let state = setup_test_state(&mut deps, &config, round.round, round.start);

    // Create test data
    let test_data = create_test_data(
        SignedDecimal256::from_ratio(5, 10),
        SignedDecimal256::from_ratio(1000, 1),
        SignedDecimal256::from_ratio(2000, 1),
        SignedDecimal256::from_ratio(500, 1),
    );

    // Test 1: Oracle publishes data
    let oracle1 = Addr::unchecked("oracle1");
    let result = state.publish_data(&mut deps, &env, oracle1.clone(), test_data.clone());
    assert!(result.is_ok(), "Oracle should be able to publish data");

    // Test 2: Oracle tries to publish data again for the same round (should fail)
    let result = state.publish_data(&mut deps, &env, oracle1.clone(), test_data.clone());
    assert!(
        result.is_err(),
        "Oracle should not be able to publish data twice for the same round"
    );
    assert_eq!(result.err().unwrap(), ConsensusError::DoubleSubmission {});

    // Test 3: Another oracle publishes data
    let oracle2 = Addr::unchecked("oracle2");
    let test_data2 = create_test_data(
        SignedDecimal256::from_ratio(505, 1000),
        SignedDecimal256::from_ratio(1005, 1),
        SignedDecimal256::from_ratio(2010, 1),
        SignedDecimal256::from_ratio(505, 1),
    );
    let result = state.publish_data(&mut deps, &env, oracle2.clone(), test_data2);
    assert!(
        result.is_ok(),
        "Second oracle should be able to publish data"
    );

    // Test 4: Third oracle publishes data (all oracles have now published)
    let oracle3 = Addr::unchecked("oracle3");
    let test_data3 = create_test_data(
        SignedDecimal256::from_ratio(503, 1000),
        SignedDecimal256::from_ratio(1003, 1),
        SignedDecimal256::from_ratio(2005, 1),
        SignedDecimal256::from_ratio(503, 1),
    );
    let result = state.publish_data(&mut deps, &env, oracle3.clone(), test_data3);
    assert!(
        result.is_ok(),
        "Third oracle should be able to publish data"
    );

    // Test 5: Verify consensus was reached and data was published
    let published_data = state.last_published_data.may_load(&deps).unwrap();
    assert!(
        published_data.is_some(),
        "Consensus data should be published"
    );
    let data = published_data.unwrap();
    assert_eq!(data.round, 1, "Published data should be for round 1");
    assert_eq!(
        data.data.value1,
        SignedDecimal256::from_ratio(503, 1000),
        "Consensus value1 should be correct"
    );
    assert_eq!(
        data.data.value2,
        SignedDecimal256::from_ratio(1003, 1),
        "Consensus value2 should be correct"
    );
    assert_eq!(
        data.data.value3,
        SignedDecimal256::from_ratio(2005, 1),
        "Consensus value3 should be correct"
    );
    assert_eq!(
        data.data.value4,
        SignedDecimal256::from_ratio(503, 1),
        "Consensus value4 should be correct"
    );
}

#[test]
fn test_state_round_advancement() {
    // Set up test environment
    let mut deps = cosmwasm_std::testing::MockStorage::new();
    let mut env = mock_env();
    let start_time = 1000;
    env.block.time = Timestamp::from_seconds(start_time);

    // Set up initial state
    let config = create_test_config();
    let round = Round {
        round: 1,
        start: start_time,
    };
    let state = setup_test_state(&mut deps, &config, round.round, round.start);

    // Test 1: Advance time to trigger round change
    env.block.time = Timestamp::from_seconds(start_time + config.round_length + 1);

    // Oracle publishes data for the new round
    let oracle1 = Addr::unchecked("oracle1");
    let test_data = create_test_data(
        SignedDecimal256::from_ratio(5, 10),
        SignedDecimal256::from_ratio(1000, 1),
        SignedDecimal256::from_ratio(2000, 1),
        SignedDecimal256::from_ratio(500, 1),
    );
    let result = state.publish_data(&mut deps, &env, oracle1.clone(), test_data);
    assert!(
        result.is_ok(),
        "Oracle should be able to publish data for the new round"
    );

    // Verify round has advanced
    let pending_round = state.pending_round.load(&deps).unwrap();
    assert_eq!(pending_round.round, 2, "Round should have advanced");
    assert_eq!(
        pending_round.start,
        start_time + config.round_length,
        "Round start time should be updated"
    );

    // Test 2: Advance time by multiple rounds
    env.block.time = Timestamp::from_seconds(start_time + 3 * config.round_length + 1);

    // Oracle publishes data for the new round
    let test_data_multi = create_test_data(
        SignedDecimal256::from_ratio(51, 100),
        SignedDecimal256::from_ratio(1010, 1),
        SignedDecimal256::from_ratio(2020, 1),
        SignedDecimal256::from_ratio(510, 1),
    );
    let result = state.publish_data(&mut deps, &env, oracle1.clone(), test_data_multi);
    assert!(
        result.is_ok(),
        "Oracle should be able to publish data after multiple round advances"
    );

    // Verify round has advanced by multiple steps
    let pending_round = state.pending_round.load(&deps).unwrap();
    assert_eq!(
        pending_round.round, 4,
        "Round should have advanced by multiple steps"
    );
    assert_eq!(
        pending_round.start,
        start_time + 3 * config.round_length,
        "Round start time should be updated"
    );
}

#[test]
fn test_state_get_last_published_data() {
    // Set up test environment
    let mut deps = cosmwasm_std::testing::MockStorage::new();
    let mut env = mock_env();
    let start_time = 1000;
    env.block.time = Timestamp::from_seconds(start_time);

    // Set up initial state
    let config = create_test_config();
    let round = Round {
        round: 1,
        start: start_time,
    };
    let state = setup_test_state(&mut deps, &config, round.round, round.start);

    // Test 1: No data published yet
    let result = state.get_last_published_data(&env, &deps).unwrap();
    assert!(result.is_none(), "No data should be published initially");

    // Test 2: Publish data from all oracles
    let oracle1 = Addr::unchecked("oracle1");
    let test_data1 = create_test_data(
        SignedDecimal256::from_ratio(5, 10),
        SignedDecimal256::from_ratio(1000, 1),
        SignedDecimal256::from_ratio(2000, 1),
        SignedDecimal256::from_ratio(500, 1),
    );
    state
        .publish_data(&mut deps, &env, oracle1.clone(), test_data1)
        .unwrap();

    let oracle2 = Addr::unchecked("oracle2");
    let test_data2 = create_test_data(
        SignedDecimal256::from_ratio(505, 1000),
        SignedDecimal256::from_ratio(1005, 1),
        SignedDecimal256::from_ratio(2010, 1),
        SignedDecimal256::from_ratio(505, 1),
    );
    state
        .publish_data(&mut deps, &env, oracle2.clone(), test_data2)
        .unwrap();

    let oracle3 = Addr::unchecked("oracle3");
    let test_data3 = create_test_data(
        SignedDecimal256::from_ratio(503, 1000),
        SignedDecimal256::from_ratio(1003, 1),
        SignedDecimal256::from_ratio(2005, 1),
        SignedDecimal256::from_ratio(503, 1),
    );
    state
        .publish_data(&mut deps, &env, oracle3.clone(), test_data3)
        .unwrap();

    // Verify data is published
    let result = state.get_last_published_data(&env, &deps).unwrap();
    assert!(
        result.is_some(),
        "Data should be published after all oracles submit"
    );
    let data = result.unwrap();
    assert_eq!(data.round, 1, "Published data should be for round 1");
    assert_eq!(
        data.data.value1,
        SignedDecimal256::from_ratio(503, 1000),
        "Consensus value1 should be correct"
    );

    // Test 3: Advance time to next round but don't publish enough data
    env.block.time = Timestamp::from_seconds(start_time + config.round_length + 1);

    // Only one oracle publishes data (below threshold)
    let test_data4 = create_test_data(
        SignedDecimal256::from_ratio(52, 100),
        SignedDecimal256::from_ratio(1020, 1),
        SignedDecimal256::from_ratio(2040, 1),
        SignedDecimal256::from_ratio(520, 1),
    );
    state
        .publish_data(&mut deps, &env, oracle1.clone(), test_data4)
        .unwrap();

    // Verify we still get the last published data from round 1
    let result = state.get_last_published_data(&env, &deps).unwrap();
    assert!(result.is_some(), "Should still return last published data");
    let data = result.unwrap();
    assert_eq!(data.round, 1, "Should still return data from round 1");

    // Test 4: Publish enough data in round 2 to reach threshold
    let test_data5 = create_test_data(
        SignedDecimal256::from_ratio(525, 1000),
        SignedDecimal256::from_ratio(1025, 1),
        SignedDecimal256::from_ratio(2050, 1),
        SignedDecimal256::from_ratio(525, 1),
    );
    state
        .publish_data(&mut deps, &env, oracle2.clone(), test_data5)
        .unwrap();

    // But verify we still get the last published data from round 1 because round is not passed yet
    let result = state.get_last_published_data(&env, &deps).unwrap();
    assert!(result.is_some(), "Should still return last published data");
    let data = result.unwrap();
    assert_eq!(data.round, 1, "Should still return data from round 1");

    // Advance time to next round but don't publish more data
    env.block.time = Timestamp::from_seconds(start_time + config.round_length * 2 + 1);

    // Verify we get the consensus data for round 2 because time is passed
    let result = state.get_last_published_data(&env, &deps).unwrap();
    assert!(result.is_some(), "Should return consensus data for round 2");
    let data = result.unwrap();
    assert_eq!(data.round, 2, "Should return data from round 2");
    assert_eq!(
        data.data.value1,
        SignedDecimal256::from_ratio(5225, 10000),
        "Consensus value1 should be correct"
    );

    // Test scenario: Messengers X and Y in config but no data from them in pending_data
    {
        let mut deps_scenario1 = cosmwasm_std::testing::MockStorage::new();
        let config_with_inactive = Config {
            messengers: vec![
                Addr::unchecked("oracle1"),
                Addr::unchecked("oracle2"),
                Addr::unchecked("oracle_inactive1"), // In config but no data
                Addr::unchecked("oracle_inactive2"), // In config but no data
            ],
            threshold: 2,
            data_delta_ppm: 10000,
            round_length: 3600,
        };
        let state_scenario1 =
            setup_test_state(&mut deps_scenario1, &config_with_inactive, 1, start_time);

        // Reset time
        env.block.time = Timestamp::from_seconds(start_time);

        // Only oracle1 and oracle2 publish data (oracle_inactive1 and oracle_inactive2 don't)
        let test_data_s1_1 = create_test_data(
            SignedDecimal256::from_ratio(5, 10),
            SignedDecimal256::from_ratio(1000, 1),
            SignedDecimal256::from_ratio(2000, 1),
            SignedDecimal256::from_ratio(500, 1),
        );
        state_scenario1
            .publish_data(
                &mut deps_scenario1,
                &env,
                Addr::unchecked("oracle1"),
                test_data_s1_1,
            )
            .unwrap();

        let test_data_s1_2 = create_test_data(
            SignedDecimal256::from_ratio(505, 1000),
            SignedDecimal256::from_ratio(1005, 1),
            SignedDecimal256::from_ratio(2010, 1),
            SignedDecimal256::from_ratio(505, 1),
        );
        state_scenario1
            .publish_data(
                &mut deps_scenario1,
                &env,
                Addr::unchecked("oracle2"),
                test_data_s1_2,
            )
            .unwrap();

        // Advance time to trigger consensus since not all messengers submitted data
        env.block.time =
            Timestamp::from_seconds(start_time + config_with_inactive.round_length + 1);

        // Test that get_last_published_data can form consensus from available data
        let result = state_scenario1
            .get_last_published_data(&env, &deps_scenario1)
            .unwrap();
        assert!(
            result.is_some(),
            "Consensus should be reached even when some config messengers don't provide data"
        );
        let consensus = result.unwrap();
        assert_eq!(consensus.round, 1);
        assert_eq!(
            consensus.data.value1,
            SignedDecimal256::from_ratio(5025, 10000)
        ); // median of oracle1 and oracle2
    }

    // Test scenario: No messengers X and Y in config but data exists from them in pending_data
    {
        let mut deps_scenario2 = cosmwasm_std::testing::MockStorage::new();
        let config_limited = Config {
            messengers: vec![Addr::unchecked("oracle1"), Addr::unchecked("oracle2")],
            threshold: 2,
            data_delta_ppm: 10000,
            round_length: 3600,
        };
        let state_scenario2 = setup_test_state(&mut deps_scenario2, &config_limited, 1, start_time);

        // Reset time
        env.block.time = Timestamp::from_seconds(start_time);

        // Add data from oracles that are in config
        let test_data_s2_1 = create_test_data(
            SignedDecimal256::from_ratio(5, 10),
            SignedDecimal256::from_ratio(1000, 1),
            SignedDecimal256::from_ratio(2000, 1),
            SignedDecimal256::from_ratio(500, 1),
        );
        state_scenario2
            .publish_data(
                &mut deps_scenario2,
                &env,
                Addr::unchecked("oracle1"),
                test_data_s2_1,
            )
            .unwrap();

        let test_data_s2_2 = create_test_data(
            SignedDecimal256::from_ratio(505, 1000),
            SignedDecimal256::from_ratio(1005, 1),
            SignedDecimal256::from_ratio(2010, 1),
            SignedDecimal256::from_ratio(505, 1),
        );
        state_scenario2
            .publish_data(
                &mut deps_scenario2,
                &env,
                Addr::unchecked("oracle2"),
                test_data_s2_2,
            )
            .unwrap();

        // Manually add data from external oracles (not in config)
        let external_data_1 = create_test_data(
            SignedDecimal256::from_ratio(8, 10),
            SignedDecimal256::from_ratio(8000, 1),
            SignedDecimal256::from_ratio(8000, 1),
            SignedDecimal256::from_ratio(800, 1),
        );
        let external_data_2 = create_test_data(
            SignedDecimal256::from_ratio(9, 10),
            SignedDecimal256::from_ratio(9000, 1),
            SignedDecimal256::from_ratio(9000, 1),
            SignedDecimal256::from_ratio(900, 1),
        );

        // Save data from external oracles (not in config)
        state_scenario2
            .pending_data
            .save(
                &mut deps_scenario2,
                Addr::unchecked("external_oracle_x"),
                &crate::consensus::OracleData {
                    round: 1,
                    timestamp: start_time,
                    data: external_data_1,
                },
            )
            .unwrap();
        state_scenario2
            .pending_data
            .save(
                &mut deps_scenario2,
                Addr::unchecked("external_oracle_y"),
                &crate::consensus::OracleData {
                    round: 1,
                    timestamp: start_time,
                    data: external_data_2,
                },
            )
            .unwrap();

        // Verify consensus is reached based only on config messengers, ignoring external data
        let published_data = state_scenario2
            .last_published_data
            .may_load(&deps_scenario2)
            .unwrap();
        assert!(
            published_data.is_some(),
            "Consensus should be reached based on config messengers only"
        );
        let consensus = published_data.unwrap();
        assert_eq!(consensus.round, 1);
        // Consensus should be based only on oracle1 and oracle2 data, not external oracles
        assert_eq!(
            consensus.data.value1,
            SignedDecimal256::from_ratio(5025, 10000)
        ); // median of oracle1 and oracle2, not external oracles

        // Verify external data doesn't affect the result
        assert_ne!(consensus.data.value1, SignedDecimal256::from_ratio(8, 10)); // Not external_oracle_x value
        assert_ne!(consensus.data.value1, SignedDecimal256::from_ratio(9, 10)); // Not external_oracle_y value
    }
}

#[test]
fn test_state_init() {
    // Set up test environment
    let mut deps = cosmwasm_std::testing::MockStorage::new();
    let env = mock_env();

    // Create a new state
    let state = State::<MockData>::default();

    // Initialize the state
    let config = create_test_config();
    let result = state.initialize(&mut deps, &env, config.clone());
    assert!(result.is_ok(), "State initialization should succeed");

    // Verify config was saved
    let saved_config = state.config.load(&deps).unwrap();
    assert_eq!(
        saved_config, config,
        "Saved config should match the provided config"
    );

    // Verify pending round was initialized
    let pending_round = state.pending_round.load(&deps).unwrap();
    assert_eq!(pending_round.round, 0, "Initial round should be 0");
    assert_eq!(
        pending_round.start,
        env.block.time.seconds(),
        "Round start time should match env time"
    );
}

#[test]
fn test_get_all_pending_data_scenarios() {
    // Set up test environment
    let mut deps = cosmwasm_std::testing::MockStorage::new();
    let mut env = mock_env();
    let start_time = 1000;
    env.block.time = Timestamp::from_seconds(start_time);

    // Test scenario 1: Messengers in config but no data from them in pending_data
    {
        let config = Config {
            messengers: vec![
                Addr::unchecked("oracle1"),
                Addr::unchecked("oracle2"),
                Addr::unchecked("oracle3"),
            ],
            threshold: 2,
            data_delta_ppm: 10000,
            round_length: 3600,
        };
        let state = setup_test_state(&mut deps, &config, 1, start_time);

        // Advance time so the round is passed and get_last_published_data calls get_all_pending_data
        env.block.time = Timestamp::from_seconds(start_time + config.round_length + 1);

        // Call get_last_published_data which internally calls get_all_pending_data
        // Should not return any errors even though there's no pending data
        let result = state.get_last_published_data(&env, &deps);
        assert!(
            result.is_ok(),
            "get_last_published_data should not fail when no pending data exists"
        );
        let data = result.unwrap();
        assert!(
            data.is_none(),
            "Should return None when no pending data and no previous published data"
        );
    }

    // Test scenario 2: No messengers in config but data exists from them in pending_data
    {
        let mut deps2 = cosmwasm_std::testing::MockStorage::new();

        // Config with empty messengers list
        let config_empty = Config {
            messengers: vec![], // No messengers in config
            threshold: 2,
            data_delta_ppm: 10000,
            round_length: 3600,
        };
        let state = setup_test_state(&mut deps2, &config_empty, 1, start_time);

        // Manually insert pending data for oracles not in config
        let test_data = create_test_data(
            SignedDecimal256::from_ratio(5, 10),
            SignedDecimal256::from_ratio(1000, 1),
            SignedDecimal256::from_ratio(2000, 1),
            SignedDecimal256::from_ratio(500, 1),
        );
        let oracle_data = crate::consensus::OracleData {
            round: 1,
            timestamp: start_time,
            data: test_data,
        };

        // Save data for oracles not in config
        state
            .pending_data
            .save(
                &mut deps2,
                Addr::unchecked("external_oracle1"),
                &oracle_data,
            )
            .unwrap();
        state
            .pending_data
            .save(
                &mut deps2,
                Addr::unchecked("external_oracle2"),
                &oracle_data,
            )
            .unwrap();

        // Advance time so the round is passed
        env.block.time = Timestamp::from_seconds(start_time + config_empty.round_length + 1);

        // Call get_last_published_data which internally calls get_all_pending_data
        // Should not return any errors even though pending data exists from oracles not in config
        let result = state.get_last_published_data(&env, &deps2);
        assert!(result.is_ok(), "get_last_published_data should not fail when pending data from non-config oracles exists");
        let data = result.unwrap();
        assert!(
            data.is_none(),
            "Should return None when no data from config messengers"
        );
    }

    // Test scenario 3: Mixed scenario - some messengers in config with data, some without, and some external data
    {
        let mut deps3 = cosmwasm_std::testing::MockStorage::new();

        let config_mixed = Config {
            messengers: vec![
                Addr::unchecked("oracle1"),
                Addr::unchecked("oracle2"), // This one won't have data
                Addr::unchecked("oracle3"),
            ],
            threshold: 2,
            data_delta_ppm: 10000,
            round_length: 3600,
        };
        let state = setup_test_state(&mut deps3, &config_mixed, 1, start_time);

        let test_data1 = create_test_data(
            SignedDecimal256::from_ratio(5, 10),
            SignedDecimal256::from_ratio(1000, 1),
            SignedDecimal256::from_ratio(2000, 1),
            SignedDecimal256::from_ratio(500, 1),
        );
        let test_data3 = create_test_data(
            SignedDecimal256::from_ratio(505, 1000),
            SignedDecimal256::from_ratio(1005, 1),
            SignedDecimal256::from_ratio(2010, 1),
            SignedDecimal256::from_ratio(505, 1),
        );
        let external_data = create_test_data(
            SignedDecimal256::from_ratio(6, 10),
            SignedDecimal256::from_ratio(1200, 1),
            SignedDecimal256::from_ratio(2500, 1),
            SignedDecimal256::from_ratio(600, 1),
        );

        // Save data for oracle1 and oracle3 (from config)
        state
            .pending_data
            .save(
                &mut deps3,
                Addr::unchecked("oracle1"),
                &crate::consensus::OracleData {
                    round: 1,
                    timestamp: start_time,
                    data: test_data1,
                },
            )
            .unwrap();
        state
            .pending_data
            .save(
                &mut deps3,
                Addr::unchecked("oracle3"),
                &crate::consensus::OracleData {
                    round: 1,
                    timestamp: start_time,
                    data: test_data3,
                },
            )
            .unwrap();

        // Save data for external oracle (not in config)
        state
            .pending_data
            .save(
                &mut deps3,
                Addr::unchecked("external_oracle"),
                &crate::consensus::OracleData {
                    round: 1,
                    timestamp: start_time,
                    data: external_data,
                },
            )
            .unwrap();

        // oracle2 has no data

        // Advance time so the round is passed
        env.block.time = Timestamp::from_seconds(start_time + config_mixed.round_length + 1);

        // Call get_last_published_data which internally calls get_all_pending_data
        let result = state.get_last_published_data(&env, &deps3);
        assert!(
            result.is_ok(),
            "get_last_published_data should not fail in mixed scenario"
        );
        let data = result.unwrap();
        assert!(
            data.is_some(),
            "Should return consensus data when threshold is met by config messengers"
        );

        // Verify that only data from config messengers was used (oracle1 and oracle3)
        let consensus_data = data.unwrap();
        assert_eq!(consensus_data.round, 1);
        // The consensus should be the median of oracle1 and oracle3 data (external_oracle data ignored)
        assert_eq!(
            consensus_data.data.value1,
            SignedDecimal256::from_ratio(5025, 10000)
        ); // median of 0.5 and 0.505
    }
}

#[test]
fn test_get_all_pending_data_through_publish_data() {
    // Test get_all_pending_data indirectly through publish_data method
    let mut deps = cosmwasm_std::testing::MockStorage::new();
    let mut env = mock_env();
    let start_time = 1000;
    env.block.time = Timestamp::from_seconds(start_time);

    // Test scenario 1: Config messengers without pending data
    {
        let config = Config {
            messengers: vec![Addr::unchecked("oracle1"), Addr::unchecked("oracle2")],
            threshold: 2,
            data_delta_ppm: 10000,
            round_length: 3600,
        };
        let state = setup_test_state(&mut deps, &config, 1, start_time);

        // Advance time to trigger round processing
        env.block.time = Timestamp::from_seconds(start_time + config.round_length + 1);

        let test_data = create_test_data(
            SignedDecimal256::from_ratio(5, 10),
            SignedDecimal256::from_ratio(1000, 1),
            SignedDecimal256::from_ratio(2000, 1),
            SignedDecimal256::from_ratio(500, 1),
        );

        // publish_data calls get_all_pending_data when round is passed
        let result = state.publish_data(&mut deps, &env, Addr::unchecked("oracle1"), test_data);
        assert!(
            result.is_ok(),
            "publish_data should not fail when no prior pending data exists"
        );
    }

    // Test scenario 2: External oracle data exists but not in config
    {
        let mut deps2 = cosmwasm_std::testing::MockStorage::new();

        let config_limited = Config {
            messengers: vec![Addr::unchecked("oracle1")], // Only one messenger in config
            threshold: 1,
            data_delta_ppm: 10000,
            round_length: 3600,
        };
        let state = setup_test_state(&mut deps2, &config_limited, 1, start_time);

        // Add data from external oracle not in config
        let external_data = create_test_data(
            SignedDecimal256::from_ratio(9, 10),
            SignedDecimal256::from_ratio(9000, 1),
            SignedDecimal256::from_ratio(9000, 1),
            SignedDecimal256::from_ratio(900, 1),
        );
        state
            .pending_data
            .save(
                &mut deps2,
                Addr::unchecked("external_oracle"),
                &crate::consensus::OracleData {
                    round: 1,
                    timestamp: start_time,
                    data: external_data,
                },
            )
            .unwrap();

        // Advance time to trigger round processing
        env.block.time = Timestamp::from_seconds(start_time + config_limited.round_length + 1);

        let oracle1_data = create_test_data(
            SignedDecimal256::from_ratio(5, 10),
            SignedDecimal256::from_ratio(1000, 1),
            SignedDecimal256::from_ratio(2000, 1),
            SignedDecimal256::from_ratio(500, 1),
        );

        // publish_data should only consider data from config messengers
        let result = state.publish_data(
            &mut deps2,
            &env,
            Addr::unchecked("oracle1"),
            oracle1_data.clone(),
        );
        assert!(
            result.is_ok(),
            "publish_data should not fail with external oracle data present"
        );

        let (publish_result, _) = result.unwrap();
        match publish_result {
            crate::consensus::PublishResult::ConsensusReached(data) => {
                // Should only use oracle1 data, ignoring external_oracle data
                assert_eq!(data.data.value1, oracle1_data.value1);
                assert_eq!(data.data.value2, oracle1_data.value2);
                assert_eq!(data.data.value3, oracle1_data.value3);
                assert_eq!(data.data.value4, oracle1_data.value4);
            }
            _ => panic!("Expected consensus to be reached with threshold 1"),
        }
    }
}
