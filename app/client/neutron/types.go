package neutron

import (
	"cmp"
	"slices"

	"cosmossdk.io/math"
)

type NextRound struct {
	Round     int64 `json:"round"`
	Timestamp int64 `json:"timestamp"`
}

type BinanceData struct {
	Round                 int64             `json:"round"`
	Unimmr                math.LegacyDec    `json:"unimmr"`
	Positions             []BinancePosition `json:"positions"`
	UmBalanceUsdt         math.LegacyDec    `json:"um_balance_usdt"`
	SpotBalances          []BinanceBalance  `json:"spot_balances"`
	PmAccountActualEquity math.LegacyDec    `json:"pm_account_actual_equity"`
	WithdrawableUsdt      math.LegacyDec    `json:"withdrawable_usdt"`
}

type BinancePosition struct {
	Symbol string         `json:"symbol"`
	Amount math.LegacyDec `json:"amount"`
	Pnl    math.LegacyDec `json:"pnl"`
}

type BinanceBalance struct {
	Asset  string         `json:"asset"`
	Amount math.LegacyDec `json:"amount"`
}

type SolanaData struct {
	Round              int64                 `json:"round"`
	CustodyAssets      []JupiterCustodyAsset `json:"custody_assets"`
	AumUsd             math.Uint             `json:"aum_usd"`
	JlpTokenDecimals   uint8                 `json:"jlp_token_decimals"`
	TotalJlpSupply     math.Uint             `json:"total_jlp_supply"`
	StrategyJlpBalance math.Uint             `json:"strategy_jlp_balance"`
}

func (s *SolanaData) SortCustodyAssets() {
	slices.SortFunc(s.CustodyAssets, func(a, b JupiterCustodyAsset) int {
		return cmp.Compare(a.Denom, b.Denom)
	})
}

type JupiterCustodyAsset struct {
	Owned         uint64 `json:"owned"`
	Locked        uint64 `json:"locked"`
	GuaranteedUsd uint64 `json:"guaranteed_usd"`
	Decimals      uint8  `json:"decimals"`
	Denom         string `json:"denom"`
}
