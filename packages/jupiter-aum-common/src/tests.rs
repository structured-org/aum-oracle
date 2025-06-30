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
        jlp_token_decimals: 6,
        total_jlp_supply: Uint128::new(jlp),
        strategy_jlp_balance: Uint128::new(strat),
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

#[test]
fn custody_assets_mismatch_fails() {
    let a = make_data(100, 100, 100, "usdc", 100);
    let mut b = make_data(100, 100, 100, "usdt", 100);
    let c = make_data(100, 100, 100, "usdc", 100);
    // order matters so force align
    b.custody_assets.sort_by(|a, b| a.denom.cmp(&b.denom));
    let result = SolanaData::try_consensus(&[a, b, c], 3, 10_000);
    assert!(result.is_none()); // denom mismatch fails exact consensus
}

#[test]
fn custody_assets_length_mismatch_returns_none() {
    let d1 = make_data(100, 100, 100, "usdc", 100);
    let mut d2 = make_data(100, 100, 100, "usdc", 100);
    d2.custody_assets.push(sample_asset("usdt", 100));
    let result = SolanaData::try_consensus(&[d1, d2], 2, 10_000);
    assert!(result.is_none());
}
