package solana

import (
	solanabin "github.com/gagliardetto/binary"
	solana "github.com/gagliardetto/solana-go"
)

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

type JupiterPoolAccount struct {
	Discriminator          [8]byte
	Name                   string
	Custodies              []solana.PublicKey
	AumUsd                 solanabin.Uint128
	Limit                  JupiterPoolLimit
	Fees                   JupiterPoolFees
	StableSwapTaxBps       uint64
	LiquidationRewardBps   uint64
	ProtocolShareBps       uint64
	PoolApr                JupiterPoolApr
	MaxRequestExecutionSec int64
	Bump                   uint8
	LpTokenBump            uint8
	InceptionTime          int64
	ParameterUpdateOracle  Secp256k1Pubkey
}

type JupiterPoolLimit struct {
	MaxAumUsd               solanabin.Uint128
	TokenWeightageBufferBps uint64
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
	RealizedFeeUsd solanabin.Uint128
}

type Secp256k1Pubkey struct {
	Discriminator [8]byte
	Key           [32]byte
}
