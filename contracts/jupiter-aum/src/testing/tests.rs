#[cfg(test)]
mod tests {
    use crate::contract::{calculate_aum_in_btc, execute, instantiate, query};
    use crate::state::{CONFIG, LAST_PUBLISHED_DATA, PENDING_DATA};
    use crate::testing::mock_querier::mock_dependencies;
    use cosmwasm_std::testing::{message_info, mock_env, MockApi};
    use cosmwasm_std::{attr, from_json, Decimal, Timestamp, Uint128};
    use jupiter_aum_common::error::ContractError;
    use jupiter_aum_common::msg::{ExecuteMsg, GetAUMResponse, InstantiateMsg, QueryMsg};
    use jupiter_aum_common::types::{CustodyAsset, SolanaData};
    use std::str::FromStr;

    // Helper to create a default instantiate message
    fn default_init_msg(api: &MockApi) -> InstantiateMsg {
        InstantiateMsg {
            admin: api.addr_make("admin").to_string(),
            oracles: vec![
                api.addr_make("oracle1").to_string(),
                api.addr_make("oracle2").to_string(),
                api.addr_make("oracle3").to_string(),
            ],
            threshold: 2,
            extract_period: 10,
            valid_period: 1_000,
        }
    }

    /// Tests the following scenario:
    ///     1.  A non-authorized address tries to update config's contract (error)
    ///     2.  An authorized address tries to update config's contract with invalid date
    ///     3.  An authorized address tries to update config's contract
    #[test]
    fn test_update_config() {
        let mut deps = mock_dependencies();

        // Instantiate
        let env = mock_env();
        let admin_info = message_info(&deps.api.addr_make("admin"), &[]);
        let msg = default_init_msg(&deps.api);
        let init_res = instantiate(deps.as_mut(), env.clone(), admin_info.clone(), msg).unwrap();
        assert_eq!(init_res.messages.len(), 0);

        let config = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(config.admin, admin_info.sender);

        let update_msg = ExecuteMsg::UpdateConfig {
            admin: Some(deps.api.addr_make("admin2").to_string()),
            oracles: Some(vec![
                deps.api.addr_make("oracle3").to_string(),
                deps.api.addr_make("oracle4").to_string(),
            ]),
            threshold: Some(2),
            extract_period: Some(100_000),
            valid_period: Some(50_000),
        };

        // Unauthorized update
        let stranger_info = message_info(&deps.api.addr_make("stranger"), &[]);
        let unauthorized_res = execute(
            deps.as_mut(),
            env.clone(),
            stranger_info,
            update_msg.clone(),
        );
        assert_eq!(
            unauthorized_res.err().unwrap(),
            ContractError::Unauthorized {}
        );

        // Authorized update but new config is invalid (threshold > oracles.len())
        let invalid_update_msg = ExecuteMsg::UpdateConfig {
            admin: Some(deps.api.addr_make("admin2").to_string()),
            oracles: Some(vec![deps.api.addr_make("oracle3").to_string()]), // Only 1 oracle
            threshold: Some(2),                                             // Threshold 2
            extract_period: Some(100_000),
            valid_period: Some(50_000),
        };
        let authorized_res = execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            invalid_update_msg.clone(),
        );
        assert_eq!(
            authorized_res.err().unwrap(),
            ContractError::InvalidThreshold {
                threshold: 2,
                oracles: 1
            }
        );

        // Authorized update
        let authorized_res = execute(deps.as_mut(), env.clone(), admin_info.clone(), update_msg);
        assert!(authorized_res.is_ok());

