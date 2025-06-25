use crate::consensus::{consensus_on_items, Config, ConsensusData, OracleData, Round, State};
use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{Addr, SignedDecimal, Timestamp};
use serde::{Deserialize, Serialize};

// Mock data structure for testing OracleData<T>
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct MockData {
    pub value1: SignedDecimal,
    pub value2: SignedDecimal,
    pub value3: SignedDecimal,
    pub value4: SignedDecimal,
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

// Helper function to create a test MockData object
fn create_test_data(
    round: u64,
    timestamp: u64,
    value1: SignedDecimal,
    value2: SignedDecimal,
    value3: SignedDecimal,
    value4: SignedDecimal,
) -> OracleData<MockData> {
    OracleData {
        round,
        timestamp,
        data: MockData {
            value1,
            value2,
            value3,
            value4,
        },
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
            MockData::try_consensus(&data, config.threshold as usize, config.data_delta_ppm);
        assert!(consensus.is_some());

        let result = consensus.unwrap();
        assert_eq!(result.value1, SignedDecimal::from_ratio(5025, 10000)); // median of 0.5 and 0.505
        assert_eq!(result.value2, SignedDecimal::from_ratio(10025, 10)); // median of 1000 and 1005
        assert_eq!(result.value3, SignedDecimal::from_ratio(2005, 1)); // median of 2000 and 2010
        assert_eq!(result.value4, SignedDecimal::from_ratio(5025, 10)); // median of 500 and 505
    }

    // Test 4: Data outside acceptable delta should not reach consensus
    {
        let mut config = create_test_config();
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

        let consensus = MockData::try_consensus(
            &data_multiple,
            config.threshold as usize,
            config.data_delta_ppm,
        );
        assert!(consensus.is_some());

        let result = consensus.unwrap();
        assert_eq!(result.value1, SignedDecimal::from_ratio(503, 1000));
        assert_eq!(result.value2, SignedDecimal::from_ratio(1003, 1));
        assert_eq!(result.value3, SignedDecimal::from_ratio(2005, 1));
        assert_eq!(result.value4, SignedDecimal::from_ratio(503, 1));
    }

    // Test 6: Consensus with some outliers
    {
        let config = create_test_config();
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

        let consensus = MockData::try_consensus(
            &data_with_outliers,
            config.threshold as usize,
            config.data_delta_ppm,
        );
        assert!(consensus.is_some());

        // The outlier should be excluded from the consensus
        let result = consensus.unwrap();
        assert_eq!(result.value1, SignedDecimal::from_ratio(5025, 10000)); // median of 0.5 and 0.505 (outlier excluded)
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
    // Test with empty array
    let empty: Vec<SignedDecimal> = vec![];
    assert!(
        consensus_on_items(&empty, 1, 10000).is_none(),
        "Empty array should return None"
    );

    // Test with array smaller than threshold
    let small = vec![SignedDecimal::from_ratio(5, 10)];
    assert!(
        consensus_on_items(&small, 2, 10000).is_none(),
        "Array smaller than threshold should return None"
    );

    // Test with values within delta
    let within_delta = vec![
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(505, 1000),
        SignedDecimal::from_ratio(51, 100),
    ];
    let result = consensus_on_items(&within_delta, 2, 10000); // 1% delta
    assert!(
        result.is_some(),
        "Values within delta should reach consensus"
    );
    assert_eq!(
        result.unwrap(),
        SignedDecimal::from_ratio(5025, 10000),
        "Median should be correct"
    );

    // Test with values outside delta
    let outside_delta = vec![
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(6, 10), // 20% difference
        SignedDecimal::from_ratio(45, 100),
    ];
    let result = consensus_on_items(&outside_delta, 2, 1000); // 0.1% delta
    assert!(
        result.is_none(),
        "Values outside delta should not reach consensus"
    );

    // Test with values that are all within delta
    let all_within_delta = vec![
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(505, 1000),
        SignedDecimal::from_ratio(51, 100),
    ];
    let result = consensus_on_items(&all_within_delta, 2, 10000); // 1% delta
    assert!(
        result.is_some(),
        "Values within delta should reach consensus"
    );
    assert_eq!(
        result.unwrap(),
        SignedDecimal::from_ratio(5025, 10000),
        "Median should be correct"
    );

    // Test with values that are all within delta but with zero as corner value
    let all_within_delta = vec![
        SignedDecimal::from_ratio(-5, 1),
        SignedDecimal::from_ratio(-4, 1),
        SignedDecimal::from_ratio(0, 1),
    ];
    let result = consensus_on_items(&all_within_delta, 3, 1000000); // 100% delta
    assert!(
        result.is_some(),
        "Values within delta should reach consensus"
    );
    assert_eq!(
        result.unwrap(),
        SignedDecimal::from_ratio(-4, 1),
        "Median should be correct"
    );
}

// Helper function to setup storage with config and current round
fn setup_test_state(
    deps: &mut cosmwasm_std::testing::MockStorage,
    config: &Config,
    round: u64,
    start_time: u64,
) -> State<MockData> {
    let state = State::new();
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
        1,
        start_time,
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(1000, 1),
        SignedDecimal::from_ratio(2000, 1),
        SignedDecimal::from_ratio(500, 1),
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
    match result {
        Err(cosmwasm_std::StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Oracle has already submitted data for this round");
        }
        _ => panic!("Unexpected error"),
    }

    // Test 3: Another oracle publishes data
    let oracle2 = Addr::unchecked("oracle2");
    let test_data2 = create_test_data(
        1,
        start_time + 1,
        SignedDecimal::from_ratio(505, 1000),
        SignedDecimal::from_ratio(1005, 1),
        SignedDecimal::from_ratio(2010, 1),
        SignedDecimal::from_ratio(505, 1),
    );
    let result = state.publish_data(&mut deps, &env, oracle2.clone(), test_data2);
    assert!(
        result.is_ok(),
        "Second oracle should be able to publish data"
    );

    // Test 4: Third oracle publishes data (all oracles have now published)
    let oracle3 = Addr::unchecked("oracle3");
    let test_data3 = create_test_data(
        1,
        start_time + 2,
        SignedDecimal::from_ratio(503, 1000),
        SignedDecimal::from_ratio(1003, 1),
        SignedDecimal::from_ratio(2005, 1),
        SignedDecimal::from_ratio(503, 1),
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
        SignedDecimal::from_ratio(503, 1000),
        "Consensus value1 should be correct"
    );
    assert_eq!(
        data.data.value2,
        SignedDecimal::from_ratio(1003, 1),
        "Consensus value2 should be correct"
    );
    assert_eq!(
        data.data.value3,
        SignedDecimal::from_ratio(2005, 1),
        "Consensus value3 should be correct"
    );
    assert_eq!(
        data.data.value4,
        SignedDecimal::from_ratio(503, 1),
        "Consensus value4 should be correct"
    );

    // Test 6: Oracle tries to publish data for an already finalized round (should fail)
    let test_data_finalized = create_test_data(
        1, // Same round that's already finalized
        start_time + 3,
        SignedDecimal::from_ratio(51, 100),
        SignedDecimal::from_ratio(1010, 1),
        SignedDecimal::from_ratio(2020, 1),
        SignedDecimal::from_ratio(510, 1),
    );
    let result = state.publish_data(&mut deps, &env, oracle1.clone(), test_data_finalized);
    assert!(
        result.is_err(),
        "Oracle should not be able to publish data for a finalized round"
    );
    match result {
        Err(cosmwasm_std::StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Invalid round");
        }
        _ => panic!("Unexpected error"),
    }
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
        2, // New round
        start_time + config.round_length + 1,
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(1000, 1),
        SignedDecimal::from_ratio(2000, 1),
        SignedDecimal::from_ratio(500, 1),
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
        4, // New round after multiple advances
        start_time + 3 * config.round_length + 1,
        SignedDecimal::from_ratio(51, 100),
        SignedDecimal::from_ratio(1010, 1),
        SignedDecimal::from_ratio(2020, 1),
        SignedDecimal::from_ratio(510, 1),
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
        1,
        start_time,
        SignedDecimal::from_ratio(5, 10),
        SignedDecimal::from_ratio(1000, 1),
        SignedDecimal::from_ratio(2000, 1),
        SignedDecimal::from_ratio(500, 1),
    );
    state
        .publish_data(&mut deps, &env, oracle1.clone(), test_data1)
        .unwrap();

    let oracle2 = Addr::unchecked("oracle2");
    let test_data2 = create_test_data(
        1,
        start_time + 1,
        SignedDecimal::from_ratio(505, 1000),
        SignedDecimal::from_ratio(1005, 1),
        SignedDecimal::from_ratio(2010, 1),
        SignedDecimal::from_ratio(505, 1),
    );
    state
        .publish_data(&mut deps, &env, oracle2.clone(), test_data2)
        .unwrap();

    let oracle3 = Addr::unchecked("oracle3");
    let test_data3 = create_test_data(
        1,
        start_time + 2,
        SignedDecimal::from_ratio(503, 1000),
        SignedDecimal::from_ratio(1003, 1),
        SignedDecimal::from_ratio(2005, 1),
        SignedDecimal::from_ratio(503, 1),
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
        SignedDecimal::from_ratio(503, 1000),
        "Consensus value1 should be correct"
    );

    // Test 3: Advance time to next round but don't publish enough data
    env.block.time = Timestamp::from_seconds(start_time + config.round_length + 1);

    // Only one oracle publishes data (below threshold)
    let test_data4 = create_test_data(
        2,
        start_time + config.round_length + 1,
        SignedDecimal::from_ratio(52, 100),
        SignedDecimal::from_ratio(1020, 1),
        SignedDecimal::from_ratio(2040, 1),
        SignedDecimal::from_ratio(520, 1),
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
        2,
        start_time + config.round_length + 2,
        SignedDecimal::from_ratio(525, 1000),
        SignedDecimal::from_ratio(1025, 1),
        SignedDecimal::from_ratio(2050, 1),
        SignedDecimal::from_ratio(525, 1),
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
        SignedDecimal::from_ratio(5225, 10000),
        "Consensus value1 should be correct"
    );
}

#[test]
fn test_state_init() {
    // Set up test environment
    let mut deps = cosmwasm_std::testing::MockStorage::new();
    let env = mock_env();

    // Create a new state
    let state = State::<MockData>::new();

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
