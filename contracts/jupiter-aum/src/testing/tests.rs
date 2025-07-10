use crate::contract::{calculate_aum_in_btc, execute, instantiate, query};
use crate::state::{CONFIG, CONSENSUS_STATE};
use crate::testing::mock_querier::mock_dependencies;
use cosmwasm_std::testing::{message_info, mock_env, MockApi};
use cosmwasm_std::{from_json, Int256, SignedDecimal256, Timestamp, Uint128};
use jupiter_aum_common::error::ContractError;
use jupiter_aum_common::msg;
use jupiter_aum_common::msg::ExecuteMsg::UpdateConfig;
use jupiter_aum_common::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use jupiter_aum_common::types::{CustodyAsset, SolanaData};
use std::str::FromStr;

// Helper to create a default instantiate message
fn default_init_msg(api: &MockApi) -> InstantiateMsg {
    InstantiateMsg {
        owner: api.addr_make("owner").to_string(),
        oracles: vec![
            api.addr_make("oracle1").to_string(),
            api.addr_make("oracle2").to_string(),
            api.addr_make("oracle3").to_string(),
        ],
        threshold: 2,
        data_delta_ppm: 1000,
        round_length: 100,
        consensus_data_validity_period: 1_000,
        required_custody_assets: vec!["USDC".to_string()],
        price_data_validity_period: 100,
    }
}

