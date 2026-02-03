use crate::error::ContractError;
use crate::types::{
    Config, CustodyAsset, PriceTicker, SolanaBalance, SolanaData, SolanaTokenDecimals,
    SolanaTokenTotalSupply,
};
use consensus::consensus::ConsensusData;
use consensus::error::ConsensusError;
use cosmwasm_std::Uint128;
use std::collections::HashMap;

#[test]
fn no_data() {
    let result = SolanaData::try_consensus(&[], 3, 10_000);
    assert!(result.is_none());
}

#[test]
fn not_enough_data() {
    let data = vec![make_data(100, 100, 100, "usdc", 100)];
    let result = SolanaData::try_consensus(&data, 3, 10_000);
    assert!(result.is_none());
}

#[test]
fn exact_consensus() {
    let data = vec![
        make_data(100, 100, 100, "usdc", 100),
        make_data(100, 100, 100, "usdc", 100),
        make_data(100, 100, 100, "usdc", 100),
    ];
    let result = SolanaData::try_consensus(&data, 3, 10_000);
    assert!(result.is_some());
    let r = result.unwrap();
    assert_eq!(r.aum_usd, Uint128::new(100));
    assert_eq!(r.custody_assets[0].owned, 100);
}

#[test]
fn almost_exact_consensus() {
    let data = vec![
        make_data(100, 100, 100, "usdc", 100),
        make_data(100, 100, 100, "usdc", 100),
        make_data(101, 100, 100, "usdc", 100), // only aum_usd differs a bit
    ];
    let result = SolanaData::try_consensus(&data, 3, 20_000); // 2% tolerance
    assert!(result.is_some());
    assert_eq!(result.unwrap().aum_usd, Uint128::new(100)); // median
}

#[test]
fn no_consensus_due_to_small_variance() {
    let data = vec![
        make_data(100, 100, 100, "usdc", 100),
        make_data(100, 100, 100, "usdc", 100),
        make_data(103, 100, 100, "usdc", 100), // only aum_usd differs a bit
    ];
    let result = SolanaData::try_consensus(&data, 3, 20_000); // 2% tolerance
    assert!(result.is_none());
}

#[test]
fn partial_consensus() {
    let data = vec![
        make_data(100, 100, 100, "usdc", 100),
        make_data(100, 100, 100, "usdc", 100),
        make_data(103, 100, 100, "usdc", 100), // only aum_usd differs a bit
    ];
    let result = SolanaData::try_consensus(&data, 2, 20_000); // 2% tolerance
    assert!(result.is_some());
    assert_eq!(result.unwrap().aum_usd, Uint128::new(100)); // median
}

#[test]
fn no_consensus_due_to_variance() {
    let data = vec![
        make_data(100, 100, 100, "usdc", 100),
        make_data(300, 100, 100, "usdc", 100),
        make_data(500, 100, 100, "usdc", 100),
    ];
    let result = SolanaData::try_consensus(&data, 3, 10_000);
    assert!(result.is_none());
}
// #[test]
// fn custody_assets_mismatch_fails() {
//     let a = make_data(100, 100, 100, "usdc", 100);
//     let mut b = make_data(100, 100, 100, "usdt", 100);
//     let c = make_data(100, 100, 100, "usdc", 100);
//     // order matters so force align
//     b.custody_assets.sort_by(|a, b| a.denom.cmp(&b.denom));
//     let result = SolanaData::try_consensus(&[a, b, c], 3, 10_000);
//     assert!(result.is_none()); // denom mismatch fails exact consensus
// }

#[test]
fn custody_assets_length_mismatch_returns_none() {
    let d1 = make_data(100, 100, 100, "usdc", 100);
    let mut d2 = make_data(100, 100, 100, "usdc", 100);
    d2.custody_assets.push(sample_asset("usdt", 100));
    let result = SolanaData::try_consensus(&[d1, d2], 2, 10_000);
    assert!(result.is_none());
}

#[test]
fn test_all_equal_consensus_7_messengers() {
    let data = vec![base_data(); 7];
    assert!(SolanaData::try_consensus(&data, 5, 0).is_some());
}

