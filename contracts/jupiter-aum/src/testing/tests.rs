
// ----------------------------------------
//  Tests
// ----------------------------------------
#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{
        mock_env, message_info, MockApi
    };
    use cosmwasm_std::{attr, from_json, Decimal, Timestamp, Uint128};
    use std::str::FromStr;
    use crate::contract::{calculate_aum_in_btc, execute, instantiate, query};
    use crate::error::ContractError;
    use crate::error::ContractError::{AlreadyPublished, DataNotValid, InvalidThreshold, Unauthorized};
    use crate::msg::{ExecuteMsg, GetAUMResponse, InstantiateMsg, QueryMsg};
    use crate::state::{SolanaData, CONFIG, LAST_PUBLISHED_DATA, PENDING_DATA};
    use crate::testing::mock_querier::mock_dependencies;

    // Helper to create a default instantiate message
    fn default_init_msg(api: &MockApi) -> InstantiateMsg {
        InstantiateMsg {
            admin: api.addr_make("admin").to_string(),
            oracles: vec![
                api.addr_make("oracle1").to_string(),
                api.addr_make("oracle2").to_string(),
                api.addr_make("oracle3").to_string(),
            ],
            threshold: 2, // Default threshold for tests
            extract_period: 10,
            valid_period: 1000,
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
            extract_period: Some(100000),
            valid_period: Some(50000),
        };

        // Unauthorized update
        let stranger_info = message_info(&deps.api.addr_make("stranger"), &[]);
        let unauthorized_res = execute(
            deps.as_mut(),
            env.clone(),
            stranger_info,
            update_msg.clone(),
        );
        assert_eq!(unauthorized_res.err().unwrap(), Unauthorized {});

        // Authorized update but new config is invalid (threshold > oracles.len())
        let invalid_update_msg = ExecuteMsg::UpdateConfig {
            admin: Some(deps.api.addr_make("admin2").to_string()),
            oracles: Some(vec![deps.api.addr_make("oracle3").to_string()]), // Only 1 oracle
            threshold: Some(2), // Threshold 2
            extract_period: Some(100000),
            valid_period: Some(50000),
        };
        let authorized_res = execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            invalid_update_msg.clone(),
        );
        assert_eq!(
            authorized_res.err().unwrap(),
            InvalidThreshold {
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
        assert_eq!(config.extract_period, 100000);
        assert_eq!(config.valid_period, 50000);
    }

    #[test]
    fn test_calculate_aum_in_btc() {
        // Test case 1: Standard calculation
        let data1 = SolanaData {
            timestamp: Timestamp::from_seconds(1),
            slot: 1,
            custody_assets: Uint128::new(0), // Not used in this specific AUM formula
            aum_usd: Uint128::new(500000), // $500,000 AUM USD
            jlp_total_supply: Uint128::new(1000), // 1000 JLP total supply
            strategy_jlp_balance: Uint128::new(10000), // 10,000 JLP balance
        };
        let btc_price_in_usd1 = Decimal::from_str("25000.0").unwrap(); // $25,000 per BTC
        // jlp_virtual_price = 500,000 / 1000 = 500 USD/JLP
        // jlp_balance_in_usd = 500 * 10,000 = 5,000,000 USD
        // aum_in_btc = 5,000,000 / 25,000 = 200 BTC
        let res1 = calculate_aum_in_btc(data1, btc_price_in_usd1);
        // The `calculate_aum_in_btc` now uses `Decimal` internally and returns `to_uint_floor()`.
        // So `200` is the expected integer part of the BTC value.
        assert_eq!(res1.unwrap(), Uint128::new(200), "Test Case 1 Failed");

        // Test case 2: Different values
        let data2 = SolanaData {
            timestamp: Timestamp::from_seconds(1),
            slot: 1,
            custody_assets: Uint128::new(0),
            aum_usd: Uint128::new(1_000_000_000), // $1 Billion AUM
            jlp_total_supply: Uint128::new(50_000), // 50,000 JLP total
            strategy_jlp_balance: Uint128::new(20_000), // 20,000 JLP balance
        };
        let btc_price_in_usd2 = Decimal::from_str("50000.0").unwrap(); // $50,000 per BTC
        // jlp_virtual_price = 1,000,000,000 / 50,000 = 20,000 USD/JLP
        // jlp_balance_in_usd = 20,000 * 20,000 = 400,000,000 USD
        // aum_in_btc = 400,000,000 / 50,000 = 8,000 BTC
        let res2 = calculate_aum_in_btc(data2, btc_price_in_usd2);
        assert_eq!(res2.unwrap(), Uint128::new(8000), "Test Case 2 Failed");

        // Test case 3: Division by zero for jlp_total_supply
        let data3 = SolanaData {
            timestamp: Timestamp::from_seconds(1),
            slot: 1,
            custody_assets: Uint128::new(0),
            aum_usd: Uint128::new(100),
            jlp_total_supply: Uint128::new(0), // Zero supply
            strategy_jlp_balance: Uint128::new(10),
        };
        let btc_price_in_usd3 = Decimal::from_str("1.0").unwrap();
        let err3 = calculate_aum_in_btc(data3, btc_price_in_usd3).unwrap_err();
        assert!(matches!(err3, ContractError::DecimalError { reason } if reason.contains("Division by zero")), "Test Case 3 Failed: {:?}", err3);

        // Test case 4: Division by zero for btc_price_in_usd
        let data4 = SolanaData {
            timestamp: Timestamp::from_seconds(1),
            slot: 1,
            custody_assets: Uint128::new(0),
            aum_usd: Uint128::new(100),
            jlp_total_supply: Uint128::new(10),
            strategy_jlp_balance: Uint128::new(5),
        };
        let btc_price_in_usd4 = Decimal::from_str("0.0").unwrap(); // Zero BTC price
        let err4 = calculate_aum_in_btc(data4, btc_price_in_usd4).unwrap_err();
        assert!(matches!(err4, ContractError::DecimalError { reason } if reason.contains("Division by zero")), "Test Case 4 Failed: {:?}", err4);
    }


    /// Comprehensive test suite for the `publish_data` execute message.
    #[test]
    fn test_publish_data() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();

        let admin_info = message_info(&deps.api.addr_make("admin"), &[]);
        let oracle1 = deps.api.addr_make("oracle1");
        let oracle2 = deps.api.addr_make("oracle2");
        let oracle3 = deps.api.addr_make("oracle3");
        let non_oracle = deps.api.addr_make("non_oracle");

        let init_msg = default_init_msg(&deps.api);
        instantiate(deps.as_mut(), env.clone(), admin_info.clone(), init_msg).unwrap();

        // --- Error Cases ---

        // Case 1: Not an oracle
        let data_unauth = SolanaData {
            timestamp: env.block.time,
            slot: 10,
            custody_assets: Uint128::new(100),
            aum_usd: Uint128::new(1000),
            jlp_total_supply: Uint128::new(100),
            strategy_jlp_balance: Uint128::new(50),
        };
        let err = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&non_oracle, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_unauth.timestamp,
                slot: data_unauth.slot,
                custody_assets: data_unauth.custody_assets,
                aum_usd: data_unauth.aum_usd,
                jlp_total_supply: data_unauth.jlp_total_supply,
                strategy_jlp_balance: data_unauth.strategy_jlp_balance,
            },
        )
            .unwrap_err();
        assert_eq!(err, Unauthorized {});

        // Case 2: Incorrect slot (not multiple of extract_period)
        let data_invalid_slot = SolanaData {
            timestamp: env.block.time,
            slot: 11, // Not a multiple of 10
            custody_assets: Uint128::new(100),
            aum_usd: Uint128::new(1000),
            jlp_total_supply: Uint128::new(100),
            strategy_jlp_balance: Uint128::new(50),
        };
        let err = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_invalid_slot.timestamp,
                slot: data_invalid_slot.slot,
                custody_assets: data_invalid_slot.custody_assets,
                aum_usd: data_invalid_slot.aum_usd,
                jlp_total_supply: data_invalid_slot.jlp_total_supply,
                strategy_jlp_balance: data_invalid_slot.strategy_jlp_balance,
            },
        )
            .unwrap_err();
        assert_eq!(
            err,
            InvalidSolanaSlot {
                extract_period: 10
            }
        );

        // Case 3: Slot too old (publish data for slot 10, then try to publish for slot 10 again)
        // First, publish valid data for slot 10 from oracle1 to set LAST_PUBLISHED_DATA (part of consensus below)
        let data_s10_v1 = SolanaData {
            timestamp: env.block.time,
            slot: 10,
            custody_assets: Uint128::new(100),
            aum_usd: Uint128::new(1000),
            jlp_total_supply: Uint128::new(100),
            strategy_jlp_balance: Uint128::new(50),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_s10_v1.timestamp,
                slot: data_s10_v1.slot,
                custody_assets: data_s10_v1.custody_assets,
                aum_usd: data_s10_v1.aum_usd,
                jlp_total_supply: data_s10_v1.jlp_total_supply,
                strategy_jlp_balance: data_s10_v1.strategy_jlp_balance,
            },
        )
            .unwrap();
        // Now, publish data from oracle2 for slot 10 to reach consensus
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_s10_v1.timestamp,
                slot: data_s10_v1.slot,
                custody_assets: data_s10_v1.custody_assets,
                aum_usd: data_s10_v1.aum_usd,
                jlp_total_supply: data_s10_v1.jlp_total_supply,
                strategy_jlp_balance: data_s10_v1.strategy_jlp_balance,
            },
        )
            .unwrap();
        // Verify LAST_PUBLISHED_DATA is set
        assert!(LAST_PUBLISHED_DATA.load(&deps.storage).is_ok());

        let data_s10_too_old = SolanaData {
            timestamp: env.block.time,
            slot: 10, // Same slot as last published
            custody_assets: Uint128::new(200),
            aum_usd: Uint128::new(2000),
            jlp_total_supply: Uint128::new(200),
            strategy_jlp_balance: Uint128::new(100),
        };
        let err = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle3, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_s10_too_old.timestamp,
                slot: data_s10_too_old.slot,
                custody_assets: data_s10_too_old.custody_assets,
                aum_usd: data_s10_too_old.aum_usd,
                jlp_total_supply: data_s10_too_old.jlp_total_supply,
                strategy_jlp_balance: data_s10_too_old.strategy_jlp_balance,
            },
        )
            .unwrap_err();
        assert_eq!(
            err,
            SlotTooOld {
                new_slot: 10,
                last_slot: 10
            }
        );

        // Case 4: Double publishing by the same oracle for the same slot (and hash)
        let data_s20_v1 = SolanaData {
            timestamp: env.block.time,
            slot: 20,
            custody_assets: Uint128::new(100),
            aum_usd: Uint128::new(1000),
            jlp_total_supply: Uint128::new(100),
            strategy_jlp_balance: Uint128::new(50),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_s20_v1.timestamp,
                slot: data_s20_v1.slot,
                custody_assets: data_s20_v1.clone().custody_assets,
                aum_usd: data_s20_v1.aum_usd,
                jlp_total_supply: data_s20_v1.jlp_total_supply,
                strategy_jlp_balance: data_s20_v1.strategy_jlp_balance,
            },
        )
            .unwrap(); // First publish for slot 20 by oracle1

        let err = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_s20_v1.timestamp,
                slot: data_s20_v1.slot,
                custody_assets: data_s20_v1.custody_assets,
                aum_usd: data_s20_v1.aum_usd,
                jlp_total_supply: data_s20_v1.jlp_total_supply,
                strategy_jlp_balance: data_s20_v1.strategy_jlp_balance,
            },
        )
            .unwrap_err(); // Second publish for slot 20 by oracle1
        assert_eq!(err, AlreadyPublished {});


        // --- Green Cases ---
        // Reset state for green cases
        let mut deps = mock_dependencies();
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
            custody_assets: Uint128::new(100),
            aum_usd: Uint128::new(1000),
            jlp_total_supply: Uint128::new(100),
            strategy_jlp_balance: Uint128::new(50),
        };
        let res = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_s30_v1.timestamp,
                slot: data_s30_v1.slot,
                custody_assets: data_s30_v1.clone().custody_assets,
                aum_usd: data_s30_v1.aum_usd,
                jlp_total_supply: data_s30_v1.jlp_total_supply,
                strategy_jlp_balance: data_s30_v1.strategy_jlp_balance,
            },
        )
            .unwrap();
        assert_eq!(res.attributes.len(), 3); // action, slot, oracle
        assert_eq!(LAST_PUBLISHED_DATA.load(&deps.storage).is_err(), true); // No consensus yet
        let pending_key = (data_s30_v1.slot, data_s30_v1.hash().unwrap());
        assert!(PENDING_DATA.has(&deps.storage, pending_key.clone()));
        let pending_entries = PENDING_DATA.load(&deps.storage, pending_key).unwrap();
        assert_eq!(pending_entries.len(), 1);


        // Case 6: First consensus reached
        // Oracle2 publishes the same data for slot 30, reaching threshold (2 oracles, threshold 2)
        let res = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_s30_v1.timestamp,
                slot: data_s30_v1.slot,
                custody_assets: data_s30_v1.custody_assets,
                aum_usd: data_s30_v1.aum_usd,
                jlp_total_supply: data_s30_v1.jlp_total_supply,
                strategy_jlp_balance: data_s30_v1.strategy_jlp_balance,
            },
        )
            .unwrap();
        assert_eq!(res.attributes.len(), 5); // action, slot, oracle, consensus_reached, published_at
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
            custody_assets: Uint128::new(200),
            aum_usd: Uint128::new(2000),
            jlp_total_supply: Uint128::new(200),
            strategy_jlp_balance: Uint128::new(100),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_s40_v1.timestamp,
                slot: data_s40_v1.slot,
                custody_assets: data_s40_v1.clone().custody_assets,
                aum_usd: data_s40_v1.aum_usd,
                jlp_total_supply: data_s40_v1.jlp_total_supply,
                strategy_jlp_balance: data_s40_v1.strategy_jlp_balance,
            },
        )
            .unwrap();
        let res = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_s40_v1.timestamp,
                slot: data_s40_v1.slot,
                custody_assets: data_s40_v1.clone().custody_assets,
                aum_usd: data_s40_v1.aum_usd,
                jlp_total_supply: data_s40_v1.jlp_total_supply,
                strategy_jlp_balance: data_s40_v1.strategy_jlp_balance,
            },
        )
            .unwrap();
        assert_eq!(res.attributes.len(), 5);
        let last_published = LAST_PUBLISHED_DATA.load(&deps.storage).unwrap();
        assert_eq!(last_published.data, data_s40_v1);
        assert_eq!(last_published.published_at, env.block.time);
        // Pending data for slot 40 should be cleared
        let pending_key_s40 = (data_s40_v1.slot, data_s40_v1.hash().unwrap());
        assert_eq!(PENDING_DATA.load(&deps.storage, pending_key_s40).is_err(), true);


        // Case 8: Consensus reached, there are already publications for the next slot
        env.block.time = env.block.time.plus_seconds(100); // Advance time
        let data_s50_v1 = SolanaData {
            timestamp: env.block.time,
            slot: 50,
            custody_assets: Uint128::new(300),
            aum_usd: Uint128::new(3000),
            jlp_total_supply: Uint128::new(300),
            strategy_jlp_balance: Uint128::new(150),
        };
        let data_s60_v1 = SolanaData {
            timestamp: env.block.time.plus_seconds(10), // A bit later
            slot: 60,
            custody_assets: Uint128::new(400),
            aum_usd: Uint128::new(4000),
            jlp_total_supply: Uint128::new(400),
            strategy_jlp_balance: Uint128::new(200),
        };
        // Publish for slot 60 first (no consensus yet)
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_s60_v1.timestamp,
                slot: data_s60_v1.slot,
                custody_assets: data_s60_v1.clone().custody_assets,
                aum_usd: data_s60_v1.aum_usd,
                jlp_total_supply: data_s60_v1.jlp_total_supply,
                strategy_jlp_balance: data_s60_v1.strategy_jlp_balance,
            },
        )
            .unwrap();
        let pending_key_s60 = (data_s60_v1.slot, data_s60_v1.hash().unwrap());
        assert!(PENDING_DATA.has(&deps.storage, pending_key_s60));

        // Now, publish for slot 50 to reach consensus
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_s50_v1.timestamp,
                slot: data_s50_v1.slot,
                custody_assets: data_s50_v1.clone().custody_assets,
                aum_usd: data_s50_v1.aum_usd,
                jlp_total_supply: data_s50_v1.jlp_total_supply,
                strategy_jlp_balance: data_s50_v1.strategy_jlp_balance,
            },
        )
            .unwrap();
        let res = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_s50_v1.timestamp,
                slot: data_s50_v1.slot,
                custody_assets: data_s50_v1.clone().custody_assets,
                aum_usd: data_s50_v1.aum_usd,
                jlp_total_supply: data_s50_v1.jlp_total_supply,
                strategy_jlp_balance: data_s50_v1.strategy_jlp_balance,
            },
        )
            .unwrap();
        assert_eq!(LAST_PUBLISHED_DATA.load(&deps.storage).unwrap().data, data_s50_v1);
        // Pending data for slot 50 should be cleared
        let pending_key_s50 = (data_s50_v1.slot, data_s50_v1.hash().unwrap());
        assert_eq!(PENDING_DATA.load(&deps.storage, pending_key_s50).is_err(), true);
        // Pending data for slot 60 should remain
        assert!(PENDING_DATA.has(&deps.storage, pending_key_s60));


        // Case 9: Multiple different data's for one slot, already publications for the next slot,
        //         consensus reached for one version
        env.block.time = env.block.time.plus_seconds(100); // Advance time
        let data_s70_v1 = SolanaData {
            timestamp: env.block.time,
            slot: 70,
            custody_assets: Uint128::new(500),
            aum_usd: Uint128::new(5000),
            jlp_total_supply: Uint128::new(500),
            strategy_jlp_balance: Uint128::new(250),
        };
        let data_s70_v2 = SolanaData {
            timestamp: env.block.time,
            slot: 70,
            custody_assets: Uint128::new(510), // Different value
            aum_usd: Uint128::new(5100),
            jlp_total_supply: Uint128::new(510),
            strategy_jlp_balance: Uint128::new(255),
        };
        // Publish for slot 80 first (pending data for next slot)
        let data_s80_v1 = SolanaData {
            timestamp: env.block.time.plus_seconds(10),
            slot: 80,
            custody_assets: Uint128::new(600),
            aum_usd: Uint128::new(6000),
            jlp_total_supply: Uint128::new(600),
            strategy_jlp_balance: Uint128::new(300),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_s80_v1.timestamp,
                slot: data_s80_v1.slot,
                custody_assets: data_s80_v1.clone().custody_assets,
                aum_usd: data_s80_v1.aum_usd,
                jlp_total_supply: data_s80_v1.jlp_total_supply,
                strategy_jlp_balance: data_s80_v1.strategy_jlp_balance,
            },
        )
            .unwrap();
        let pending_key_s80 = (data_s80_v1.slot, data_s80_v1.hash().unwrap());
        assert!(PENDING_DATA.has(&deps.storage, pending_key_s80));

        // Oracle1 publishes data_s70_v1
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_s70_v1.timestamp,
                slot: data_s70_v1.slot,
                custody_assets: data_s70_v1.clone().custody_assets,
                aum_usd: data_s70_v1.aum_usd,
                jlp_total_supply: data_s70_v1.jlp_total_supply,
                strategy_jlp_balance: data_s70_v1.strategy_jlp_balance,
            },
        )
            .unwrap();
        // Oracle2 publishes data_s70_v2 (different)
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_s70_v2.timestamp,
                slot: data_s70_v2.slot,
                custody_assets: data_s70_v2.clone().custody_assets,
                aum_usd: data_s70_v2.aum_usd,
                jlp_total_supply: data_s70_v2.jlp_total_supply,
                strategy_jlp_balance: data_s70_v2.strategy_jlp_balance,
            },
        )
            .unwrap();
        // Oracle3 publishes data_s70_v1 (reaching consensus for v1)
        let res = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle3, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_s70_v1.timestamp,
                slot: data_s70_v1.slot,
                custody_assets: data_s70_v1.clone().custody_assets,
                aum_usd: data_s70_v1.aum_usd,
                jlp_total_supply: data_s70_v1.jlp_total_supply,
                strategy_jlp_balance: data_s70_v1.strategy_jlp_balance,
            },
        )
            .unwrap();
        // Assert v1 is finalized
        assert_eq!(LAST_PUBLISHED_DATA.load(&deps.storage).unwrap().data, data_s70_v1);
        // Assert pending data for slot 70 is cleared (both v1 and v2)
        let pending_key_s70_v1 = (data_s70_v1.slot, data_s70_v1.hash().unwrap());
        let pending_key_s70_v2 = (data_s70_v2.slot, data_s70_v2.hash().unwrap());
        assert_eq!(PENDING_DATA.load(&deps.storage, pending_key_s70_v1).is_err(), true);
        assert_eq!(PENDING_DATA.load(&deps.storage, pending_key_s70_v2).is_err(), true);
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
        assert_eq!(err, NoDataPublished {});

        // First, publish some data and finalize it to set LAST_PUBLISHED_DATA
        let initial_data = SolanaData {
            timestamp: env.block.time,
            slot: 10,
            custody_assets: Uint128::new(1),
            aum_usd: Uint128::new(500_000),
            jlp_total_supply: Uint128::new(1000),
            strategy_jlp_balance: Uint128::new(10000),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            ExecuteMsg::PublishData {
                timestamp: initial_data.timestamp,
                slot: initial_data.slot,
                custody_assets: initial_data.custody_assets,
                aum_usd: initial_data.aum_usd,
                jlp_total_supply: initial_data.jlp_total_supply,
                strategy_jlp_balance: initial_data.strategy_jlp_balance,
            },
        )
            .unwrap();
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            ExecuteMsg::PublishData {
                timestamp: initial_data.timestamp,
                slot: initial_data.slot,
                custody_assets: initial_data.custody_assets,
                aum_usd: initial_data.aum_usd,
                jlp_total_supply: initial_data.jlp_total_supply,
                strategy_jlp_balance: initial_data.strategy_jlp_balance,
            },
        )
            .unwrap();
        assert!(LAST_PUBLISHED_DATA.load(&deps.storage).is_ok()); // Ensure data is published

        // Case 2: Data not valid anymore
        env.block.time = env.block.time.plus_seconds(1001); // Advance time past valid_period (1000s)
        let err = query(deps.as_ref(), env.clone(), QueryMsg::GetAUM {}).unwrap_err();
        assert_eq!(err, DataNotValid {});

        // Reset time for further tests
        env.block.time = env.block.time.minus_seconds(1000); // Go back to original +1s

        // Case 3: Slinky BTC price missing or query fails (e.g., empty price string)
        deps.querier.with_price("".to_string());
        let err = query(deps.as_ref(), env.clone(), QueryMsg::GetAUM {}).unwrap_err();
        assert!(matches!(err, ContractError::DecimalError { reason } if reason.contains("Failed to parse price string")), "Expected DecimalError for empty price string");
        // Revert querier to return a valid price
        deps.querier.with_price((25_000 * 1_000_000).to_string());

        // Case 4: Division by zero (jlp_total_supply) - Requires re-publishing data
        let data_zero_jlp_supply = SolanaData {
            timestamp: env.block.time,
            slot: 20,
            custody_assets: Uint128::new(1),
            aum_usd: Uint128::new(100),
            jlp_total_supply: Uint128::new(0), // Zero supply
            strategy_jlp_balance: Uint128::new(5),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_zero_jlp_supply.timestamp,
                slot: data_zero_jlp_supply.slot,
                custody_assets: data_zero_jlp_supply.clone().custody_assets,
                aum_usd: data_zero_jlp_supply.aum_usd,
                jlp_total_supply: data_zero_jlp_supply.jlp_total_supply,
                strategy_jlp_balance: data_zero_jlp_supply.strategy_jlp_balance,
            },
        )
            .unwrap();
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_zero_jlp_supply.timestamp,
                slot: data_zero_jlp_supply.slot,
                custody_assets: data_zero_jlp_supply.custody_assets,
                aum_usd: data_zero_jlp_supply.aum_usd,
                jlp_total_supply: data_zero_jlp_supply.jlp_total_supply,
                strategy_jlp_balance: data_zero_jlp_supply.strategy_jlp_balance,
            },
        )
            .unwrap();
        let err = query(deps.as_ref(), env.clone(), QueryMsg::GetAUM {}).unwrap_err();
        assert!(matches!(err, ContractError::DecimalError { reason } if reason.contains("Division by zero")), "Expected DecimalError for zero jlp_total_supply");

        // Case 5: Division by zero (btc_price_in_usd) - Requires re-setting querier
        let data_valid_aum = SolanaData {
            timestamp: env.block.time,
            slot: 30,
            custody_assets: Uint128::new(1),
            aum_usd: Uint128::new(100),
            jlp_total_supply: Uint128::new(10),
            strategy_jlp_balance: Uint128::new(5),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_valid_aum.timestamp,
                slot: data_valid_aum.slot,
                custody_assets: data_valid_aum.clone().custody_assets,
                aum_usd: data_valid_aum.aum_usd,
                jlp_total_supply: data_valid_aum.jlp_total_supply,
                strategy_jlp_balance: data_valid_aum.strategy_jlp_balance,
            },
        )
            .unwrap();
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            ExecuteMsg::PublishData {
                timestamp: data_valid_aum.timestamp,
                slot: data_valid_aum.slot,
                custody_assets: data_valid_aum.custody_assets,
                aum_usd: data_valid_aum.aum_usd,
                jlp_total_supply: data_valid_aum.jlp_total_supply,
                strategy_jlp_balance: data_valid_aum.strategy_jlp_balance,
            },
        )
            .unwrap();

        deps.querier.with_price("0".to_string());
        let err = query(deps.as_ref(), env.clone(), QueryMsg::GetAUM {}).unwrap_err();
        assert!(matches!(err, ContractError::DecimalError { reason } if reason.contains("Division by zero")), "Expected DecimalError for zero BTC price");

        // Revert querier to return a valid price again for green case
        deps.querier.with_price((25_000 * 1_000_000).to_string());


        // --- Green Cases ---

        // Case 6: Successful AUM calculation
        // Data for this case is already set. Now, we use larger values to get a clear integer result.
        let initial_data_large_values = SolanaData {
            timestamp: env.block.time,
            slot: 40, // Use a new slot to override previous LAST_PUBLISHED_DATA
            custody_assets: Uint128::new(1),
            aum_usd: Uint128::new(500_000_000_000u128), // 500 Billion USD
            jlp_total_supply: Uint128::new(1_000_000_000u128), // 1 Billion JLP
            strategy_jlp_balance: Uint128::new(10_000_000u128), // 10 Million JLP
        };
        // Re-publish to update LAST_PUBLISHED_DATA with these large values
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle1, &[]),
            ExecuteMsg::PublishData {
                timestamp: initial_data_large_values.timestamp,
                slot: initial_data_large_values.slot,
                custody_assets: initial_data_large_values.clone().custody_assets,
                aum_usd: initial_data_large_values.aum_usd,
                jlp_total_supply: initial_data_large_values.jlp_total_supply,
                strategy_jlp_balance: initial_data_large_values.strategy_jlp_balance,
            },
        )
            .unwrap();
        execute(
            deps.as_mut(),
            env.clone(),
            message_info(&oracle2, &[]),
            ExecuteMsg::PublishData {
                timestamp: initial_data_large_values.timestamp,
                slot: initial_data_large_values.slot,
                custody_assets: initial_data_large_values.custody_assets,
                aum_usd: initial_data_large_values.aum_usd,
                jlp_total_supply: initial_data_large_values.jlp_total_supply,
                strategy_jlp_balance: initial_data_large_values.strategy_jlp_balance,
            },
        )
            .unwrap();

        env.block.time = env.block.time.minus_seconds(1); // Ensure data is still valid
        let res: GetAUMResponse =
            from_json(&query(deps.as_ref(), env.clone(), QueryMsg::GetAUM {}).unwrap()).unwrap();
        assert_eq!(res.aum_in_btc, Uint128::new(200_000), "Case 6 Failed: Large values AUM calculation");
    }
}