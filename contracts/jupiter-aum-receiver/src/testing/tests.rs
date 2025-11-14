use crate::contract::{calculate_aum_in_wbtc, execute, instantiate, query};
use crate::state::{CONFIG, CONSENSUS_STATE};
use crate::testing::mock_querier::{mock_dependencies, WasmMockQuerier};
use aum_receiver_common::types::GetAumResponse;
use consensus::consensus::{Config as ConsensusConfig, Round};
use consensus::error::ConsensusError;
use cosmwasm_std::testing::{message_info, mock_env, MockApi, MockStorage};
use cosmwasm_std::{
    from_json, Env, Int256, OwnedDeps, Response, SignedDecimal256, Timestamp, Uint128,
};
use cw_ownable::Action;
use cw_ownable::OwnershipError::{NotOwner, NotPendingOwner};
use jupiter_aum_common::error::ContractError;
use jupiter_aum_common::msg;
use jupiter_aum_common::msg::ExecuteMsg::UpdateConfig;
use jupiter_aum_common::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use jupiter_aum_common::types::{
    CustodyAsset, SolanaBalance, SolanaData, SolanaTokenDecimals, SolanaTokenTotalSupply,
};
use std::collections::HashMap;
use std::str::FromStr;

// Helper to create a default instantiate message
fn default_init_msg(api: &MockApi) -> InstantiateMsg {
    InstantiateMsg {
        owner: api.addr_make("owner").to_string(),
        messengers: vec![
            api.addr_make("messenger1").to_string(),
            api.addr_make("messenger2").to_string(),
            api.addr_make("messenger3").to_string(),
        ],
        threshold: 2,
        data_delta_ppm: 1000,
        round_length: 100,
        consensus_data_valid_period: 1_000,
        required_custody_assets: vec!["USDC".to_string()],
        price_data_valid_period: 100,
        required_solana_balances: HashMap::from([(
            "strategy".to_string(),
            vec!["jlp_token".to_string()],
        )]),
        required_solana_token_total_supply: vec!["jlp_token".to_string()],
        strategy_address: "strategy".to_string(),
        jlp_token: "jlp_token".to_string(),
    }
}

#[test]
fn test_ownership() {
    let mut deps = cosmwasm_std::testing::mock_dependencies();
    let env = mock_env();
    let first_owner = deps.api.addr_make("first_owner");
    let second_owner = deps.api.addr_make("second_owner");
    let other = deps.api.addr_make("other");

    let owner_info = message_info(&first_owner, &[]);
    let second_owner_info = message_info(&second_owner, &[]);
    let other_info = message_info(&other, &[]);

    // Test empty vector for messengers
    let mut msg = default_init_msg(&deps.api);
    msg.owner = first_owner.to_string();
    let result = instantiate(deps.as_mut(), env.clone(), owner_info.clone(), msg);
    assert!(result.is_ok());

    // owner is written in instantiate
    let ownership = cw_ownable::get_ownership(&deps.storage).unwrap();
    assert_eq!(ownership.owner.unwrap(), first_owner);

    // non-owner cannot transfer ownership
    let update_msg_1 = ExecuteMsg::UpdateOwnership(Action::TransferOwnership {
        new_owner: other.to_string(),
        expiry: None,
    });
    let result = execute(deps.as_mut(), env.clone(), other_info.clone(), update_msg_1).unwrap_err();
    assert_eq!(result, ContractError::Ownable(NotOwner));

    // transfer ownership writes pending owner
    let update_msg_2 = ExecuteMsg::UpdateOwnership(Action::TransferOwnership {
        new_owner: second_owner.to_string(),
        expiry: None,
    });
    let result = execute(deps.as_mut(), env.clone(), owner_info, update_msg_2);
    assert!(result.is_ok());
    let ownership = cw_ownable::get_ownership(&deps.storage).unwrap();
    assert_eq!(ownership.owner.unwrap(), first_owner);
    assert_eq!(ownership.pending_owner.unwrap(), second_owner);

    // other user cannot accept ownership
    let update_msg_3 = ExecuteMsg::UpdateOwnership(Action::AcceptOwnership {});
    let result = execute(deps.as_mut(), env.clone(), other_info, update_msg_3).unwrap_err();
    assert_eq!(result, ContractError::Ownable(NotPendingOwner));

    // pending owner can accept ownership
    let update_msg_4 = ExecuteMsg::UpdateOwnership(Action::AcceptOwnership {});
    let result = execute(deps.as_mut(), env.clone(), second_owner_info, update_msg_4);
    assert!(result.is_ok());
    let ownership = cw_ownable::get_ownership(&deps.storage).unwrap();
    assert_eq!(ownership.owner.unwrap(), second_owner);
}

