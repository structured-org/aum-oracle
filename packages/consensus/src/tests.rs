use crate::consensus::{
    all_items_equal, consensus_on_items_dec256, Config, ConsensusData, Round, State,
};
use crate::error::ConsensusError;
use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{Addr, SignedDecimal256, Timestamp};
use serde::{Deserialize, Serialize};

// Mock data structure for testing data publishing and consensus calculations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct MockData {
    pub value1: SignedDecimal256,
    pub value2: SignedDecimal256,
    pub value3: SignedDecimal256,
    pub value4: SignedDecimal256,
}

struct MockConfig {}

impl ConsensusData<MockConfig> for MockData {
    fn prepublish_cleanup(&mut self, _: MockConfig) -> Result<(), ConsensusError> {
        Ok(())
    }
    // Consensus function for MockData
    fn try_consensus(data: &[MockData], threshold: usize, delta_ppm: u64) -> Option<MockData> {
        if data.len() < threshold {
            return None;
        }

        // Try to reach consensus on each field
        let value1 = consensus_on_items_dec256(
            &data.iter().map(|d| d.value1).collect::<Vec<_>>(),
            threshold,
            delta_ppm,
        );
        let value2 = consensus_on_items_dec256(
            &data.iter().map(|d| d.value2).collect::<Vec<_>>(),
            threshold,
            delta_ppm,
        );
        let value3 = consensus_on_items_dec256(
            &data.iter().map(|d| d.value3).collect::<Vec<_>>(),
            threshold,
            delta_ppm,
        );
        let value4 = consensus_on_items_dec256(
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
            Addr::unchecked("messenger1"),
            Addr::unchecked("messenger2"),
            Addr::unchecked("messenger3"),
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

fn create_mock_config() -> MockConfig {
    MockConfig {}
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

    // Test 5: Consensus with more than threshold messengers
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
            consensus_on_items_dec256(&tc.items, tc.threshold, tc.delta),
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
) -> State<MockData, MockConfig> {
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

    // Test 1: Messenger publishes data
    let messenger1 = Addr::unchecked("messenger1");
    let result = state.publish_data(
        &mut deps,
        &env,
        messenger1.clone(),
        test_data.clone(),
        create_mock_config(),
    );
    assert!(result.is_ok(), "Messenger should be able to publish data");

    // Test 2: Messenger tries to publish data again for the same round (should fail)
    let result = state.publish_data(
        &mut deps,
        &env,
        messenger1.clone(),
        test_data.clone(),
        create_mock_config(),
    );
    assert!(
        result.is_err(),
        "Messenger should not be able to publish data twice for the same round"
    );
    assert_eq!(result.err().unwrap(), ConsensusError::DoubleSubmission {});

    // Test 3: Another messenger publishes data
    let messenger2 = Addr::unchecked("messenger2");
    let test_data2 = create_test_data(
        SignedDecimal256::from_ratio(505, 1000),
        SignedDecimal256::from_ratio(1005, 1),
        SignedDecimal256::from_ratio(2010, 1),
        SignedDecimal256::from_ratio(505, 1),
    );
    let result = state.publish_data(
        &mut deps,
        &env,
        messenger2.clone(),
        test_data2,
        create_mock_config(),
    );
    assert!(
        result.is_ok(),
        "Second messenger should be able to publish data"
    );

    // Test 4: Third messenger publishes data (all messengers have now published)
    let messenger3 = Addr::unchecked("messenger3");
    let test_data3 = create_test_data(
        SignedDecimal256::from_ratio(503, 1000),
        SignedDecimal256::from_ratio(1003, 1),
        SignedDecimal256::from_ratio(2005, 1),
        SignedDecimal256::from_ratio(503, 1),
    );
    let result = state.publish_data(
        &mut deps,
        &env,
        messenger3.clone(),
        test_data3,
        create_mock_config(),
    );
    assert!(
        result.is_ok(),
        "Third messenger should be able to publish data"
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

    // Messenger publishes data for the new round
    let messenger1 = Addr::unchecked("messenger1");
    let test_data = create_test_data(
        SignedDecimal256::from_ratio(5, 10),
        SignedDecimal256::from_ratio(1000, 1),
        SignedDecimal256::from_ratio(2000, 1),
        SignedDecimal256::from_ratio(500, 1),
    );
    let result = state.publish_data(
        &mut deps,
        &env,
        messenger1.clone(),
        test_data,
        create_mock_config(),
    );
    assert!(
        result.is_ok(),
        "Messenger should be able to publish data for the new round"
    );

    // Verify round has advanced
    let pending_round = state.pending_round.load(&deps).unwrap();
    assert_eq!(pending_round.round, 2, "Round should have advanced");
    assert_eq!(
        pending_round.start,
        env.block.time.seconds(),
        "Round start time should be updated"
    );

    // Test 2: Advance time by multiple rounds
    env.block.time = Timestamp::from_seconds(start_time + 3 * config.round_length + 1);

    // Messenger publishes data for the new round
    let test_data_multi = create_test_data(
        SignedDecimal256::from_ratio(51, 100),
        SignedDecimal256::from_ratio(1010, 1),
        SignedDecimal256::from_ratio(2020, 1),
        SignedDecimal256::from_ratio(510, 1),
    );
    let result = state.publish_data(
        &mut deps,
        &env,
        messenger1.clone(),
        test_data_multi,
        create_mock_config(),
    );
    assert!(
        result.is_ok(),
        "Messenger should be able to publish data after multiple round advances"
    );

    // Verify round has advanced by one step, but the start time of the round has advanced by multiple steps
    let pending_round = state.pending_round.load(&deps).unwrap();
    assert_eq!(
        pending_round.round, 3,
        "Round should have advanced by multiple steps"
    );
    assert_eq!(
        pending_round.start,
        env.block.time.seconds(),
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

    // Test 2: Publish data from all messengers
    let messenger1 = Addr::unchecked("messenger1");
    let test_data1 = create_test_data(
        SignedDecimal256::from_ratio(5, 10),
        SignedDecimal256::from_ratio(1000, 1),
        SignedDecimal256::from_ratio(2000, 1),
        SignedDecimal256::from_ratio(500, 1),
    );
    state
        .publish_data(
            &mut deps,
            &env,
            messenger1.clone(),
            test_data1,
            create_mock_config(),
        )
        .unwrap();

    let messenger2 = Addr::unchecked("messenger2");
    let test_data2 = create_test_data(
        SignedDecimal256::from_ratio(505, 1000),
        SignedDecimal256::from_ratio(1005, 1),
        SignedDecimal256::from_ratio(2010, 1),
        SignedDecimal256::from_ratio(505, 1),
    );
    state
        .publish_data(
            &mut deps,
            &env,
            messenger2.clone(),
            test_data2,
            create_mock_config(),
        )
        .unwrap();

    let messenger3 = Addr::unchecked("messenger3");
    let test_data3 = create_test_data(
        SignedDecimal256::from_ratio(503, 1000),
        SignedDecimal256::from_ratio(1003, 1),
        SignedDecimal256::from_ratio(2005, 1),
        SignedDecimal256::from_ratio(503, 1),
    );
    state
        .publish_data(
            &mut deps,
            &env,
            messenger3.clone(),
            test_data3,
            create_mock_config(),
        )
        .unwrap();

    // Verify data is published
    let result = state.get_last_published_data(&env, &deps).unwrap();
    assert!(
        result.is_some(),
        "Data must be published after all messengers submit"
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

    // Only one messenger publishes data (below the threshold)
    let test_data4 = create_test_data(
        SignedDecimal256::from_ratio(52, 100),
        SignedDecimal256::from_ratio(1020, 1),
        SignedDecimal256::from_ratio(2040, 1),
        SignedDecimal256::from_ratio(520, 1),
    );
    state
        .publish_data(
            &mut deps,
            &env,
            messenger1.clone(),
            test_data4,
            create_mock_config(),
        )
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
        .publish_data(
            &mut deps,
            &env,
            messenger2.clone(),
            test_data5,
            create_mock_config(),
        )
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
}

#[test]
fn test_state_init() {
    // Set up test environment
    let mut deps = cosmwasm_std::testing::MockStorage::new();
    let env = mock_env();

    // Create a new state
    let state = State::<MockData, MockConfig>::default();

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
fn test_exact_consensus_on_items_various_cases() {
    // empty input
    {
        let items: Vec<u64> = vec![];
        assert_eq!(all_items_equal(&items), None);
    }

    // single item
    {
        let items = vec![42];
        assert_eq!(all_items_equal(&items), Some(42));
    }

    // all equal
    {
        let items = vec![7, 7, 7, 7];
        assert_eq!(all_items_equal(&items), Some(7));
    }

    // one different
    {
        let items = vec![1, 1, 2, 1];
        assert_eq!(all_items_equal(&items), None);
    }

    // all equal strings
    {
        let items = vec!["a", "a", "a"];
        assert_eq!(all_items_equal(&items), Some("a"));
    }

    // different strings
    {
        let items = vec!["a", "b", "a"];
        assert_eq!(all_items_equal(&items), None);
    }

    // custom struct
    {
        #[derive(Clone, Eq, PartialEq, Debug)]
        struct Foo(u8);
        let items = vec![Foo(1), Foo(1), Foo(1)];
        assert_eq!(all_items_equal(&items), Some(Foo(1)));
    }
}

#[test]
fn test_finalize_round_on_update_messengers_count() {
    // This test verifies that when a pending config updates the messengers count during round advancement,
    // the updated messengers count is used to check if the round can be finalized, not the outdated one.

    // 1) 1 → 2 messengers: First publication should NOT trigger round finalization
    {
        let mut deps = cosmwasm_std::testing::MockStorage::new();
        let mut env = mock_env();
        let start_time = 1000;
        env.block.time = Timestamp::from_seconds(start_time);

        let initial_config = Config {
            messengers: vec![Addr::unchecked("messenger1")], // Set up initial state with 1 messenger
            threshold: 1,
            data_delta_ppm: 10000,
            round_length: 3600,
        };

        let round = Round {
            round: 1,
            start: start_time,
        };
        let state = setup_test_state(&mut deps, &initial_config, round.round, round.start);

        // Save the new config with 2 messengers as pending (will be applied on round switch)
        state
            .save_config(
                &mut deps,
                Config {
                    messengers: vec![Addr::unchecked("messenger1"), Addr::unchecked("messenger2")],
                    threshold: 1, // Keep threshold low to focus on messengers.len() check
                    data_delta_ppm: 10000,
                    round_length: 3600,
                },
            )
            .unwrap();

        // Advance time to trigger round advancement
        env.block.time = Timestamp::from_seconds(start_time + initial_config.round_length + 1);

        // Create test data
        let test_data = create_test_data(
            SignedDecimal256::from_ratio(5, 10),
            SignedDecimal256::from_ratio(1000, 1),
            SignedDecimal256::from_ratio(2000, 1),
            SignedDecimal256::from_ratio(500, 1),
        );

        // Publish data from only one messenger
        // With the OLD config (1 messenger), this would trigger immediate consensus finalization
        // With the NEW config (2 messengers), this should NOT trigger immediate consensus
        let messenger1 = Addr::unchecked("messenger1");
        let result = state
            .publish_data(
                &mut deps,
                &env,
                messenger1.clone(),
                test_data.clone(),
                create_mock_config(),
            )
            .unwrap();

        // Verify that consensus was NOT reached immediately because we now have 2 messengers total
        // and only 1 has submitted data (the check should be pending_data.len() == 2, not 1)
        match result.0 {
            crate::consensus::PublishResult::ConsensusNotReached => {
                // This is expected - with 2 messengers total, we need both to submit before immediate consensus
            }
            crate::consensus::PublishResult::ConsensusReached(_) => {
                panic!("Consensus should NOT be reached immediately with only 1 out of 2 messengers submitting data");
            }
        }

        // Verify that the config has been updated in storage
        let current_config = state.config.load(&deps).unwrap();
        assert_eq!(
            current_config.messengers.len(),
            2,
            "Config should have 2 messengers now"
        );

        // Verify that pending config was cleared
        let pending_config = state.pending_config.may_load(&deps).unwrap();
        assert!(
            pending_config.is_none(),
            "Pending config should be cleared after application"
        );

        // Now publish data from the second messenger - this should trigger immediate consensus
        let messenger2 = Addr::unchecked("messenger2");
        let test_data2 = create_test_data(
            SignedDecimal256::from_ratio(505, 1000),
            SignedDecimal256::from_ratio(1005, 1),
            SignedDecimal256::from_ratio(2010, 1),
            SignedDecimal256::from_ratio(505, 1),
        );

        let result2 = state
            .publish_data(
                &mut deps,
                &env,
                messenger2.clone(),
                test_data2,
                create_mock_config(),
            )
            .unwrap();

        // Now consensus should be reached because all 2 messengers have submitted data
        match result2.0 {
            crate::consensus::PublishResult::ConsensusReached(outcome) => {
                assert_eq!(outcome.round, 2, "Consensus should be reached for round 2");
            }
            crate::consensus::PublishResult::ConsensusNotReached => {
                panic!("Consensus SHOULD be reached with both messengers submitting data");
            }
        }
    }

    // 2) 2 → 1 messengers: First publication SHOULD trigger round finalization
    {
        let mut deps = cosmwasm_std::testing::MockStorage::new();
        let mut env = mock_env();
        let start_time = 2000; // Different start time to avoid conflicts
        env.block.time = Timestamp::from_seconds(start_time);

        // Set up initial state with 2 messengers
        let initial_config = Config {
            messengers: vec![Addr::unchecked("messenger1"), Addr::unchecked("messenger2")],
            threshold: 1,
            data_delta_ppm: 10000,
            round_length: 3600,
        };

        let round = Round {
            round: 1,
            start: start_time,
        };
        let state = setup_test_state(&mut deps, &initial_config, round.round, round.start);

        // Save the new config with 1 messenger as pending (will be applied on round switch)
        state
            .save_config(
                &mut deps,
                Config {
                    messengers: vec![Addr::unchecked("messenger1")],
                    threshold: 1,
                    data_delta_ppm: 10000,
                    round_length: 3600,
                },
            )
            .unwrap();

        // Advance time to trigger round advancement
        env.block.time = Timestamp::from_seconds(start_time + initial_config.round_length + 1);

        // Create test data
        let test_data = create_test_data(
            SignedDecimal256::from_ratio(6, 10),
            SignedDecimal256::from_ratio(1100, 1),
            SignedDecimal256::from_ratio(2100, 1),
            SignedDecimal256::from_ratio(600, 1),
        );

        // Publish data from only one messenger
        // With the OLD config (2 messengers), this would NOT trigger immediate consensus finalization
        // With the NEW config (1 messenger), this SHOULD trigger immediate consensus
        let messenger1 = Addr::unchecked("messenger1");
        let result = state
            .publish_data(
                &mut deps,
                &env,
                messenger1.clone(),
                test_data.clone(),
                create_mock_config(),
            )
            .unwrap();

        // Verify that consensus WAS reached immediately because we now have only 1 messenger total
        // and 1 has submitted data (the check should be pending_data.len() == 1, not 2)
        match result.0 {
            crate::consensus::PublishResult::ConsensusReached(outcome) => {
                assert_eq!(
                    outcome.round, 2,
                    "Consensus should be reached immediately for round 2"
                );
            }
            crate::consensus::PublishResult::ConsensusNotReached => {
                panic!("Consensus SHOULD be reached immediately with 1 out of 1 messenger submitting data");
            }
        }

        // Verify that the config has been updated in storage
        let current_config = state.config.load(&deps).unwrap();
        assert_eq!(
            current_config.messengers.len(),
            1,
            "Config should have 1 messenger now"
        );

        // Verify that pending config was cleared
        let pending_config = state.pending_config.may_load(&deps).unwrap();
        assert!(
            pending_config.is_none(),
            "Pending config should be cleared after application"
        );
    }
}
