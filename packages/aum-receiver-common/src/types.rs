use crate::constants::WBTC_DECIMALS;
use consensus::consensus::Round;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Int256;

/// Response type for AUM Oracle receiver GetRoundInfo queries.
#[cw_serde]
pub struct RoundInfoResponse {
    /// Currently pending round
    pub pending_round: Round,
    /// The next round
    pub next_round: Round,
}

/// Response type for AUM Oracle receiver GetAum queries. **Must** return the AUM in micro-Bitcoin
/// (uwBTC) with precision of `WBTC_DECIMALS`.
#[cw_serde]
pub struct GetAumResponse {
    /// The latest AUM in the AUM Oracle receiver reported by messengers scaled to `WBTC_DECIMALS`.
    pub aum_in_wbtc: Int256,
    /// Represents the number of decimals that the `aum_in_wbtc` is represented in. Must always
    /// equal to `WBTC_DECIMALS`. It is used to scale the `aum_in_wbtc` to its base BTC value.
    /// E.g. `base_aum_in_btc = aum_in_wbtc / 10^decimals`.
    pub decimals: u32,
}

/// Helper function to create a GetAumResponse from a uwBTC value with the correct decimals.
/// The aum_in_wbtc must be a value of `WBTC_DECIMALS` precision.
pub fn aum_response_from_uwbtc(aum_in_wbtc: Int256) -> GetAumResponse {
    GetAumResponse {
        aum_in_wbtc,
        decimals: WBTC_DECIMALS,
    }
}