#[test]
fn test_instantiate_validation() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner_info = message_info(&deps.api.addr_make("owner"), &[]);

    // Test zero value for consensus_data_valid_period
    let mut msg = default_init_msg(&deps.api);
    msg.consensus_data_valid_period = 0;
    let result = instantiate(deps.as_mut(), env.clone(), owner_info.clone(), msg);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ContractError::InvalidConsensusPeriod {}
    );

    // Test zero value for price_data_valid_period
    let mut msg = default_init_msg(&deps.api);
    msg.price_data_valid_period = 0;
    let result = instantiate(deps.as_mut(), env.clone(), owner_info.clone(), msg);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ContractError::InvalidPriceDataPeriod {}
    );

    // Test empty vector for messengers
    let mut msg = default_init_msg(&deps.api);
    msg.messengers = vec![];
    let result = instantiate(deps.as_mut(), env.clone(), owner_info.clone(), msg);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ContractError::ConsensusError(ConsensusError::EmptyMessengers {})
    );

    let messenger_a = deps.api.addr_make("messenger_a");
    let messenger_b = deps.api.addr_make("messenger_b");
    let messenger_c = deps.api.addr_make("messenger_c");

    // Test vector with duplicates for messengers
    let mut msg = default_init_msg(&deps.api);
    msg.messengers = vec![
        messenger_a.to_string(),
        messenger_b.to_string(),
        messenger_a.to_string(),
        messenger_c.to_string(),
    ];
    let result = instantiate(deps.as_mut(), env.clone(), owner_info.clone(), msg);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ContractError::ConsensusError(ConsensusError::DuplicateMessengers {})
    );

    // Test zero value for threshold
    let mut msg = default_init_msg(&deps.api);
    msg.threshold = 0;
    let result = instantiate(deps.as_mut(), env.clone(), owner_info.clone(), msg);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ContractError::ConsensusError(ConsensusError::ZeroThreshold {})
    );

    // Test unreachable value for threshold
    let mut msg = default_init_msg(&deps.api);
    msg.threshold = msg.messengers.len() as u32 + 1;
    let result = instantiate(deps.as_mut(), env, owner_info, msg);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ContractError::ConsensusError(ConsensusError::UnreachableThreshold {})
    );
}

#[test]
fn test_update_config() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner_info = message_info(&deps.api.addr_make("owner"), &[]);
    let msg = default_init_msg(&deps.api);
    instantiate(deps.as_mut(), env.clone(), owner_info.clone(), msg).unwrap();

    let update = msg::UpdateConfig {
        consensus_data_valid_period: Some(50_000),
        required_custody_assets: Some(vec!["BTC".to_string()]),
        price_data_valid_period: Some(999),
        messengers: Some(vec![deps.api.addr_make("messenger1").to_string()]),
        threshold: Some(1),
        data_delta_ppm: Some(1234),
        round_length: Some(99),
        required_solana_balances: Some(HashMap::from([(
            "new_strategy".to_string(),
            vec!["new_jlp_token".to_string()],
        )])),
        required_solana_token_total_supply: Some(vec!["new_jlp_token".to_string()]),
        strategy_address: Some("new_strategy".to_string()),
        jlp_token: Some("new_jlp_token".to_string()),
    };

    let update_msg = UpdateConfig {
        new_config: update.clone(),
    };

    // Unauthorized
    let stranger_info = message_info(&deps.api.addr_make("stranger"), &[]);
    let unauthorized = execute(
        deps.as_mut(),
        env.clone(),
        stranger_info,
        update_msg.clone(),
    );
    assert_eq!(unauthorized.unwrap_err(), ContractError::Ownable(NotOwner));

    // Authorized
    let authorized = execute(deps.as_mut(), env.clone(), owner_info.clone(), update_msg);
    assert!(authorized.is_ok());

    // Assert config updated
    let config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(config.consensus_data_valid_period, 50_000);
    assert_eq!(config.required_custody_assets, vec!["BTC"]);
    assert_eq!(config.price_data_valid_period, 999);
    assert_eq!(
        config.required_solana_balances,
        HashMap::from([(
            "new_strategy".to_string(),
            vec!["new_jlp_token".to_string()],
        )])
    );
    assert_eq!(
        config.required_solana_token_total_supply,
        vec!["new_jlp_token".to_string()]
    );
    assert_eq!(config.strategy_address, "new_strategy".to_string());
    assert_eq!(config.jlp_token, "new_jlp_token".to_string());

    // Pending config also updated
    let consensus = CONSENSUS_STATE.pending_config.load(&deps.storage).unwrap();
    assert_eq!(consensus.threshold, 1);
    assert_eq!(consensus.data_delta_ppm, 1234);
    assert_eq!(consensus.round_length, 99);
    assert_eq!(consensus.messengers, vec![deps.api.addr_make("messenger1")]);
}

