use crate::types::{CustodyAsset, SolanaData};
use consensus::consensus::ConsensusData;
use cosmwasm_std::Uint128;

fn sample_asset(denom: &str, val: u64) -> CustodyAsset {
    CustodyAsset {
        owned: val,
        locked: val,
        guaranteed_usd: val,
        decimals: 6,
        denom: denom.to_string(),
    }
}

fn make_data(aum: u128, jlp: u128, strat: u128, denom: &str, val: u64) -> SolanaData {
    SolanaData {
        custody_assets: vec![sample_asset(denom, val)],
        aum_usd: Uint128::new(aum),
        total_jlp_supply: Uint128::new(jlp),
        strategy_jlp_balance: Uint128::new(strat),
        total_jlp_supply_decimals: 6,
        strategy_jlp_balance_decimals: 6,
    }
}

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
        total_jlp_supply: Uint128::new(5000),
        total_jlp_supply_decimals: 6,
        strategy_jlp_balance: Uint128::new(300),
        strategy_jlp_balance_decimals: 6,
        custody_assets: base_asset_set(),
    }
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
fn test_total_jlp_supply_decimals_exact_consensus_fail() {
    let mut data = vec![base_data(); 7];
    data[0].total_jlp_supply_decimals = 5;
    data[1].total_jlp_supply_decimals = 4;
    assert!(SolanaData::try_consensus(&data, 6, 0).is_none());
}

#[test]
fn test_strategy_jlp_balance_decimals_exact_consensus_pass() {
    let mut data = vec![base_data(); 7];
    data[1].strategy_jlp_balance_decimals = 5;
    data[2].strategy_jlp_balance_decimals = 5;
    assert!(SolanaData::try_consensus(&data, 5, 0).is_some());
}

#[test]
fn test_all_exact_fields_match_min_threshold() {
    let mut data = vec![base_data(); 7];
    for item in data.iter_mut().take(2) {
        item.custody_assets[0].decimals = 3;
        item.total_jlp_supply_decimals = 7;
        item.strategy_jlp_balance_decimals = 8;
    }
    let result = SolanaData::try_consensus(&data, 5, 10000);
    assert!(result.is_some());
    let consensus = result.unwrap();
    assert_eq!(consensus.total_jlp_supply_decimals, 6);
    assert_eq!(consensus.strategy_jlp_balance_decimals, 6);
    assert_eq!(consensus.custody_assets[0].decimals, 6);
    assert_eq!(consensus.custody_assets[1].decimals, 6);
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
    data[4].total_jlp_supply_decimals = 3;
    data[4].aum_usd = Uint128::new(999999999);
    data[5].strategy_jlp_balance_decimals = 4;
    data[5].aum_usd = Uint128::new(888888888);
    data[6].custody_assets[0].decimals = 5;
    data[6].aum_usd = Uint128::new(777777777);
    for (i, item) in data.iter_mut().enumerate().take(4) {
        item.aum_usd = Uint128::new(1000 + i as u128 * 2); // 1000, 1002, 1004, 1006
    }
    let result = SolanaData::try_consensus(&data, 4, 10000);
    assert!(result.is_some());
    let consensus = result.unwrap();
    let expected = base_data();
    assert_eq!(
        consensus.total_jlp_supply_decimals,
        expected.total_jlp_supply_decimals
    );
    assert_eq!(
        consensus.strategy_jlp_balance_decimals,
        expected.strategy_jlp_balance_decimals
    );
    assert_eq!(
        consensus.custody_assets[0].decimals,
        expected.custody_assets[0].decimals
    );
    assert_eq!(
        consensus.custody_assets[1].decimals,
        expected.custody_assets[1].decimals
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