#[test]
fn test_one_malicious_aum_still_consensus_7_messengers() {
    let mut data = vec![base_data(); 7];
    data[6].aum_usd = Uint128::new(50000);
    assert!(SolanaData::try_consensus(&data, 5, 0).is_some());
}

#[test]
fn test_jlp_decimals_exact_consensus() {
    let mut data = vec![base_data(); 7];
    set_jlp_decimals(&mut data[0], 5);
    set_jlp_decimals(&mut data[1], 4);
    assert!(SolanaData::try_consensus(&data, 6, 0).is_none());
    assert!(SolanaData::try_consensus(&data, 5, 0).is_some());
}

#[test]
fn test_strategy_jlp_balance_exact_consensus() {
    let mut data = vec![base_data(); 7];
    set_strategy_jlp_balance(&mut data[1], Uint128::new(5));
    set_strategy_jlp_balance(&mut data[2], Uint128::new(5));
    assert!(SolanaData::try_consensus(&data, 6, 0).is_none());
    assert!(SolanaData::try_consensus(&data, 5, 0).is_some());
}

#[test]
fn test_jlp_total_supply_exact_consensus() {
    let mut data = vec![base_data(); 7];
    set_jlp_total_supply(&mut data[1], Uint128::new(5));
    set_jlp_total_supply(&mut data[2], Uint128::new(5));
    assert!(SolanaData::try_consensus(&data, 6, 0).is_none());
    assert!(SolanaData::try_consensus(&data, 5, 0).is_some());
}

#[test]
fn test_all_exact_fields_match_min_threshold() {
    let mut data = vec![base_data(); 7];
    for item in data.iter_mut().take(2) {
        set_jlp_decimals(item, 5);
        set_strategy_jlp_balance(item, Uint128::new(5));
        set_jlp_total_supply(item, Uint128::new(5));
    }
    let result = SolanaData::try_consensus(&data, 5, 10000);
    assert!(result.is_some());
    let consensus = result.unwrap();
    assert_eq!(get_jlp_decimals(&consensus), 6);
    assert_eq!(get_strategy_jlp_balance(&consensus), Uint128::new(300));
    assert_eq!(get_jlp_total_supply(&consensus), Uint128::new(5000));
}

#[test]
fn test_custody_decimals_exact_consensus() {
    let mut data = vec![base_data(); 7];
    for item in data.iter_mut().take(3) {
        item.custody_assets[0].decimals = 5;
    }
    assert!(SolanaData::try_consensus(&data, 4, 0).is_some());
}

#[test]
fn test_custody_decimals_exact_fail() {
    let mut data = vec![base_data(); 7];
    for item in data.iter_mut().take(4) {
        item.custody_assets[1].decimals = 3;
    }
    assert!(SolanaData::try_consensus(&data, 5, 0).is_none());
}

#[test]
fn test_each_wrong_one_exact_field_gets_filtered_out() {
    let mut data = vec![base_data(); 7];
    set_jlp_decimals(&mut data[4], 3);
    data[4].aum_usd = Uint128::new(999999999);
    set_custody_decimals(&mut data[5], "USDC".to_string(), 4);
    data[5].aum_usd = Uint128::new(888888888);
    set_custody_decimals(&mut data[6], "SOL".to_string(), 5);
    data[6].aum_usd = Uint128::new(777777777);
    for (i, item) in data.iter_mut().enumerate().take(4) {
        item.aum_usd = Uint128::new(1000 + i as u128 * 2); // 1000, 1002, 1004, 1006
    }
    let result = SolanaData::try_consensus(&data, 4, 10000);
    assert!(result.is_some());
    let consensus = result.unwrap();
    let expected = base_data();
    assert_eq!(get_jlp_decimals(&consensus), get_jlp_decimals(&expected));
    assert_eq!(
        get_custody_decimals(&consensus, "USDC".to_string()),
        get_custody_decimals(&expected, "USDC".to_string())
    );
    assert_eq!(
        get_custody_decimals(&consensus, "SOL".to_string()),
        get_custody_decimals(&expected, "SOL".to_string())
    );
    let avg = (1000 + 1002 + 1004 + 1006) / 4;
    assert_eq!(consensus.aum_usd, Uint128::new(avg));
}