#[test]
fn test_update_config_validation() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner_info = message_info(&deps.api.addr_make("owner"), &[]);
    let msg = default_init_msg(&deps.api);
    instantiate(deps.as_mut(), env.clone(), owner_info.clone(), msg.clone()).unwrap();

    // Test zero value for consensus_data_valid_period
    let update = msg::UpdateConfig {
        consensus_data_valid_period: Some(0),
        required_custody_assets: None,
        price_data_valid_period: None,
        messengers: None,
        threshold: None,
        data_delta_ppm: None,
        round_length: None,
        required_solana_balances: None,
        required_solana_token_total_supply: None,
        strategy_address: None,
        jlp_token: None,
    };
    let update_msg = UpdateConfig {
        new_config: update.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), owner_info.clone(), update_msg);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ContractError::InvalidConsensusPeriod {}
    );

    // Test zero value for price_data_valid_period
    let update = msg::UpdateConfig {
        consensus_data_valid_period: None,
        required_custody_assets: None,
        price_data_valid_period: Some(0),
        messengers: None,
        threshold: None,
        data_delta_ppm: None,
        round_length: None,
        required_solana_balances: None,
        required_solana_token_total_supply: None,
        strategy_address: None,
        jlp_token: None,
    };
    let update_msg = UpdateConfig {
        new_config: update.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), owner_info.clone(), update_msg);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ContractError::InvalidPriceDataPeriod {}
    );

    // Test empty vector for messengers
    let update = msg::UpdateConfig {
        consensus_data_valid_period: None,
        required_custody_assets: None,
        price_data_valid_period: None,
        messengers: Some(vec![]),
        threshold: None,
        data_delta_ppm: None,
        round_length: None,
        required_solana_balances: None,
        required_solana_token_total_supply: None,
        strategy_address: None,
        jlp_token: None,
    };
    let update_msg = UpdateConfig {
        new_config: update.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), owner_info.clone(), update_msg);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ContractError::ConsensusError(ConsensusError::EmptyMessengers {})
    );

    let messenger_a = deps.api.addr_make("messenger_a");
    let messenger_b = deps.api.addr_make("messenger_b");
    let messenger_c = deps.api.addr_make("messenger_c");

    // Test vector with duplicates for messengers
    let update = msg::UpdateConfig {
        consensus_data_valid_period: None,
        required_custody_assets: None,
        price_data_valid_period: None,
        messengers: Some(vec![
            messenger_a.to_string(),
            messenger_b.to_string(),
            messenger_a.to_string(),
            messenger_c.to_string(),
        ]),
        threshold: None,
        data_delta_ppm: None,
        round_length: None,
        required_solana_balances: None,
        required_solana_token_total_supply: None,
        strategy_address: None,
        jlp_token: None,
    };
    let update_msg = UpdateConfig {
        new_config: update.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), owner_info.clone(), update_msg);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ContractError::ConsensusError(ConsensusError::DuplicateMessengers {})
    );

    // Test zero value for threshold
    let update = msg::UpdateConfig {
        consensus_data_valid_period: None,
        required_custody_assets: None,
        price_data_valid_period: None,
        messengers: None,
        threshold: Some(0),
        data_delta_ppm: None,
        round_length: None,
        required_solana_balances: None,
        required_solana_token_total_supply: None,
        strategy_address: None,
        jlp_token: None,
    };
    let update_msg = UpdateConfig {
        new_config: update.clone(),
    };
    let result = execute(deps.as_mut(), env.clone(), owner_info.clone(), update_msg);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ContractError::ConsensusError(ConsensusError::ZeroThreshold {})
    );

    // Test unreachable value for threshold
    let update = msg::UpdateConfig {
        consensus_data_valid_period: None,
        required_custody_assets: None,
        price_data_valid_period: None,
        messengers: None,
        threshold: Some(msg.messengers.len() as u32 + 1),
        data_delta_ppm: None,
        round_length: None,
        required_solana_balances: None,
        required_solana_token_total_supply: None,
        strategy_address: None,
        jlp_token: None,
    };
    let update_msg = UpdateConfig { new_config: update };
    let result = execute(deps.as_mut(), env, owner_info, update_msg);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ContractError::ConsensusError(ConsensusError::UnreachableThreshold {})
    );
}

