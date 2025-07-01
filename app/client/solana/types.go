package solana

import (
	solanabin "github.com/gagliardetto/binary"
	solana "github.com/gagliardetto/solana-go"
)

// JupiterPerpsCustodyAccount is the model for Jupiter perps custody account data.
type JupiterPerpsCustodyAccount struct {
	Discriminator       [8]byte
	Pool                solana.PublicKey
	Mint                solana.PublicKey
	TokenAccount        solana.PublicKey
	Decimals            uint8
	IsStable            bool
	Oracle              JupiterPerpsCustodyOracle
	Pricing             JupiterPerpsCustodyPricing
	Permissions         JupiterPerpsCustodyPermissions
	TargetRatioBps      uint64
	Assets              JupiterPerpsCustodyAssets
	FundingRateState    JupiterPerpsCustodyFundingRateState
	Bump                uint8
	TokenAccountBump    uint8
	IncreasePositionBps uint64
	DecreasePositionBps uint64
	MaxPositionSizeUsd  uint64
	DovesOracle         solana.PublicKey
	JumpRateState       JupiterPerpsCustodyJumpRateState
	DovesAgOracle       solana.PublicKey
	PriceImpactBuffer   JupiterPerpsCustodyPriceImpactBuffer
}

type JupiterPerpsCustodyOracle struct {
	OracleAccount  solana.PublicKey
	OracleType     uint8
	Buffer         uint64
	MaxPriceAgeSec uint32
}

type JupiterPerpsCustodyPricing struct {
	TradeImpactFeeScalar uint64
	Buffer               uint64
	SwapSpread           uint64
	MaxLeverage          uint64
	MaxGlobalLongSizes   uint64
	MaxGlobalShortSizes  uint64
}

type JupiterPerpsCustodyPermissions struct {
	AllowSwap                 bool
	AllowAddLiquidity         bool
	AllowRemoveLiquidity      bool
	AllowIncreasePosition     bool
	AllowDecreasePosition     bool
	AllowCollateralWithdrawal bool
	AllowLiquidatePosition    bool
}

type JupiterPerpsCustodyAssets struct {
	FeesReserves             uint64
	Owned                    uint64
	Locked                   uint64
	GuaranteedUsd            uint64
	GlobalShortSizes         uint64
	GlobalShortAveragePrices uint64
}

type JupiterPerpsCustodyFundingRateState struct {
	CumulativeInterestRate solanabin.Uint128
	LastUpdate             int64
	HourlyFundingDbps      uint64
}

type JupiterPerpsCustodyJumpRateState struct {
	MinRateBps            uint64
	MaxRateBps            uint64
	TargetRateBps         uint64
	TargetUtilizationRate uint64
}

type JupiterPerpsCustodyPriceImpactBuffer struct {
	OpenInterest            [60]int64
	LastUpdated             int64
	FeeFactor               uint64
	Exponent                float32
	DeltaImbalanceThreshold uint64
	MaxFeeBps               uint64
}

// JupiterPoolAccount is the model for Jupiter pool account data.
type JupiterPoolAccount struct {
	Discriminator          [8]byte
	Name                   string
	Custodies              []solana.PublicKey
	AumUsd                 solanabin.Uint128
	Limit                  JupiterPoolLimit
	Fees                   JupiterPoolFees
	PoolApr                JupiterPoolApr
	MaxRequestExecutionSec int64
	Bump                   uint8
	LpTokenBump            uint8
	InceptionTime          int64
	ParameterUpdateOracle  Secp256k1Pubkey
}

type JupiterPoolLimit struct {
	MaxAumUsd               solanabin.Uint128
	TokenWeightageBufferBps solanabin.Uint128
	Buffer                  uint64
}

type JupiterPoolFees struct {
	SwapMultiplier        uint64
	StableSwapMultiplier  uint64
	AddRemoveLiquidityBps uint64
	SwapBps               uint64
	TaxBps                uint64
	StableSwapBps         uint64
	StableSwapTaxBps      uint64
	LiquidationRewardBps  uint64
	ProtocolShareBps      uint64
}

type JupiterPoolApr struct {
	LastUpdated    int64
	FeeAprBps      uint64
	RealizedFeeUsd uint64
}

type Secp256k1Pubkey struct {
	Prefix uint8
	Key    [32]uint8
}

// NextRound contains AUM contract's next consensus round information.
type NextRound struct {
	// Round is the next consensus round number.
	Round int64
	// Timestamp is the timestamp of the next consensus round beginning.
	Timestamp int64
}

// TODO: find out proper solana data types for BinanceData fields

// BinanceData contains all Binance data that is a matter of consensus for the Binance AUM contract.
type BinanceData struct {
	// Round is the consensus round number.
	Round int64
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