#[test]
fn test_exact_field_disagreement_edge_threshold() {
    let mut data = vec![base_data(); 7];
    for item in data.iter_mut().take(3) {
        item.custody_assets[1].decimals = 2;
    }
    for item in data.iter_mut().take(5).skip(3) {
        item.custody_assets[1].decimals = 3;
    }
    assert!(SolanaData::try_consensus(&data, 6, 10000).is_none());
}

#[test]
fn test_config_valid() {
    let config = Config {
        consensus_data_valid_period: 100,
        price_data_valid_period: 100,
        required_custody_assets: vec!["btc".to_string()],
        required_solana_balances: HashMap::from([(
            "strategy".to_string(),
            vec!["jlp_token".to_string()],
        )]),
        required_solana_token_total_supply: vec!["jlp_token".to_string()],
        solana_slinky_map: HashMap::from([("jlp_token".to_string(), PriceTicker::Jlp)]),
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_valid_nojlp_nototal_supply() {
    let config = Config {
        consensus_data_valid_period: 100,
        price_data_valid_period: 100,
        required_custody_assets: vec!["btc".to_string()],
        required_solana_balances: HashMap::from([(
            "strategy".to_string(),
            vec!["some_token".to_string()],
        )]),
        required_solana_token_total_supply: vec![],
        solana_slinky_map: HashMap::from([(
            "some_token".to_string(),
            PriceTicker::Slinky {
                asset: "SOME".to_string(),
            },
        )]),
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_invalid_consensus_period() {
    let config = Config {
        consensus_data_valid_period: 0,
        price_data_valid_period: 100,
        required_custody_assets: vec!["btc".to_string()],
        required_solana_balances: HashMap::from([(
            "strategy".to_string(),
            vec!["jlp_token".to_string()],
        )]),
        required_solana_token_total_supply: vec!["jlp_token".to_string()],
        solana_slinky_map: HashMap::from([("jlp_token".to_string(), PriceTicker::Jlp)]),
    };
    let result = config.validate();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ContractError::InvalidConsensusPeriod { .. }
    ));
}

#[test]
fn test_config_invalid_price_data_period() {
    let config = Config {
        consensus_data_valid_period: 100,
        price_data_valid_period: 0,
        required_custody_assets: vec!["btc".to_string()],
        required_solana_balances: HashMap::from([(
            "strategy".to_string(),
            vec!["jlp_token".to_string()],
        )]),
        required_solana_token_total_supply: vec!["jlp_token".to_string()],
        solana_slinky_map: HashMap::from([("jlp_token".to_string(), PriceTicker::Jlp)]),
    };
    let result = config.validate();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ContractError::InvalidPriceDataPeriod { .. }
    ));
}

#[test]
fn test_config_invalid_jlp_token_address() {
    let config = Config {
        consensus_data_valid_period: 100,
        price_data_valid_period: 100,
        required_custody_assets: vec!["btc".to_string()],
        required_solana_balances: HashMap::from([(
            "strategy".to_string(),
            vec!["jlp_token".to_string()],
        )]),
        required_solana_token_total_supply: vec![
            "jlp_token".to_string(),
            "invalid_jlp_token".to_string(),
        ],
        solana_slinky_map: HashMap::from([("invalid_jlp_token".to_string(), PriceTicker::Jlp)]),
    };
    let result = config.validate();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ContractError::AssetNotInSlinkyMap { .. }
    ));
}

#[test]
fn test_config_duplicate_custody_assets() {
    let config = Config {
        consensus_data_valid_period: 100,
        price_data_valid_period: 100,
        required_custody_assets: vec![
            "eth".to_string(),
            "btc".to_string(),
            "sol".to_string(),
            "btc".to_string(),
        ],
        required_solana_balances: HashMap::from([(
            "strategy".to_string(),
            vec!["jlp_token".to_string()],
        )]),
        required_solana_token_total_supply: vec!["jlp_token".to_string()],
        solana_slinky_map: HashMap::from([("jlp_token".to_string(), PriceTicker::Jlp)]),
    };
    let result = config.validate();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ContractError::DuplicateCustodyAsset {
            asset: "btc".to_string()
        }
    );
}