        // Config should have updated values
        let config = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(config.admin, deps.api.addr_make("admin2"));
        assert_eq!(
            config.oracles,
            vec![deps.api.addr_make("oracle3"), deps.api.addr_make("oracle4")]
        );
        assert_eq!(config.threshold, 2);
        assert_eq!(config.extract_period, 100_000);
        assert_eq!(config.valid_period, 50_000);
    }

    #[test]
    fn test_calculate_aum_in_btc() {
        // Test case 1: Standard calculation
        let data1 = SolanaData {
            timestamp: Timestamp::from_seconds(1),
            slot: 1,
            custody_assets: vec![CustodyAsset {
                owned: 100,
                locked: 50,
                guaranteed_usd: 150,
                decimals: 6,
                denom: "USDC".to_string(),
            }],
            aum_usd: Uint128::new(500_000_000_000), // $500,000 AUM USD
            total_jlp_supply: Uint128::new(1_000_000_000), // 1,000 JLP total supply
            strategy_jlp_balance: Uint128::new(10_000_000_000), // 10,000 JLP balance
        };
        let btc_price_in_usd1 = Decimal::from_str("25000.0").unwrap(); // $25,000 per BTC
                                                                       // jlp_virtual_price = 500,000 / 1,000 = 500 USD/JLP
                                                                       // jlp_balance_in_usd = 500 * 10,000 = 5,000,000 USD
                                                                       // aum_in_btc = 5,000,000 / 25,000 = 200 BTC
        let res1 = calculate_aum_in_btc(data1, btc_price_in_usd1);
        assert_eq!(
            res1.unwrap(),
            Uint128::new(200_000_000),
            "Test Case 1 Failed"
        );

        // Test case 2: Different values
        let data2 = SolanaData {
            timestamp: Timestamp::from_seconds(1),
            slot: 1,
            custody_assets: vec![CustodyAsset {
                owned: 200,
                locked: 100,
                guaranteed_usd: 300,
                decimals: 6,
                denom: "USDT".to_string(),
            }],
            aum_usd: Uint128::new(1_000_000_000_000_000), // $1 Billion AUM
            total_jlp_supply: Uint128::new(50_000_000_000), // 50,000 JLP total
            strategy_jlp_balance: Uint128::new(20_000_000_000), // 20,000 JLP balance
        };
        let btc_price_in_usd2 = Decimal::from_str("50000.0").unwrap(); // $50,000 per BTC
                                                                       // jlp_virtual_price = 1,000,000,000 / 50,000 = 20,000 USD/JLP
                                                                       // jlp_balance_in_usd = 20,000 * 20,000 = 400,000,000 USD
                                                                       // aum_in_btc = 400,000,000 / 50,000 = 8,000 BTC
        let res2 = calculate_aum_in_btc(data2, btc_price_in_usd2);
        assert_eq!(
            res2.unwrap(),
            Uint128::new(8_000_000_000),
            "Test Case 2 Failed"
        );

        // Test case 3: Division by zero for total_jlp_supply
        let data3 = SolanaData {
            timestamp: Timestamp::from_seconds(1),
            slot: 1,
            custody_assets: vec![],
            aum_usd: Uint128::new(100),
            total_jlp_supply: Uint128::new(0), // Zero supply
            strategy_jlp_balance: Uint128::new(10),
        };
        let btc_price_in_usd3 = Decimal::from_str("1.0").unwrap();
        let err3 = calculate_aum_in_btc(data3, btc_price_in_usd3).unwrap_err();
        assert!(
            matches!(&err3, ContractError::DecimalError { error } if error.contains("Denominator must not be zero")),
            "Test Case 3 Failed: {:?}",
            err3
        );

        // Test case 4: Division by zero for btc_price_in_usd
        let data4 = SolanaData {
            timestamp: Timestamp::from_seconds(1),
            slot: 1,
            custody_assets: vec![],
            aum_usd: Uint128::new(100),
            total_jlp_supply: Uint128::new(10),
            strategy_jlp_balance: Uint128::new(5),
        };
        let btc_price_in_usd4 = Decimal::from_str("0.0").unwrap(); // Zero BTC price
        let err4 = calculate_aum_in_btc(data4, btc_price_in_usd4).unwrap_err();
        assert!(
            matches!(&err4, ContractError::DecimalError { error } if error.contains("Denominator must not be zero")),
            "Test Case 4 Failed: {:?}",
            err4
        );
    }

    /// Comprehensive test suite for the `publish_data` execute message.
    #[test]
    fn test_publish_data_errors() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let admin_info = message_info(&deps.api.addr_make("admin"), &[]);
        let oracle1 = deps.api.addr_make("oracle1");
        let oracle2 = deps.api.addr_make("oracle2");
        let oracle3 = deps.api.addr_make("oracle3");
        let non_oracle = deps.api.addr_make("non_oracle");

        let init_msg = default_init_msg(&deps.api);
        instantiate(deps.as_mut(), env.clone(), admin_info.clone(), init_msg).unwrap();

        // --- Error Cases ---

        // Case 1: Not an oracle
        let data_valid_slot = SolanaData {
            timestamp: env.block.time,
            slot: 10,
            custody_assets: custody_asset(),
            aum_usd: Uint128::new(1_000),
            total_jlp_supply: Uint128::new(100),
            strategy_jlp_balance: Uint128::new(50),
        };
        let err = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&non_oracle, &[]),
            publish_msg_from_solana_data(&data_valid_slot),
        )
        .unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // Case 2: Incorrect slot (not multiple of extract_period)
        let data_invalid_slot = SolanaData {
            timestamp: env.block.time,
            slot: 11, // Not a multiple of 10
            custody_assets: custody_asset(),
            aum_usd: Uint128::new(1_000),
            total_jlp_supply: Uint128::new(100),
            strategy_jlp_balance: Uint128::new(50),
        };
        let err = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            publish_msg_from_solana_data(&data_invalid_slot),
        )
        .unwrap_err();
        assert_eq!(err, ContractError::InvalidSolanaSlot { extract_period: 10 });

        // Case 3: Slot too old (publish data for slot 10, then try to publish for slot 10 again)
        // First, publish valid data for slot 10 from oracle1 to set LAST_PUBLISHED_DATA (part of consensus below)
        let data_s10_v1 = SolanaData {
            timestamp: env.block.time,
            slot: 10,
            custody_assets: custody_asset(),
            aum_usd: Uint128::new(1_000),
            total_jlp_supply: Uint128::new(100),
            strategy_jlp_balance: Uint128::new(50),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            publish_msg_from_solana_data(&data_s10_v1),
        )
        .unwrap();
        // Now, publish data from oracle2 for slot 10 to reach consensus
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            publish_msg_from_solana_data(&data_s10_v1),
        )
        .unwrap();
        // Verify LAST_PUBLISHED_DATA is updated
        assert_eq!(
            LAST_PUBLISHED_DATA.load(&deps.storage).unwrap().data.slot,
            10
        );

        let data_s10_too_old = SolanaData {
            timestamp: env.block.time,
            slot: 10, // Same slot as last published
            custody_assets: vec![CustodyAsset {
                owned: 200,
                locked: 0,
                guaranteed_usd: 200,
                decimals: 6,
                denom: "USDC".to_string(),
            }],
            aum_usd: Uint128::new(2000),
            total_jlp_supply: Uint128::new(200),
            strategy_jlp_balance: Uint128::new(100),
        };
        let err = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle3, &[]),
            publish_msg_from_solana_data(&data_s10_too_old),
        )
        .unwrap_err();
        assert_eq!(
            err,
            ContractError::SlotTooOld {
                new_slot: 10,
                last_slot: 10
            }
        );

        // Case 4: Double publishing by the same oracle for the same slot (and hash)
        let data_s20_v1 = SolanaData {
            timestamp: env.block.time,
            slot: 20,
            custody_assets: custody_asset(),
            aum_usd: Uint128::new(1_000),
            total_jlp_supply: Uint128::new(100),
            strategy_jlp_balance: Uint128::new(50),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            publish_msg_from_solana_data(&data_s20_v1),
        )
        .unwrap(); // First publish for slot 20 by oracle1

        let err = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            publish_msg_from_solana_data(&data_s20_v1),
        )
        .unwrap_err(); // Second publish for slot 20 by oracle1
        assert_eq!(err, ContractError::AlreadyPublished {});
    }
    #[test]
    fn test_publish_data_green_path() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();

        let admin_info = message_info(&deps.api.addr_make("admin"), &[]);
        let oracle1 = deps.api.addr_make("oracle1");
        let oracle2 = deps.api.addr_make("oracle2");
        let oracle3 = deps.api.addr_make("oracle3");
        let init_msg = default_init_msg(&deps.api);
        instantiate(deps.as_mut(), env.clone(), admin_info.clone(), init_msg).unwrap();

        // Ensure LAST_PUBLISHED_DATA is clear
        assert!(LAST_PUBLISHED_DATA.load(&deps.storage).is_err());

        // Case 5: First publish (no consensus)
        let data_s30_v1 = SolanaData {
            timestamp: env.block.time,
            slot: 30,
            custody_assets: custody_asset(),
            aum_usd: Uint128::new(1_000),
            total_jlp_supply: Uint128::new(100),
            strategy_jlp_balance: Uint128::new(50),
        };
        let res = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            publish_msg_from_solana_data(&data_s30_v1),
        )
        .unwrap();
        assert!(
            res.attributes
                .iter()
                .any(|a| a.key == "consensus_reached" && a.value == "false"),
            "Expected not 'consensus_reached' = false attribute"
        );
        assert_eq!(LAST_PUBLISHED_DATA.load(&deps.storage).is_err(), true); // No consensus yet
        let pending_key = (data_s30_v1.slot, data_s30_v1.hash().unwrap());
        assert!(PENDING_DATA.has(&deps.storage, pending_key.clone()));
        let pending_entries = PENDING_DATA
            .load(&deps.storage, pending_key.clone())
            .unwrap();
        assert_eq!(pending_entries.len(), 1);

        // Case 6: First consensus reached
        // Oracle2 publishes the same data for slot 30, reaching threshold (2 oracles, threshold 2)
        let res = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            publish_msg_from_solana_data(&data_s30_v1),
        )
        .unwrap();
        assert!(
            res.attributes
                .iter()
                .any(|a| a.key == "consensus_reached" && a.value == "true"),
            "Expected 'consensus_reached' = true attribute"
        );
        assert_eq!(
            res.attributes.last().unwrap(),
            &attr("published_at", env.block.time.to_string())
        );
        let last_published = LAST_PUBLISHED_DATA.load(&deps.storage).unwrap();
        assert_eq!(last_published.data, data_s30_v1);
        assert_eq!(last_published.published_at, env.block.time);
        // Pending data for slot 30 (any hash for slot 30) should be cleared
        assert_eq!(PENDING_DATA.load(&deps.storage, pending_key).is_err(), true);

        // Case 7: Next consensus reached + pending data removed
        env.block.time = env.block.time.plus_seconds(100); // Advance time
        let data_s40_v1 = SolanaData {
            timestamp: env.block.time,
            slot: 40,
            custody_assets: vec![CustodyAsset {
                owned: 200,
                locked: 0,
                guaranteed_usd: 200,
                decimals: 6,
                denom: "USDC".to_string(),
            }],
            aum_usd: Uint128::new(2000),
            total_jlp_supply: Uint128::new(200),
            strategy_jlp_balance: Uint128::new(100),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            publish_msg_from_solana_data(&data_s40_v1),
        )
        .unwrap();
        let res = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            publish_msg_from_solana_data(&data_s40_v1),
        )
        .unwrap();
        assert!(
            res.attributes
                .iter()
                .any(|a| a.key == "consensus_reached" && a.value == "true"),
            "Expected 'consensus_reached' = true attribute"
        );
        let last_published = LAST_PUBLISHED_DATA.load(&deps.storage).unwrap();
        assert_eq!(last_published.data, data_s40_v1);
        assert_eq!(last_published.published_at, env.block.time);
        // Pending data for slot 40 should be cleared
        let pending_key_s40 = (data_s40_v1.slot, data_s40_v1.hash().unwrap());
        assert_eq!(
            PENDING_DATA.load(&deps.storage, pending_key_s40).is_err(),
            true
        );

        // Case 8: Consensus reached, there are already publications for the next slot
        env.block.time = env.block.time.plus_seconds(100); // Advance time
        let data_s50_v1 = SolanaData {
            timestamp: env.block.time,
            slot: 50,
            custody_assets: vec![CustodyAsset {
                owned: 300,
                locked: 0,
                guaranteed_usd: 300,
                decimals: 6,
                denom: "USDC".to_string(),
            }],
            aum_usd: Uint128::new(3000),
            total_jlp_supply: Uint128::new(300),
            strategy_jlp_balance: Uint128::new(150),
        };
        let data_s60_v1 = SolanaData {
            timestamp: env.block.time.plus_seconds(10), // A bit later
            slot: 60,
            custody_assets: vec![CustodyAsset {
                owned: 400,
                locked: 0,
                guaranteed_usd: 400,
                decimals: 6,
                denom: "USDC".to_string(),
            }],
            aum_usd: Uint128::new(4000),
            total_jlp_supply: Uint128::new(400),
            strategy_jlp_balance: Uint128::new(200),
        };
        // Publish for slot 60 first (no consensus yet)
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            publish_msg_from_solana_data(&data_s60_v1),
        )
        .unwrap();
        let pending_key_s60 = (data_s60_v1.slot, data_s60_v1.hash().unwrap());
        assert!(PENDING_DATA.has(&deps.storage, pending_key_s60.clone()));

        // Now, publish for slot 50 to reach consensus
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            publish_msg_from_solana_data(&data_s50_v1),
        )
        .unwrap();
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            publish_msg_from_solana_data(&data_s50_v1),
        )
        .unwrap();
        assert_eq!(
            LAST_PUBLISHED_DATA.load(&deps.storage).unwrap().data,
            data_s50_v1
        );
        // Pending data for slot 50 should be cleared
        let pending_key_s50 = (data_s50_v1.slot, data_s50_v1.hash().unwrap());
        assert_eq!(
            PENDING_DATA.load(&deps.storage, pending_key_s50).is_err(),
            true
        );
        // Pending data for slot 60 should remain
        assert!(PENDING_DATA.has(&deps.storage, pending_key_s60));

        // Case 9: Multiple different data's for one slot, already publications for the next slot,
        //         consensus reached for one version
        env.block.time = env.block.time.plus_seconds(100); // Advance time
        let data_s70_v1 = SolanaData {
            timestamp: env.block.time,
            slot: 70,
            custody_assets: vec![CustodyAsset {
                owned: 500,
                locked: 0,
                guaranteed_usd: 500,
                decimals: 6,
                denom: "USDC".to_string(),
            }],
            aum_usd: Uint128::new(5000),
            total_jlp_supply: Uint128::new(500),
            strategy_jlp_balance: Uint128::new(250),
        };
        let data_s70_v2 = SolanaData {
            timestamp: env.block.time,
            slot: 70,
            custody_assets: vec![CustodyAsset {
                owned: 510,
                locked: 0,
                guaranteed_usd: 510,
                decimals: 6,
                denom: "USDC".to_string(),
            }], // Different value
            aum_usd: Uint128::new(5100),
            total_jlp_supply: Uint128::new(510),
            strategy_jlp_balance: Uint128::new(255),
        };
        // Publish for slot 80 first (pending data for next slot)
        let data_s80_v1 = SolanaData {
            timestamp: env.block.time.plus_seconds(10),
            slot: 80,
            custody_assets: vec![CustodyAsset {
                owned: 600,
                locked: 0,
                guaranteed_usd: 600,
                decimals: 6,
                denom: "USDC".to_string(),
            }],
            aum_usd: Uint128::new(6000),
            total_jlp_supply: Uint128::new(600),
            strategy_jlp_balance: Uint128::new(300),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            publish_msg_from_solana_data(&data_s80_v1),
        )
        .unwrap();
        let pending_key_s80 = (data_s80_v1.slot, data_s80_v1.hash().unwrap());
        assert!(PENDING_DATA.has(&deps.storage, pending_key_s80.clone()));

        // Oracle1 publishes data_s70_v1
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            publish_msg_from_solana_data(&data_s70_v1),
        )
        .unwrap();
        // Oracle2 publishes data_s70_v2 (different)
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            publish_msg_from_solana_data(&data_s70_v2),
        )
        .unwrap();
        // Oracle3 publishes data_s70_v1 (reaching consensus for v1)
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle3, &[]),
            publish_msg_from_solana_data(&data_s70_v1),
        )
        .unwrap();
        // Assert v1 is finalized
        assert_eq!(
            LAST_PUBLISHED_DATA.load(&deps.storage).unwrap().data,
            data_s70_v1
        );
        // Assert pending data for slot 70 is cleared (both v1 and v2)
        let pending_key_s70_v1 = (data_s70_v1.slot, data_s70_v1.hash().unwrap());
        let pending_key_s70_v2 = (data_s70_v2.slot, data_s70_v2.hash().unwrap());
        assert_eq!(
            PENDING_DATA
                .load(&deps.storage, pending_key_s70_v1)
                .is_err(),
            true
        );
        assert_eq!(
            PENDING_DATA
                .load(&deps.storage, pending_key_s70_v2)
                .is_err(),
            true
        );
        // Pending data for slot 80 should remain
        assert!(PENDING_DATA.has(&deps.storage, pending_key_s80));
    }

    /// Comprehensive test suite for the `query_get_aum` query message.
    #[test]
    fn test_query_get_aum() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();

        let admin_info = message_info(&deps.api.addr_make("admin"), &[]);
        let oracle1 = deps.api.addr_make("oracle1");
        let oracle2 = deps.api.addr_make("oracle2");

        let init_msg = default_init_msg(&deps.api);
        instantiate(deps.as_mut(), env.clone(), admin_info.clone(), init_msg).unwrap();

        // --- Error Cases ---

        // Case 1: No data published yet
        let err = query(deps.as_ref(), env.clone(), QueryMsg::GetAUM {}).unwrap_err();
        assert_eq!(err, ContractError::NoDataPublished {});

        // First, publish some data and finalize it to set LAST_PUBLISHED_DATA
        let initial_data = SolanaData {
            timestamp: env.block.time,
            slot: 10,
            custody_assets: vec![CustodyAsset {
                owned: 1,
                locked: 0,
                guaranteed_usd: 1,
                decimals: 6,
                denom: "USDC".to_string(),
            }],
            aum_usd: Uint128::new(500_000),
            total_jlp_supply: Uint128::new(1_000),
            strategy_jlp_balance: Uint128::new(10_000),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            publish_msg_from_solana_data(&initial_data),
        )
        .unwrap();
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            publish_msg_from_solana_data(&initial_data),
        )
        .unwrap();
        assert!(LAST_PUBLISHED_DATA.load(&deps.storage).is_ok()); // Ensure data is published

        // Case 2: Data not valid anymore
        env.block.time = env.block.time.plus_seconds(1_001); // Advance time past valid_period (1000s)
        let err = query(deps.as_ref(), env.clone(), QueryMsg::GetAUM {}).unwrap_err();
        assert_eq!(err, ContractError::DataNotValid {});

        // Reset time for further tests
        env.block.time = env.block.time.minus_seconds(1_000); // Go back to original +1s

        // Case 3: Slinky BTC price missing or query fails (e.g., empty price string)
        deps.querier.with_price("".to_string());
        let err = query(deps.as_ref(), env.clone(), QueryMsg::GetAUM {}).unwrap_err();
        assert!(
            matches!(
                err,
                ContractError::SlinkyBTCPriceIncorrect { price: _, error: _ }
            ),
            "Expected DecimalError for empty price string"
        );
        // Revert querier to return a valid price
        deps.querier
            .with_price((25_000u64 * 1_000_000u64).to_string());

        // Case 4: Division by zero (total_jlp_supply)
        let data_zero_jlp_supply = SolanaData {
            timestamp: env.block.time,
            slot: 20,
            custody_assets: vec![CustodyAsset {
                owned: 1,
                locked: 0,
                guaranteed_usd: 1,
                decimals: 6,
                denom: "USDC".to_string(),
            }],
            aum_usd: Uint128::new(100),
            total_jlp_supply: Uint128::new(0), // Zero supply
            strategy_jlp_balance: Uint128::new(5),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            publish_msg_from_solana_data(&data_zero_jlp_supply),
        )
        .unwrap();
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            publish_msg_from_solana_data(&data_zero_jlp_supply),
        )
        .unwrap();
        let err = query(deps.as_ref(), env.clone(), QueryMsg::GetAUM {}).unwrap_err();
        assert!(
            matches!(err, ContractError::DecimalError { error } if error.contains("Denominator must not be zero")),
            "Expected DecimalError for zero total_jlp_supply"
        );

        // Case 5: Division by zero (btc_price_in_usd)
        let data_valid_aum = SolanaData {
            timestamp: env.block.time,
            slot: 30,
            custody_assets: vec![CustodyAsset {
                owned: 1,
                locked: 0,
                guaranteed_usd: 1,
                decimals: 6,
                denom: "USDC".to_string(),
            }],
            aum_usd: Uint128::new(100),
            total_jlp_supply: Uint128::new(10),
            strategy_jlp_balance: Uint128::new(5),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            publish_msg_from_solana_data(&data_valid_aum),
        )
        .unwrap();
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            publish_msg_from_solana_data(&data_valid_aum),
        )
        .unwrap();

        deps.querier.with_price("0".to_string());
        let err = query(deps.as_ref(), env.clone(), QueryMsg::GetAUM {}).unwrap_err();
        assert!(
            matches!(err, ContractError::DecimalError { error } if error.contains("Denominator must not be zero")),
            "Expected DecimalError for zero BTC price"
        );

        // Revert querier to return a valid price again for the green case
        deps.querier
            .with_price((25_000u64 * 1_000_000u64).to_string());

        // --- Green Cases ---

        // Case 6: Successful AUM calculation
        // Data for this case is already set. Now, we use larger values to get a clear integer result.
        let initial_data_large_values = SolanaData {
            timestamp: env.block.time,
            slot: 40, // Use a new slot to override previous LAST_PUBLISHED_DATA
            custody_assets: vec![CustodyAsset {
                owned: 1,
                locked: 0,
                guaranteed_usd: 1,
                decimals: 6,
                denom: "USDC".to_string(),
            }],
            aum_usd: Uint128::new(500_000_000_000u128), // 500 Billion USD
            total_jlp_supply: Uint128::new(1_000_000_000u128), // 1 Billion JLP
            strategy_jlp_balance: Uint128::new(10_000_000u128), // 10 Million JLP
        };
        // Re-publish to update LAST_PUBLISHED_DATA with these large values
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            publish_msg_from_solana_data(&initial_data_large_values),
        )
        .unwrap();
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            publish_msg_from_solana_data(&initial_data_large_values),
        )
        .unwrap();

        env.block.time = env.block.time.minus_seconds(1); // Ensure data is still valid
        let res: GetAUMResponse =
            from_json(&query(deps.as_ref(), env.clone(), QueryMsg::GetAUM {}).unwrap()).unwrap();
        assert_eq!(
            res.aum_in_btc,
            Uint128::new(200_000),
            "Case 6 Failed: Large values AUM calculation"
        );
    }

    fn publish_msg_from_solana_data(data: &SolanaData) -> ExecuteMsg {
        ExecuteMsg::PublishData {
            timestamp: data.timestamp,
            slot: data.slot,
            custody_assets: data.clone().custody_assets,
            aum_usd: data.aum_usd,
            total_jlp_supply: data.total_jlp_supply,
            strategy_jlp_balance: data.strategy_jlp_balance,
        }
    }

    fn custody_asset() -> Vec<CustodyAsset> {
        vec![CustodyAsset {
            owned: 100,
            locked: 0,
            guaranteed_usd: 100,
            decimals: 6,
            denom: "USDC".to_string(),
        }]
    }
}
