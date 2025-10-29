package neutron

import (
	"cmp"
	"slices"

	"cosmossdk.io/math"
)

// NextRound contains AUM receiver contract's next consensus round information.
type NextRound struct {
	// Round is the next consensus round number.
	Round uint64 `json:"round"`
	// Timestamp is the timestamp of the next consensus round beginning.
	Timestamp uint64 `json:"timestamp"`
}

// BinanceAumData contains all Binance data that is a matter of consensus for the Binance AUM
// receiver contract.
type BinanceAumData struct {
	// Unimmr is the Unified Account Maintenance Margin Ratio. It is the overall risk measure of
	// the entire portfolio.
	Unimmr math.LegacyDec `json:"unimmr"`
	// Positions is the list of positions in the Binance Portfolio Margin account.
	Positions []BinancePosition `json:"positions"`
	// UmBalanceUsdt is the USDT balance on Portfolio Margin account. Used for perpetual futures
	// funding payments.
	UmBalanceUsdt math.LegacyDec `json:"um_balance_usdt"`
	// SpotBalances is the list of balances on the Binance spot account.
	SpotBalances []BinanceBalance `json:"spot_balances"`
	// PmAccountActualEquity is the actual equity of the Portfolio Margin account that also
	// represents the total collateral on the account.
	PmAccountActualEquity math.LegacyDec `json:"pm_account_actual_equity"`
	// WithdrawableUsdt is the max withdrawable amount in USDT allowed without making UniMMR
	// going under 1.05 which is when liquidation happens.
	WithdrawableUsdt math.LegacyDec `json:"withdrawable_usdt"`
}

// BinancePosition is a perpetual futures position on Binance.
type BinancePosition struct {
	// Symbol is the symbol of the position.
	Symbol string `json:"symbol"`
	// Amount is the amount of the position. Negative for shorts, positive for longs.
	Amount math.LegacyDec `json:"amount"`
	// Pnl is the profits and losses of the position. Negative for losses, positive for profits.
	Pnl math.LegacyDec `json:"pnl"`
}

// BinanceBalance is some asset balance on an account on Binance.
type BinanceBalance struct {
	// Asset is the asset name.
	Asset string `json:"asset"`
	// Amount is the asset amount on the account balance.
	Amount math.LegacyDec `json:"amount"`
}

// JupiterAumData contains all Jupiter data that is a matter of consensus for the Jupiter AUM contract.
type JupiterAumData struct {
	// CustodyAssets contains information about Jupiter custodies.
	CustodyAssets []JupiterCustodyAsset `json:"custody_assets"`
	// AumUsd is the total Jupiter protocol AUM in USD.
	AumUsd math.Uint `json:"aum_usd"`
	// TotalJlpSupply is the total supply of the JLP token.
	TotalJlpSupply math.Uint `json:"total_jlp_supply"`
	// TotalJlpSupplyDecimals is the number of decimals of the JLP token supply.
	TotalJlpSupplyDecimals uint8 `json:"total_jlp_supply_decimals"`
	// StrategyJlpBalance is the amount of the JLP token that is held by the Jupiter strategy
	// address.
	StrategyJlpBalance math.Uint `json:"strategy_jlp_balance"`
	// StrategyJlpBalanceDecimals is the number of decimals of the strategy jlp balance
	StrategyJlpBalanceDecimals uint8 `json:"strategy_jlp_balance_decimals"`
}

// SortCustodyAssets sorts the custody assets by their denomination.
func (s *JupiterAumData) SortCustodyAssets() {
	slices.SortFunc(s.CustodyAssets, func(a, b JupiterCustodyAsset) int {
		return cmp.Compare(a.Denom, b.Denom)
	})
}

// JupiterCustodyAsset contains information about a custody asset on a Jupiter custody.
type JupiterCustodyAsset struct {
	// Owned is the amount of the asset owned by the Jupiter custody.
	Owned uint64 `json:"owned"`
	// Locked is the amount of the asset locked in the Jupiter custody.
	Locked uint64 `json:"locked"`
	// GuaranteedUsd is an estimate of the total size of all long positions.
	GuaranteedUsd uint64 `json:"guaranteed_usd"`
	// Decimals is the number of decimals of the custody asset.
	Decimals uint8 `json:"decimals"`
	// Denom is the custody asset denomination.
	Denom string `json:"denom"`
}

// Smart contract types

// GetRoundResponse is the response of AUM receiver smart contracts to the "get_round_info" query
type GetRoundResponse struct {
	// PendingRound is a currently pending round.
	PendingRound Round `json:"pending_round"`
	// NextRound is the next round.
	NextRound Round `json:"next_round"`
}

// Round is the round info from AUM receiver smart contracts.
type Round struct {
	// Round is a number of the round.
	Round uint64 `json:"round"`
	// Start is when the round started (UNIX timestamp in seconds).
	Start uint64 `json:"start"`
}