#[test]
fn test_config_duplicate_solana_balance_assets() {
    let config = Config {
        consensus_data_valid_period: 100,
        price_data_valid_period: 100,
        required_custody_assets: vec!["btc".to_string()],
        required_solana_balances: HashMap::from([
            (
                "some_address".to_string(),
                vec!["some_token".to_string(), "another_token".to_string()],
            ),
            (
                "strategy".to_string(),
                vec![
                    "some_token".to_string(),
                    "jlp_token".to_string(),
                    "another_token".to_string(),
                    "jlp_token".to_string(),
                ],
            ),
        ]),
        required_solana_token_total_supply: vec!["jlp_token".to_string()],
        solana_slinky_map: HashMap::from([
            (
                "some_token".to_string(),
                PriceTicker::Slinky {
                    asset: "SOME".to_string(),
                },
            ),
            ("jlp_token".to_string(), PriceTicker::Jlp),
            (
                "another_token".to_string(),
                PriceTicker::Slinky {
                    asset: "ANOTHER".to_string(),
                },
            ),
        ]),
    };
    let result = config.validate();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ContractError::DuplicateSolanaBalanceAsset {
            address: "strategy".to_string(),
            asset: "jlp_token".to_string()
        }
    );
}

#[test]
fn test_config_duplicate_solana_token_total_supply() {
    let config = Config {
        consensus_data_valid_period: 100,
        price_data_valid_period: 100,
        required_custody_assets: vec!["btc".to_string()],
        required_solana_balances: HashMap::from([(
            "strategy".to_string(),
            vec!["jlp_token".to_string()],
        )]),
        required_solana_token_total_supply: vec![
            "some_token".to_string(),
            "jlp_token".to_string(),
            "another_token".to_string(),
            "jlp_token".to_string(),
        ],
        solana_slinky_map: HashMap::from([
            (
                "some_token".to_string(),
                PriceTicker::Slinky {
                    asset: "SOME".to_string(),
                },
            ),
            ("jlp_token".to_string(), PriceTicker::Jlp),
            (
                "another_token".to_string(),
                PriceTicker::Slinky {
                    asset: "ANOTHER".to_string(),
                },
            ),
        ]),
    };
    let result = config.validate();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ContractError::DuplicateSolanaTokenTotalSupply {
            asset: "jlp_token".to_string()
        }
    );
}

#[test]
fn test_config_jlp_total_supply_not_tracked() {
    let config = Config {
        consensus_data_valid_period: 100,
        price_data_valid_period: 100,
        required_custody_assets: vec!["btc".to_string()],
        required_solana_balances: HashMap::from([(
            "strategy".to_string(),
            vec!["jlp_token".to_string()],
        )]),
        required_solana_token_total_supply: vec!["some_token".to_string()],
        solana_slinky_map: HashMap::from([
            (
                "some_token".to_string(),
                PriceTicker::Slinky {
                    asset: "SOME".to_string(),
                },
            ),
            ("jlp_token".to_string(), PriceTicker::Jlp),
        ]),
    };
    let result = config.validate();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ContractError::JlpTotalSupplyNotTracked { .. }
    ));
}

#[test]
fn test_config_asset_not_in_slinky_map() {
    let config = Config {
        consensus_data_valid_period: 100,
        price_data_valid_period: 100,
        required_custody_assets: vec!["btc".to_string()],
        required_solana_balances: HashMap::from([
            ("strategy".to_string(), vec!["jlp_token".to_string()]),
            ("some_address".to_string(), vec!["some_token".to_string()]),
        ]),
        required_solana_token_total_supply: vec!["jlp_token".to_string()],
        solana_slinky_map: HashMap::from([("jlp_token".to_string(), PriceTicker::Jlp)]),
    };
    let result = config.validate();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ContractError::AssetNotInSlinkyMap {
            asset: "some_token".to_string()
        }
    );
}