#[test]
fn test_calculate_aum_in_wbtc() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner_info = message_info(&deps.api.addr_make("owner"), &[]);
    let msg = default_init_msg(&deps.api);
    instantiate(deps.as_mut(), env.clone(), owner_info.clone(), msg.clone()).unwrap();
    let config = CONFIG.load(&deps.storage).unwrap();

    // Test case 1: Standard calculation
    let data1 = SolanaData {
        custody_assets: vec![],
        aum_usd: Uint128::new(500_000), // $500,000 AUM USD
        solana_balances: vec![SolanaBalance {
            address: "strategy".to_string(),
            asset: "jlp_token".to_string(),
            amount: Uint128::new(10_000_000_000), // 10,000 JLP balance
        }],
        solana_token_total_supply: vec![SolanaTokenTotalSupply {
            asset: "jlp_token".to_string(),
            total_supply: Uint128::new(1_000_000_000), // 1,000 JLP total supply
        }],
        solana_token_decimals: vec![SolanaTokenDecimals {
            asset: "jlp_token".to_string(),
            decimals: 6,
        }],
    };
    let btc_price_in_usd1 = SignedDecimal256::from_str("25000.0").unwrap(); // $25,000 per BTC
                                                                            // jlp_virtual_price = 500,000 / 1,000 = 500 USD/JLP
                                                                            // jlp_balance_in_usd = 500 * 10,000 = 5,000,000 USD
                                                                            // aum_in_btc = 5,000,000 / 25,000 = 200 BTC
    let res1 = calculate_aum_in_wbtc(data1, btc_price_in_usd1, &config);
    assert_eq!(
        res1.unwrap(),
        Int256::from(20_000_000_000_i128),
        "Test Case 1 Failed"
    );

    // Test case 2: Different values
    let data2 = SolanaData {
        custody_assets: vec![],
        aum_usd: Uint128::new(1_000_000_000), // $1 Billion AUM
        solana_balances: vec![SolanaBalance {
            address: "strategy".to_string(),
            asset: "jlp_token".to_string(),
            amount: Uint128::new(20_000_000_000), // 20,000 JLP balance
        }],
        solana_token_total_supply: vec![SolanaTokenTotalSupply {
            asset: "jlp_token".to_string(),
            total_supply: Uint128::new(50_000_000_000), // 50,000 JLP total supply
        }],
        solana_token_decimals: vec![SolanaTokenDecimals {
            asset: "jlp_token".to_string(),
            decimals: 6,
        }],
    };
    let btc_price_in_usd2 = SignedDecimal256::from_str("50000.0").unwrap(); // $50,000 per BTC
                                                                            // jlp_virtual_price = 1,000,000,000 / 50,000 = 20,000 USD/JLP
                                                                            // jlp_balance_in_usd = 20,000 * 20,000 = 400,000,000 USD
                                                                            // aum_in_btc = 400,000,000 / 50,000 = 8,000 BTC
    let res2 = calculate_aum_in_wbtc(data2, btc_price_in_usd2, &config);
    assert_eq!(
        res2.unwrap(),
        Int256::from(800_000_000_000_i128),
        "Test Case 2 Failed"
    );

    // Test case 3: Division by zero for total_jlp_supply
    let data3 = SolanaData {
        custody_assets: vec![],
        aum_usd: Uint128::new(100),
        solana_balances: vec![SolanaBalance {
            address: "strategy".to_string(),
            asset: "jlp_token".to_string(),
            amount: Uint128::new(10), // 10 JLP balance
        }],
        solana_token_total_supply: vec![SolanaTokenTotalSupply {
            asset: "jlp_token".to_string(),
            total_supply: Uint128::zero(), // Zero supply
        }],
        solana_token_decimals: vec![SolanaTokenDecimals {
            asset: "jlp_token".to_string(),
            decimals: 6,
        }],
    };
    let btc_price_in_usd3 = SignedDecimal256::from_str("1.0").unwrap();
    let err3 = calculate_aum_in_wbtc(data3, btc_price_in_usd3, &config).unwrap_err();
    assert!(
        matches!(&err3, ContractError::CheckedDiv(_)),
        "Test Case 3 Failed: {:?}",
        err3
    );

    // Test case 4: Division by zero for btc_price_in_usd
    let data4 = SolanaData {
        custody_assets: vec![],
        aum_usd: Uint128::new(100),
        solana_balances: vec![SolanaBalance {
            address: "strategy".to_string(),
            asset: "jlp_token".to_string(),
            amount: Uint128::new(5),
        }],
        solana_token_total_supply: vec![SolanaTokenTotalSupply {
            asset: "jlp_token".to_string(),
            total_supply: Uint128::new(10),
        }],
        solana_token_decimals: vec![SolanaTokenDecimals {
            asset: "jlp_token".to_string(),
            decimals: 6,
        }],
    };
    let btc_price_in_usd4 = SignedDecimal256::from_str("0.0").unwrap(); // Zero BTC price
    let err4 = calculate_aum_in_wbtc(data4, btc_price_in_usd4, &config).unwrap_err();
    assert!(
        matches!(&err4, ContractError::CheckedDiv(_)),
        "Test Case 4 Failed: {:?}",
        err4
    );
}