#[test]
fn test_update_config() {
    let mut deps = mock_dependencies();

    let env = mock_env();
    let owner_info = message_info(&deps.api.addr_make("owner"), &[]);
    let msg = default_init_msg(&deps.api);
    instantiate(deps.as_mut(), env.clone(), owner_info.clone(), msg).unwrap();

    let config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(config.owner, owner_info.sender);

    let update = msg::UpdateConfig {
        owner: Some(deps.api.addr_make("owner2").to_string()),
        consensus_data_validity_period: Some(50_000),
        required_custody_assets: Some(vec!["BTC".to_string()]),
        price_data_validity_period: Some(999),
        oracles: Some(vec![deps.api.addr_make("oracle1").to_string()]),
        threshold: Some(1),
        data_delta_ppm: Some(1234),
        round_length: Some(99),
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
    assert_eq!(unauthorized.unwrap_err(), ContractError::Unauthorized {});

    // Authorized
    let authorized = execute(deps.as_mut(), env.clone(), owner_info.clone(), update_msg);
    assert!(authorized.is_ok());

    // Assert config updated
    let config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(config.owner, deps.api.addr_make("owner2"));
    assert_eq!(config.consensus_data_validity_period, 50_000);
    assert_eq!(config.required_custody_assets, vec!["BTC"]);
    assert_eq!(config.price_data_validity_period, 999);

    let consensus = CONSENSUS_STATE.config.load(&deps.storage).unwrap();
    assert_eq!(consensus.threshold, 1);
    assert_eq!(consensus.data_delta_ppm, 1234);
    assert_eq!(consensus.round_length, 99);
    assert_eq!(consensus.oracles, vec![deps.api.addr_make("oracle1")]);
}

#[test]
fn test_calculate_aum_in_btc() {
    // Test case 1: Standard calculation
    let data1 = SolanaData {
        custody_assets: vec![],
        aum_usd: Uint128::new(500_000), // $500,000 AUM USD
        total_jlp_supply: Uint128::new(1_000_000_000), // 1,000 JLP total supply
        strategy_jlp_balance: Uint128::new(10_000_000_000u128), // 10,000 JLP balance
        total_jlp_supply_decimals: 6,
        strategy_jlp_balance_decimals: 6,
    };
    let btc_price_in_usd1 = SignedDecimal256::from_str("25000.0").unwrap(); // $25,000 per BTC
                                                                            // jlp_virtual_price = 500,000 / 1,000 = 500 USD/JLP
                                                                            // jlp_balance_in_usd = 500 * 10,000 = 5,000,000 USD
                                                                            // aum_in_btc = 5,000,000 / 25,000 = 200 BTC
    let res1 = calculate_aum_in_btc(data1, btc_price_in_usd1);
    assert_eq!(
        res1.unwrap(),
        Int256::from(20_000_000_000_i128),
        "Test Case 1 Failed"
    );

    // Test case 2: Different values
    let data2 = SolanaData {
        custody_assets: vec![],
        aum_usd: Uint128::new(1_000_000_000u128), // $1 Billion AUM
        total_jlp_supply: Uint128::new(50_000_000_000u128), // 50,000 JLP total
        strategy_jlp_balance: Uint128::new(20_000_000_000u128), // 20,000 JLP balance
        total_jlp_supply_decimals: 6,
        strategy_jlp_balance_decimals: 6,
    };
    let btc_price_in_usd2 = SignedDecimal256::from_str("50000.0").unwrap(); // $50,000 per BTC
                                                                            // jlp_virtual_price = 1,000,000,000 / 50,000 = 20,000 USD/JLP
                                                                            // jlp_balance_in_usd = 20,000 * 20,000 = 400,000,000 USD
                                                                            // aum_in_btc = 400,000,000 / 50,000 = 8,000 BTC
    let res2 = calculate_aum_in_btc(data2, btc_price_in_usd2);
    assert_eq!(
        res2.unwrap(),
        Int256::from(800_000_000_000_i128),
        "Test Case 2 Failed"
    );

    // Test case 3: Division by zero for total_jlp_supply
    let data3 = SolanaData {
        custody_assets: vec![],
        aum_usd: Uint128::new(100),
        total_jlp_supply: Uint128::zero(), // Zero supply
        strategy_jlp_balance: Uint128::new(10),
        total_jlp_supply_decimals: 6,
        strategy_jlp_balance_decimals: 6,
    };
    let btc_price_in_usd3 = SignedDecimal256::from_str("1.0").unwrap();
    let err3 = calculate_aum_in_btc(data3, btc_price_in_usd3).unwrap_err();
    assert!(
        matches!(&err3, ContractError::CheckedDiv(_)),
        "Test Case 3 Failed: {:?}",
        err3
    );

    // Test case 4: Division by zero for btc_price_in_usd
    let data4 = SolanaData {
        custody_assets: vec![],
        aum_usd: Uint128::new(100),
        total_jlp_supply: Uint128::new(10),
        strategy_jlp_balance: Uint128::new(5),
        total_jlp_supply_decimals: 6,
        strategy_jlp_balance_decimals: 6,
    };
    let btc_price_in_usd4 = SignedDecimal256::from_str("0.0").unwrap(); // Zero BTC price
    let err4 = calculate_aum_in_btc(data4, btc_price_in_usd4).unwrap_err();
    assert!(
        matches!(&err4, ContractError::CheckedDiv(_)),
        "Test Case 4 Failed: {:?}",
        err4
    );
}

/// Comprehensive test suite for the `query_get_aum` query message.
#[test]
fn test_query_get_aum_behavior() {
    let mut deps = mock_dependencies();
    let mut env = mock_env();

    let api = deps.api;
    let owner_info = message_info(&api.addr_make("owner"), &[]);
    let oracle1 = api.addr_make("oracle1");
    let oracle2 = api.addr_make("oracle2");

    let mut msg = default_init_msg(&api);
    msg.consensus_data_validity_period = 1_000;
    msg.price_data_validity_period = 100;

    instantiate(deps.as_mut(), env.clone(), owner_info, msg).unwrap();

    // 1. error: no data published yet
    let res = query(deps.as_ref(), env.clone(), QueryMsg::GetAum {});
    assert!(matches!(res, Err(ContractError::NoDataPublished {})));

    // publish valid data
    let data = dummy_solana_data();
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&oracle1, &[]),
        ExecuteMsg::PublishData {
            new_data: data.clone(),
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&oracle2, &[]),
        ExecuteMsg::PublishData {
            new_data: data.clone(),
        },
    )
    .unwrap();

    // 2. error: data published, but too old (time-based expiration)
    env.block.time = env.block.time.plus_seconds(10_000);
    let res = query(deps.as_ref(), env.clone(), QueryMsg::GetAum {});
    assert!(matches!(res, Err(ContractError::PublishedDataTooOld {})));

    // reset time
    env.block.time = env.block.time.minus_seconds(9_900);
    env.block.height = 200;

    // 3. error: no BTC price returned from oracle
    deps.querier.with_price_and_height("", 200); // empty string will trigger missing
    let res = query(deps.as_ref(), env.clone(), QueryMsg::GetAum {});
    assert!(matches!(
        res,
        Err(ContractError::SlinkyBTCPriceIncorrect { price: _, error: _ })
    ));

    // 4. error: BTC price is malformed
    deps.querier.with_price_and_height("not_a_number", 200);
    let res = query(deps.as_ref(), env.clone(), QueryMsg::GetAum {});
    assert!(matches!(
        res,
        Err(ContractError::SlinkyBTCPriceIncorrect { .. })
    ));

    // 5. error: BTC price is too old (block_height + max_blocks_old < env.height)
    deps.querier.with_price_and_height("25000", 50); // env.height is 200
    let res = query(deps.as_ref(), env.clone(), QueryMsg::GetAum {});
    assert!(matches!(
        res,
        Err(ContractError::SlinkyBTCPriceTooOld { .. })
    ));

    // 6. success: valid price and block height
    deps.querier.with_price_and_height("25000", 150); // still fresh: 150 + 100 > 200
    let res = query(deps.as_ref(), env.clone(), QueryMsg::GetAum {});
    let bin = res.unwrap();
    let parsed: msg::AumResponse = from_json(bin).unwrap();

    // expected: aum_usd = 500_000, strategy_jlp_balance = 10_000, total_jlp_supply = 1_000
    // virtual price = 500_000_000 / 1_000 = 500
    // jlp_balance_in_usd = 500_000_000 * 10_000 = 5_000_000_000_000
    // aum_in_btc = 5_000_000_000_000 / 25_000 = 200_000_000
    // scaled by 8 decimals (wbtc precision): 200_000_000_000_000_00
    assert_eq!(parsed.aum_in_btc, Int256::from(20_000_000_000_000_000_i128));
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

    let info = message_info(&api.addr_make("oracle1"), &[]);
    let mut data = dummy_solana_data();
    data.custody_assets.clear();

    let msg = ExecuteMsg::PublishData { new_data: data };
    let res = execute(deps.as_mut(), env, info, msg);
    assert!(matches!(res, Err(ContractError::Std(_))));
}