#[test]
fn test_prepublish_cleanup_ok() {
    let config = sample_config();
    let mut data = sample_data();
    let result = data.prepublish_cleanup(config.clone());
    assert!(result.is_ok());
}

#[test]
fn test_prepublish_cleanup_missing_custody_assets() {
    let config = sample_config();
    let mut data = sample_data();
    data.custody_assets.remove(0);
    let result = data.prepublish_cleanup(config.clone());
    assert_eq!(
        result.unwrap_err(),
        ConsensusError::PrepublishError {
            msg: "Provided solana custody assets don't match required ones".into()
        }
    );
}

#[test]
fn test_prepublish_cleanup_missing_solana_balances() {
    let config = sample_config();
    let mut data = sample_data();
    data.solana_balances.remove(0);
    let result = data.prepublish_cleanup(config.clone());
    assert_eq!(
        result.unwrap_err(),
        ConsensusError::PrepublishError {
            msg: "Provided solana balances don't match required ones".into()
        }
    );
}

#[test]
fn test_prepublish_cleanup_missing_solana_token_total_supply() {
    let config = sample_config();
    let mut data = sample_data();
    data.solana_token_total_supply.clear();
    let result = data.prepublish_cleanup(config.clone());
    assert_eq!(
        result.unwrap_err(),
        ConsensusError::PrepublishError {
            msg: "Provided solana token total supplies don't match required ones".into()
        }
    );
}

#[test]
fn test_prepublish_cleanup_missing_solana_token_decimals() {
    let config = sample_config();
    let mut data = sample_data();
    data.solana_token_decimals.remove(0);
    let result = data.prepublish_cleanup(config.clone());
    assert_eq!(
        result.unwrap_err(),
        ConsensusError::PrepublishError {
            msg: "Provided solana token decimals don't match required ones".into()
        }
    );
}

#[test]
fn test_prepublish_cleanup_dedup_superfluous_custody_assets() {
    let config = sample_config();
    let mut data = sample_data();
    data.custody_assets.push(sample_asset("btc", 300));
    data.custody_assets.push(sample_asset("eth", 400));
    let result = data.prepublish_cleanup(config.clone());
    assert!(result.is_ok());
    assert_eq!(data.custody_assets.len(), 2);
    assert!(data.custody_assets.iter().any(|a| a.denom == "sol"));
    assert!(data.custody_assets.iter().any(|a| a.denom == "usdc"));
}

#[test]
fn test_prepublish_cleanup_dedup_superfluous_solana_balances() {
    let config = sample_config();
    let mut data = sample_data();
    data.solana_balances.push(SolanaBalance {
        address: "extra_address".to_string(),
        asset: "extra_token".to_string(),
        amount: Uint128::new(999),
    });
    data.solana_balances.push(SolanaBalance {
        address: "strategy".to_string(),
        asset: "unwanted_token".to_string(),
        amount: Uint128::new(888),
    });
    let result = data.prepublish_cleanup(config.clone());
    assert!(result.is_ok());
    assert_eq!(data.solana_balances.len(), 2);
    assert!(data.solana_balances[0].address == "some_address");
    assert!(data.solana_balances[0].asset == "some_token");
    assert!(data.solana_balances[1].address == "strategy");
    assert!(data.solana_balances[1].asset == "jlp_token");
}

#[test]
fn test_prepublish_cleanup_dedup_superfluous_solana_token_total_supply() {
    let config = sample_config();
    let mut data = sample_data();
    data.solana_token_total_supply.push(SolanaTokenTotalSupply {
        asset: "extra_supply_token".to_string(),
        total_supply: Uint128::new(50000),
    });
    data.solana_token_total_supply.push(SolanaTokenTotalSupply {
        asset: "another_extra_token".to_string(),
        total_supply: Uint128::new(60000),
    });
    let result = data.prepublish_cleanup(config.clone());
    assert!(result.is_ok());
    assert_eq!(data.solana_token_total_supply.len(), 1);
    assert_eq!(data.solana_token_total_supply[0].asset, "jlp_token");
}