#[test]
fn test_calculate_aum_in_wbtc_missing_jlp_total_supply() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner_info = message_info(&deps.api.addr_make("owner"), &[]);
    let msg = default_init_msg(&deps.api);
    instantiate(deps.as_mut(), env.clone(), owner_info.clone(), msg.clone()).unwrap();
    let config = CONFIG.load(&deps.storage).unwrap();

    let data = SolanaData {
        custody_assets: vec![],
        aum_usd: Uint128::new(500_000),
        solana_balances: vec![SolanaBalance {
            address: "strategy".to_string(),
            asset: "jlp_token".to_string(),
            amount: Uint128::new(10_000_000_000),
        }],
        solana_token_total_supply: vec![SolanaTokenTotalSupply {
            asset: "some_token".to_string(), // missing JLP total supply
            total_supply: Uint128::new(1_000_000_000),
        }],
        solana_token_decimals: vec![SolanaTokenDecimals {
            asset: "jlp_token".to_string(),
            decimals: 6,
        }],
    };
    let btc_price_in_usd = SignedDecimal256::from_str("25000.0").unwrap();
    assert_eq!(
        calculate_aum_in_wbtc(data, btc_price_in_usd, &config).unwrap_err(),
        ContractError::CrucialConsensusDataMissing {
            details: "JLP total supply".to_string()
        }
    );
}

#[test]
fn test_calculate_aum_in_wbtc_missing_jlp_decimals() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner_info = message_info(&deps.api.addr_make("owner"), &[]);
    let msg = default_init_msg(&deps.api);
    instantiate(deps.as_mut(), env.clone(), owner_info.clone(), msg.clone()).unwrap();
    let config = CONFIG.load(&deps.storage).unwrap();

    let data = SolanaData {
        custody_assets: vec![],
        aum_usd: Uint128::new(500_000),
        solana_balances: vec![SolanaBalance {
            address: "strategy".to_string(),
            asset: "jlp_token".to_string(),
            amount: Uint128::new(10_000_000_000),
        }],
        solana_token_total_supply: vec![SolanaTokenTotalSupply {
            asset: "jlp_token".to_string(),
            total_supply: Uint128::new(1_000_000_000),
        }],
        solana_token_decimals: vec![SolanaTokenDecimals {
            asset: "some_token".to_string(), // missing JLP decimals
            decimals: 6,
        }],
    };
    let btc_price_in_usd = SignedDecimal256::from_str("25000.0").unwrap();
    assert_eq!(
        calculate_aum_in_wbtc(data, btc_price_in_usd, &config).unwrap_err(),
        ContractError::CrucialConsensusDataMissing {
            details: "JLP decimals".to_string()
        }
    );
}

