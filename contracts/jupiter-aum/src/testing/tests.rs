use crate::contract::{calculate_aum_in_btc, execute, instantiate, query};
use crate::state::{CONFIG, CONSENSUS_STATE};
use crate::testing::mock_querier::mock_dependencies;
use consensus::consensus::OracleData;
use consensus::error::ConsensusError;
use cosmwasm_std::testing::{message_info, mock_env, MockApi};
use cosmwasm_std::{Decimal, Timestamp, Uint128};
use jupiter_aum_common::error::ContractError;
use jupiter_aum_common::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
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
        data_delta_ppm: 1000,
        round_length: 100,
        valid_period: 1_000,
        required_custody_assets: vec!["USDC".to_string()],
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
        valid_period: Some(50_000),
        required_custody_assets: Some(vec!["BTC".to_string()]),
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

    // Authorized update
    let authorized_res = execute(deps.as_mut(), env.clone(), admin_info.clone(), update_msg);
    assert!(authorized_res.is_ok());

    // Config should have updated values
    let config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(config.admin, deps.api.addr_make("admin2"));
    assert_eq!(config.valid_period, 50_000);
    assert_eq!(config.required_custody_assets, vec!["BTC".to_string()]);
}

#[test]
fn test_calculate_aum_in_btc() {
    // Test case 1: Standard calculation
    let data1 = SolanaData {
        custody_assets: vec![],
        aum_usd: Uint128::new(500_000_000_000u128), // $500,000 AUM USD
        jlp_token_decimals: 6,
        total_jlp_supply: Uint128::new(1_000_000_000), // 1,000 JLP total supply
        strategy_jlp_balance: Uint128::new(10_000_000_000u128), // 10,000 JLP balance
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
        custody_assets: vec![],
        aum_usd: Uint128::new(1_000_000_000_000_000u128), // $1 Billion AUM
        jlp_token_decimals: 6,
        total_jlp_supply: Uint128::new(50_000_000_000u128), // 50,000 JLP total
        strategy_jlp_balance: Uint128::new(20_000_000_000u128), // 20,000 JLP balance
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
        custody_assets: vec![],
        aum_usd: Uint128::new(100),
        jlp_token_decimals: 6,
        total_jlp_supply: Uint128::zero(), // Zero supply
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
        custody_assets: vec![],
        aum_usd: Uint128::new(100),
        jlp_token_decimals: 6,
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

    // TODO: different decimal value for test
}

// TODO: this should be fixed as values will change and there is no slot publishing system anymore
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
    let solana_data = SolanaData {
        custody_assets: vec![CustodyAsset {
            owned: 1,
            locked: 0,
            guaranteed_usd: 1,
            decimals: 6,
            denom: "USDC".to_string(),
        }],
        aum_usd: Uint128::new(500_000),
        jlp_token_decimals: 6,
        total_jlp_supply: Uint128::new(1_000),
        strategy_jlp_balance: Uint128::new(10_000),
    };
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&oracle1, &[]),
        publish_msg_from_solana_data(&solana_data),
    )
    .unwrap();
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&oracle2, &[]),
        publish_msg_from_solana_data(&solana_data),
    )
    .unwrap();
    env.block.time = env.block.time.plus_seconds(101); // Advance time past valid_period (1000s)
                                                       // asserts data is there
    let data = CONSENSUS_STATE
        .get_last_published_data(&env, &deps.storage)
        .unwrap();
    assert!(data.is_some());
}

#[test]
fn test_publish_data_unauthorized() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let admin_info = message_info(&deps.api.addr_make("admin"), &[]);
    let init_msg = default_init_msg(&deps.api);
    instantiate(deps.as_mut(), env.clone(), admin_info, init_msg).unwrap();

    let data = dummy_oracle_data();
    let info = message_info(&deps.api.addr_make("hacker"), &[]);
    let msg = ExecuteMsg::PublishData { data };
    let res = execute(deps.as_mut(), env, info, msg);
    assert!(matches!(res, Err(ContractError::Unauthorized {})));
}