#[test]
fn test_prepublish_cleanup_dedup_superfluous_solana_token_decimals() {
    let config = sample_config();
    let mut data = sample_data();
    data.solana_token_decimals.push(SolanaTokenDecimals {
        asset: "extra_decimal_token".to_string(),
        decimals: 8,
    });
    data.solana_token_decimals.push(SolanaTokenDecimals {
        asset: "another_extra_token".to_string(),
        decimals: 12,
    });
    let result = data.prepublish_cleanup(config.clone());
    assert!(result.is_ok());
    assert_eq!(data.solana_token_decimals.len(), 2);
    #[rustfmt::skip]
    assert_eq!(data.solana_token_decimals[0].asset, "jlp_token");
    assert_eq!(data.solana_token_decimals[0].decimals, 6);
    #[rustfmt::skip]
    assert_eq!(data.solana_token_decimals[1].asset, "some_token");
    assert_eq!(data.solana_token_decimals[1].decimals, 9);
}

#[test]
fn test_prepublish_cleanup_duplicates_extra_mixed() {
    let config = sample_config();
    #[rustfmt::skip]
    let mut data = SolanaData {
        aum_usd: Uint128::new(1000),
        custody_assets: vec![
            sample_asset("eth", 500),
            sample_asset("usdc", 200),
            sample_asset("btc", 300),
            sample_asset("sol", 100),
            sample_asset("usdc", 250),
        ],
        solana_balances: vec![
            SolanaBalance { address: "unwanted".to_string(), asset: "unwanted".to_string(), amount: Uint128::new(777) },
            SolanaBalance { address: "strategy".to_string(), asset: "jlp_token".to_string(), amount: Uint128::new(1000) },
            SolanaBalance { address: "some_address".to_string(), asset: "some_token".to_string(), amount: Uint128::new(2000) },
        ],
        solana_token_total_supply: vec![
            SolanaTokenTotalSupply { asset: "extra".to_string(), total_supply: Uint128::new(99999) },
            SolanaTokenTotalSupply { asset: "jlp_token".to_string(), total_supply: Uint128::new(10000) },
        ],
        solana_token_decimals: vec![
            SolanaTokenDecimals { asset: "extra".to_string(), decimals: 18 },
            SolanaTokenDecimals { asset: "jlp_token".to_string(), decimals: 6 },
            SolanaTokenDecimals { asset: "some_token".to_string(), decimals: 9 },
        ],
    };
    let result = data.prepublish_cleanup(config.clone());
    assert!(result.is_ok());
    assert_eq!(data.custody_assets.len(), 2);
    assert_eq!(data.solana_balances.len(), 2);
    assert_eq!(data.solana_token_total_supply.len(), 1);
    assert_eq!(data.solana_token_decimals.len(), 2);
}

fn sample_asset(denom: &str, val: u64) -> CustodyAsset {
    CustodyAsset {
        owned: val,
        locked: val,
        guaranteed_usd: val,
        decimals: 6,
        denom: denom.to_string(),
    }
}

fn sample_config() -> Config {
    let config = Config {
        consensus_data_valid_period: 100,
        price_data_valid_period: 100,
        required_custody_assets: vec!["sol".to_string(), "usdc".to_string()],
        required_solana_balances: HashMap::from([
            ("strategy".to_string(), vec!["jlp_token".to_string()]),
            ("some_address".to_string(), vec!["some_token".to_string()]),
        ]),
        required_solana_token_total_supply: vec!["jlp_token".to_string()],
        solana_slinky_map: HashMap::from([
            (
                "some_token".to_string(),
                PriceTicker::Slinky {
                    asset: "SOME".to_string(),
                },
            ),
            ("jlp_token".to_string(), PriceTicker::Jlp),
        ]),
    };
    config.validate().unwrap();
    config
}