#[test]
fn test_calculate_aum_in_wbtc_missing_strategy_jlp_balance() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner_info = message_info(&deps.api.addr_make("owner"), &[]);
    let msg = default_init_msg(&deps.api);
    instantiate(deps.as_mut(), env.clone(), owner_info.clone(), msg.clone()).unwrap();
    let config = CONFIG.load(&deps.storage).unwrap();

    let data = SolanaData {
        custody_assets: vec![],
        aum_usd: Uint128::new(500_000),
        solana_balances: vec![SolanaBalance {
            address: "some_address".to_string(), // missing Strategy JLP balance
            asset: "jlp_token".to_string(),
            amount: Uint128::new(10_000_000_000),
        }],
        solana_token_total_supply: vec![SolanaTokenTotalSupply {
            asset: "jlp_token".to_string(),
            total_supply: Uint128::new(1_000_000_000),
        }],
        solana_token_decimals: vec![SolanaTokenDecimals {
            asset: "jlp_token".to_string(),
            decimals: 6,
        }],
    };
    let btc_price_in_usd = SignedDecimal256::from_str("25000.0").unwrap();
    assert_eq!(
        calculate_aum_in_wbtc(data, btc_price_in_usd, &config).unwrap_err(),
        ContractError::CrucialConsensusDataMissing {
            details: "Strategy JLP balance".to_string()
        }
    );
}

/// Comprehensive test suite for the `query_get_aum` query message.
#[test]
fn test_query_get_aum_behavior() {
    let mut deps = mock_dependencies();
    let mut env = mock_env();

    // 1. error: no data published yet
    let owner_info = message_info(&deps.api.addr_make("owner"), &[]);
    let init_msg = default_init_msg(&deps.api);
    instantiate(deps.as_mut(), env.clone(), owner_info, init_msg).unwrap();
    let res = query(deps.as_ref(), env.clone(), QueryMsg::GetAum {});
    assert!(matches!(res, Err(ContractError::NoDataPublished {})));

    let current_height = env.block.height;
    let (deps, exec_res) = publish_till_consensus(
        mock_dependencies(),
        &mut env,
        "10000".to_string(),
        current_height,
    );
    assert!(exec_res.is_ok());

    // 2. error: data published, but too old (time-based expiration)
    env.block.time = env.block.time.plus_seconds(10_000);
    let res = query(deps.as_ref(), env.clone(), QueryMsg::GetAum {});
    assert!(matches!(res, Err(ContractError::PublishedDataTooOld {})));

    // reset time
    env.block.time = env.block.time.minus_seconds(9_900);
    env.block.height = 200;

    // 3. error: no BTC price returned from oracle
    let mut env = mock_env();
    let (_, exec_res) = publish_till_consensus(mock_dependencies(), &mut env, "".to_string(), 200);
    assert!(matches!(
        exec_res,
        Err(ContractError::SlinkyBTCPriceIncorrect { price: _, error: _ })
    ));

    // 4. error: BTC price is malformed
    let (_, exec_res) = publish_till_consensus(
        mock_dependencies(),
        &mut env,
        "not_a_number".to_string(),
        200,
    );
    // let res = query(deps.as_ref(), env.clone(), QueryMsg::GetAum {});
    assert!(matches!(
        exec_res,
        Err(ContractError::SlinkyBTCPriceIncorrect { .. })
    ));

    // 5. error: BTC price is too old (block_height + max_blocks_old < env.height)
    let (_, exec_res) =
        publish_till_consensus(mock_dependencies(), &mut env, "25000".to_string(), 50);
    // let res = query(deps.as_ref(), env.clone(), QueryMsg::GetAum {});
    assert!(matches!(
        exec_res,
        Err(ContractError::SlinkyBTCPriceTooOld { .. })
    ));

    // 6. success: valid price and block height
    let (deps, exec_res) =
        publish_till_consensus(mock_dependencies(), &mut env, "25000".to_string(), 150);
    assert!(exec_res.is_ok());
    let res = query(deps.as_ref(), env.clone(), QueryMsg::GetAum {});
    let bin = res.unwrap();
    let parsed: GetAumResponse = from_json(bin).unwrap();

    // expected: aum_usd = 500_000, strategy_jlp_balance = 10_000, total_jlp_supply = 1_000
    // virtual price = 500_000_000 / 1_000 = 500
    // jlp_balance_in_usd = 500_000_000 * 10_000 = 5_000_000_000_000
    // aum_in_btc = 5_000_000_000_000 / 25_000 = 200_000_000
    // scaled by 8 decimals (wbtc precision): 200_000_000_000_000_00
    assert_eq!(
        parsed.aum_in_wbtc,
        Int256::from(20_000_000_000_000_000_i128)
    );
}