#[test]
fn test_publish_data_duplicate_oracle() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();
    let admin_info = message_info(&api.addr_make("admin"), &[]);
    instantiate(
        deps.as_mut(),
        env.clone(),
        admin_info,
        default_init_msg(&api),
    )
    .unwrap();

    let info = message_info(&api.addr_make("oracle1"), &[]);
    let data = dummy_oracle_data();
    let msg = ExecuteMsg::PublishData { data: data.clone() };

    execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert!(matches!(
        res,
        Err(ContractError::ConsensusError(
            ConsensusError::DoubleSubmission {}
        ))
    ));
}

#[test]
fn test_publish_data_invalid_custody() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let api = deps.api.clone();
    let admin_info = message_info(&api.addr_make("admin"), &[]);
    instantiate(
        deps.as_mut(),
        env.clone(),
        admin_info,
        default_init_msg(&api),
    )
    .unwrap();

    let info = message_info(&api.addr_make("oracle1"), &[]);
    let mut data = dummy_oracle_data();
    data.data.custody_assets.clear();

    let msg = ExecuteMsg::PublishData { data };
    let res = execute(deps.as_mut(), env, info, msg);
    assert!(matches!(res, Err(ContractError::Std(_))));
}

#[test]
fn test_query_get_data_and_config() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let admin_info = message_info(&deps.api.addr_make("admin"), &[]);
    let init_msg = default_init_msg(&deps.api);
    instantiate(deps.as_mut(), env.clone(), admin_info, init_msg).unwrap();

    let res = query(deps.as_ref(), env.clone(), QueryMsg::Config {}).unwrap();
    assert!(res.len() > 0);

    let res = query(deps.as_ref(), env.clone(), QueryMsg::GetData {}).unwrap();
    assert!(res.len() > 0);
}

#[test]
fn test_query_round_info() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let admin_info = message_info(&deps.api.addr_make("admin"), &[]);
    let init_msg = default_init_msg(&deps.api);

    instantiate(deps.as_mut(), env.clone(), admin_info, init_msg).unwrap();

    let res = query(deps.as_ref(), env, QueryMsg::GetRoundInfo {}).unwrap();
    assert!(res.len() > 0);
}

#[test]
fn test_query_get_aum_data_stale() {
    let mut deps = mock_dependencies();
    let mut env = mock_env();
    let start_time = 1000;
    env.block.time = Timestamp::from_seconds(start_time);

    let admin_info = message_info(&deps.api.addr_make("admin"), &[]);
    let oracle1 = deps.api.addr_make("oracle1");
    let oracle2 = deps.api.addr_make("oracle2");
    let init_msg = default_init_msg(&deps.api);

    instantiate(deps.as_mut(), env.clone(), admin_info, init_msg).unwrap();

    let data = dummy_oracle_data();
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&oracle1, &[]),
        ExecuteMsg::PublishData { data: data.clone() },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&oracle2, &[]),
        ExecuteMsg::PublishData { data: data.clone() },
    )
    .unwrap();

    env.block.time = env.block.time.plus_seconds(10_000);
    let res = query(deps.as_ref(), env, QueryMsg::GetAUM {});
    assert!(matches!(res, Err(ContractError::DataNotValid {})));
}

fn dummy_oracle_data() -> OracleData<SolanaData> {
    OracleData {
        round: 0,
        timestamp: 1000,
        data: SolanaData {
            custody_assets: vec![CustodyAsset {
                owned: 1,
                locked: 0,
                guaranteed_usd: 1,
                decimals: 6,
                denom: "USDC".to_string(),
            }],
            aum_usd: Uint128::new(500_000),
            jlp_token_decimals: 6,
            total_jlp_supply: Uint128::new(1_000),
            strategy_jlp_balance: Uint128::new(10_000),
        },
    }
}

fn publish_msg_from_solana_data(data: &SolanaData) -> ExecuteMsg {
    ExecuteMsg::PublishData {
        data: OracleData {
            round: 0,
            timestamp: 0,
            data: SolanaData {
                custody_assets: data.custody_assets.clone(),
                aum_usd: data.aum_usd,
                jlp_token_decimals: data.jlp_token_decimals,
                total_jlp_supply: data.total_jlp_supply,
                strategy_jlp_balance: data.strategy_jlp_balance,
            },
        },
    }
}
