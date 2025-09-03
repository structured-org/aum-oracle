package solana

// TODO: find out proper solana data types for BinanceAumData fields

// BinanceAumData contains all Binance data that is a matter of consensus for the Binance AUM
// receiver contract.
type BinanceAumData struct {
	// Unimmr is the Unified Account Maintenance Margin Ratio. It is the overall risk measure of
	// the entire portfolio.
	Unimmr float64
	// Positions is the list of positions in the Binance Portfolio Margin account.
	Positions []BinancePosition
	// UmBalanceUsdt is the USDT balance on Portfolio Margin account. Used for perpetual futures
	// funding payments.
	UmBalanceUsdt float64
	// SpotBalances is the list of balances on the Binance spot account.
	SpotBalances []BinanceBalance
	// PmAccountActualEquity is the actual equity of the Portfolio Margin account that also
	// represents the total collateral on the account.
	PmAccountActualEquity float64
	// WithdrawableUsdt is the max withdrawable amount in USDT allowed without making UniMMR
	// going under 1.05 which is when liquidation happens.
	WithdrawableUsdt float64
}

// BinancePosition is a perpetual futures position on Binance.
type BinancePosition struct {
	// Symbol is the symbol of the position.
	Symbol string
	// Amount is the amount of the position. Negative for shorts, positive for longs.
	Amount float64
	// Pnl is the profits and losses of the position. Negative for losses, positive for profits.
	Pnl float64
}

// BinanceBalance is some asset balance on an account on Binance.
type BinanceBalance struct {
	// Asset is the asset name.
	Asset string
	// Amount is the asset amount on the account balance.
	Amount float64
}

type Config struct {
	Discriminator [8]byte `json:"discriminator"`
	RoundTime     uint16  `json:"roundTime"`
}