#[test]
fn test_publish_data_invalid_custody() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api;
    let owner_info = message_info(&api.addr_make("owner"), &[]);
    instantiate(
        deps.as_mut(),
        env.clone(),
        owner_info,
        default_init_msg(&api),
    )
    .unwrap();

    let info = message_info(&api.addr_make("messenger1"), &[]);
    let mut data = dummy_solana_data();
    data.custody_assets.clear();

    let msg = ExecuteMsg::PublishData { new_data: data };
    let res = execute(deps.as_mut(), env, info, msg);
    assert!(matches!(
        res,
        Err(ContractError::ConsensusError(
            ConsensusError::PrepublishError { .. }
        ))
    ));
}

#[test]
fn test_execute_publish_data_up_to_date_consensus() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api;
    let owner_info = message_info(&api.addr_make("owner"), &[]);
    instantiate(
        deps.as_mut(),
        env.clone(),
        owner_info,
        default_init_msg(&api),
    )
    .unwrap();

    let consensus_config = CONSENSUS_STATE.config.load(&deps.storage).unwrap();
    let new_round_length = consensus_config.round_length + 1000;
    let new_consensus_config = ConsensusConfig {
        messengers: vec![
            api.addr_make("messenger1"),
            api.addr_make("messenger2"),
            api.addr_make("messenger3"),
        ],
        threshold: 2,
        data_delta_ppm: 10000,
        round_length: new_round_length,
    };

    // Save to pending_config
    CONSENSUS_STATE
        .pending_config
        .save(deps.as_mut().storage, &new_consensus_config)
        .unwrap();

    CONSENSUS_STATE
        .pending_round
        .save(
            deps.as_mut().storage,
            &Round {
                round: 1,
                start: 1000,
            },
        )
        .unwrap();

    let info = message_info(&api.addr_make("messenger1"), &[]);
    let data = dummy_solana_data();

    let msg = ExecuteMsg::PublishData { new_data: data };
    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert!(res.is_ok());

    let response = res.unwrap();

    let expected_next_round_ts = env.block.time.seconds() + new_round_length;
    let next_round_ts_attr = response
        .attributes
        .iter()
        .find(|attr| attr.key == "next_round_timestamp");
    assert_eq!(
        next_round_ts_attr.unwrap().value,
        expected_next_round_ts.to_string()
    );
}

#[test]
fn test_query_get_data_and_config() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner_info = message_info(&deps.api.addr_make("owner"), &[]);
    let init_msg = default_init_msg(&deps.api);
    instantiate(deps.as_mut(), env.clone(), owner_info, init_msg).unwrap();

    let res = query(deps.as_ref(), env.clone(), QueryMsg::GetConfig {}).unwrap();
    assert!(res.len() > 0);

    let res = query(deps.as_ref(), env.clone(), QueryMsg::GetData {}).unwrap();
    assert!(res.len() > 0);
}

#[test]
fn test_query_round_info() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner_info = message_info(&deps.api.addr_make("owner"), &[]);
    let init_msg = default_init_msg(&deps.api);

    instantiate(deps.as_mut(), env.clone(), owner_info, init_msg).unwrap();

    let res = query(deps.as_ref(), env, QueryMsg::GetRoundInfo {}).unwrap();
    assert!(res.len() > 0);
}

#[test]
fn test_query_get_aum_data_stale() {
    let mut env = mock_env();
    let start_time = 1000;
    env.block.time = Timestamp::from_seconds(start_time);
    let (deps, exec_res) = publish_till_consensus(
        mock_dependencies(),
        &mut env,
        "10000".to_string(),
        start_time,
    );
    assert!(exec_res.is_ok());

    env.block.time = env.block.time.plus_seconds(10_000);
    let res = query(deps.as_ref(), env, QueryMsg::GetAum {});
    assert!(matches!(res, Err(ContractError::PublishedDataTooOld {})));
}