/// Creates a sample SolanaData that perfectly matches the sample_config() result.
#[rustfmt::skip]
fn sample_data() -> SolanaData {
    SolanaData {
        aum_usd: Uint128::new(1000),
        custody_assets: vec![sample_asset("usdc", 200), sample_asset("sol", 100)],
        solana_balances: vec![
            SolanaBalance { address: "strategy".to_string(), asset: "jlp_token".to_string(), amount: Uint128::new(1000) },
            SolanaBalance { address: "some_address".to_string(), asset: "some_token".to_string(), amount: Uint128::new(2000) },
        ],
        solana_token_total_supply: vec![SolanaTokenTotalSupply {
            asset: "jlp_token".to_string(),
            total_supply: Uint128::new(10000),
        }],
        solana_token_decimals: vec![
            SolanaTokenDecimals { asset: "jlp_token".to_string(), decimals: 6 },
            SolanaTokenDecimals { asset: "some_token".to_string(), decimals: 9 },
        ],
    }
}

fn make_data(aum: u128, jlp: u128, strat: u128, denom: &str, val: u64) -> SolanaData {
    SolanaData {
        custody_assets: vec![sample_asset(denom, val)],
        aum_usd: Uint128::new(aum),
        solana_balances: vec![SolanaBalance {
            address: "strategy".to_string(),
            asset: "jlp".to_string(),
            amount: Uint128::new(strat),
        }],
        solana_token_total_supply: vec![SolanaTokenTotalSupply {
            asset: "jlp".to_string(),
            total_supply: Uint128::new(jlp),
        }],
        solana_token_decimals: vec![SolanaTokenDecimals {
            asset: "jlp".to_string(),
            decimals: 6,
        }],
    }
}

fn base_asset_set() -> Vec<CustodyAsset> {
    vec![
        CustodyAsset {
            owned: 100,
            locked: 50,
            guaranteed_usd: 1000,
            decimals: 6,
            denom: "USDC".to_string(),
        },
        CustodyAsset {
            owned: 200,
            locked: 75,
            guaranteed_usd: 2000,
            decimals: 6,
            denom: "SOL".to_string(),
        },
    ]
}

fn base_data() -> SolanaData {
    SolanaData {
        aum_usd: Uint128::new(1000),
        custody_assets: base_asset_set(),
        solana_balances: vec![SolanaBalance {
            address: "strategy".to_string(),
            asset: "jlp".to_string(),
            amount: Uint128::new(300),
        }],
        solana_token_total_supply: vec![SolanaTokenTotalSupply {
            asset: "jlp".to_string(),
            total_supply: Uint128::new(5000),
        }],
        solana_token_decimals: vec![SolanaTokenDecimals {
            asset: "jlp".to_string(),
            decimals: 6,
        }],
    }
}

fn get_custody_decimals(data: &SolanaData, denom: String) -> u8 {
    data.custody_assets
        .iter()
        .find(|c| c.denom == denom)
        .unwrap()
        .decimals
}

fn set_custody_decimals(data: &mut SolanaData, denom: String, decimals: u8) {
    data.custody_assets
        .iter_mut()
        .find(|c| c.denom == denom)
        .unwrap()
        .decimals = decimals;
}

fn get_jlp_total_supply(data: &SolanaData) -> Uint128 {
    data.solana_token_total_supply
        .iter()
        .find(|t| t.asset == *"jlp")
        .unwrap()
        .total_supply
}

fn set_jlp_total_supply(data: &mut SolanaData, total_supply: Uint128) {
    data.solana_token_total_supply
        .iter_mut()
        .find(|t| t.asset == *"jlp")
        .unwrap()
        .total_supply = total_supply;
}

fn get_jlp_decimals(data: &SolanaData) -> u8 {
    data.solana_token_decimals
        .iter()
        .find(|d| d.asset == *"jlp")
        .unwrap()
        .decimals
}

fn set_jlp_decimals(data: &mut SolanaData, decimals: u8) {
    data.solana_token_decimals
        .iter_mut()
        .find(|d| d.asset == *"jlp")
        .unwrap()
        .decimals = decimals;
}

fn get_strategy_jlp_balance(data: &SolanaData) -> Uint128 {
    data.solana_balances
        .iter()
        .find(|b| b.asset == *"jlp")
        .unwrap()
        .amount
}

fn set_strategy_jlp_balance(data: &mut SolanaData, balance: Uint128) {
    data.solana_balances
        .iter_mut()
        .find(|b| b.asset == *"jlp")
        .unwrap()
        .amount = balance;
}