#[test]
fn test_query_get_data_and_config() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner_info = message_info(&deps.api.addr_make("owner"), &[]);
    let init_msg = default_init_msg(&deps.api);
    instantiate(deps.as_mut(), env.clone(), owner_info, init_msg).unwrap();

    let res = query(deps.as_ref(), env.clone(), QueryMsg::Config {}).unwrap();
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
    let mut deps = mock_dependencies();
    let mut env = mock_env();
    let start_time = 1000;
    env.block.time = Timestamp::from_seconds(start_time);

    let owner_info = message_info(&deps.api.addr_make("owner"), &[]);
    let oracle1 = deps.api.addr_make("oracle1");
    let oracle2 = deps.api.addr_make("oracle2");
    let init_msg = default_init_msg(&deps.api);

    instantiate(deps.as_mut(), env.clone(), owner_info, init_msg).unwrap();

    let data = dummy_solana_data();
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&oracle1, &[]),
        ExecuteMsg::PublishData {
            new_data: data.clone(),
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&oracle2, &[]),
        ExecuteMsg::PublishData {
            new_data: data.clone(),
        },
    )
    .unwrap();

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
        total_jlp_supply: Uint128::new(1_000),
        strategy_jlp_balance: Uint128::new(10_000),
        total_jlp_supply_decimals: 6,
        strategy_jlp_balance_decimals: 6,
    }
}