fn dummy_solana_data() -> SolanaData {
    SolanaData {
        custody_assets: vec![CustodyAsset {
            owned: 1,
            locked: 0,
            guaranteed_usd: 1,
            decimals: 6,
            denom: "USDC".to_string(),
        }],
        aum_usd: Uint128::new(500_000),
        solana_balances: vec![SolanaBalance {
            address: "strategy".to_string(),
            asset: "jlp_token".to_string(),
            amount: Uint128::new(10_000),
        }],
        solana_token_total_supply: vec![SolanaTokenTotalSupply {
            asset: "jlp_token".to_string(),
            total_supply: Uint128::new(1_000),
        }],
        solana_token_decimals: vec![SolanaTokenDecimals {
            asset: "jlp_token".to_string(),
            decimals: 6,
        }],
    }
}

fn publish_till_consensus(
    mut deps: OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
    env: &mut Env,
    price: String,
    height: u64,
) -> (
    OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
    Result<Response, ContractError>,
) {
    let api = deps.api;
    let owner_info = message_info(&api.addr_make("owner"), &[]);
    let messenger1 = api.addr_make("messenger1");
    let messenger2 = api.addr_make("messenger2");
    let messenger3 = api.addr_make("messenger3");

    env.block.height = 200;
    env.block.time = Timestamp::from_seconds(1000);

    let mut msg = default_init_msg(&api);
    msg.consensus_data_valid_period = 1_000;
    msg.price_data_valid_period = 100;

    instantiate(deps.as_mut(), env.clone(), owner_info, msg).unwrap();

    deps.querier.with_price_and_height(price, height);

    let data = dummy_solana_data();
    env.block.time = env.block.time.plus_seconds(101); // next round
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&messenger1, &[]),
        ExecuteMsg::PublishData {
            new_data: data.clone(),
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&messenger2, &[]),
        ExecuteMsg::PublishData {
            new_data: data.clone(),
        },
    )
    .unwrap();
    let res = execute(
        deps.as_mut(),
        env.clone(),
        message_info(&messenger3, &[]),
        ExecuteMsg::PublishData {
            new_data: data.clone(),
        },
    );

    (deps, res)
}

#[test]
fn test_migrate() {
    use crate::contract::migrate;
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::Storage;
    use jupiter_aum_common::msg::MigrateMsg;
    use serde_json;

    let mut deps = cosmwasm_std::testing::mock_dependencies();
    let env = mock_env();

    // simulate pre-migration state
    #[cw_serde]
    struct OldConfig {
        consensus_data_valid_period: u64,
        required_custody_assets: Vec<String>,
        price_data_valid_period: u64,
    }
    let old_config = OldConfig {
        consensus_data_valid_period: 650,
        required_custody_assets: vec![
            "SOL".to_string(),
            "USDC".to_string(),
            "USDT".to_string(),
            "WBTC".to_string(),
            "WETH".to_string(),
        ],
        price_data_valid_period: 2,
    };
    let old_config_bytes = serde_json::to_vec(&old_config).unwrap();
    deps.storage.set(b"config", &old_config_bytes);

    // execute migration
    let migrate_msg = MigrateMsg {
        required_solana_balances: HashMap::from([(
            "strategy_address".to_string(),
            vec!["jlp_token".to_string()],
        )]),
        required_solana_token_total_supply: vec!["jlp_token".to_string()],
        strategy_address: "strategy_address".to_string(),
        jlp_token: "jlp_token".to_string(),
    };
    let result = migrate(deps.as_mut(), env, migrate_msg.clone());
    assert!(result.is_ok(), "Migration should succeed");

    // assert migration succeeds
    let new_config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(
        new_config.consensus_data_valid_period,
        old_config.consensus_data_valid_period
    );
    assert_eq!(
        new_config.required_custody_assets,
        old_config.required_custody_assets
    );
    assert_eq!(
        new_config.price_data_valid_period,
        old_config.price_data_valid_period
    );
    assert_eq!(
        new_config.required_solana_balances,
        HashMap::from([(
            "strategy_address".to_string(),
            vec!["jlp_token".to_string()],
        )])
    );
    assert_eq!(
        new_config.required_solana_token_total_supply,
        vec!["jlp_token".to_string()]
    );
    assert_eq!(new_config.strategy_address, "strategy_address".to_string());
    assert_eq!(new_config.jlp_token, "jlp_token".to_string());
}
