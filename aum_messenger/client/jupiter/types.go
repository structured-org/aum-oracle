package jupiter

import (
	solanabin "github.com/gagliardetto/binary"
	solana "github.com/gagliardetto/solana-go"
)

// JupiterPerpsCustodyAccount is the model for Jupiter perps custody account data.
type JupiterPerpsCustodyAccount struct {
	Discriminator       [8]byte                              `json:"discriminator"`
	Pool                solana.PublicKey                     `json:"pool"`
	Mint                solana.PublicKey                     `json:"mint"`
	TokenAccount        solana.PublicKey                     `json:"tokenAccount"`
	Decimals            uint8                                `json:"decimals"`
	IsStable            bool                                 `json:"isStable"`
	Oracle              JupiterPerpsCustodyOracle            `json:"oracle"`
	Pricing             JupiterPerpsCustodyPricing           `json:"pricing"`
	Permissions         JupiterPerpsCustodyPermissions       `json:"permissions"`
	TargetRatioBps      uint64                               `json:"targetRatioBps"`
	Assets              JupiterPerpsCustodyAssets            `json:"assets"`
	FundingRateState    JupiterPerpsCustodyFundingRateState  `json:"fundingRateState"`
	Bump                uint8                                `json:"bump"`
	TokenAccountBump    uint8                                `json:"tokenAccountBump"`
	IncreasePositionBps uint64                               `json:"increasePositionBps"`
	DecreasePositionBps uint64                               `json:"decreasePositionBps"`
	MaxPositionSizeUsd  uint64                               `json:"maxPositionSizeUsd"`
	DovesOracle         solana.PublicKey                     `json:"dovesOracle"`
	JumpRateState       JupiterPerpsCustodyJumpRateState     `json:"jumpRateState"`
	DovesAgOracle       solana.PublicKey                     `json:"dovesAgOracle"`
	PriceImpactBuffer   JupiterPerpsCustodyPriceImpactBuffer `json:"priceImpactBuffer"`
}

type JupiterPerpsCustodyOracle struct {
	OracleAccount  solana.PublicKey `json:"oracleAccount"`
	OracleType     uint8            `json:"oracleType"`
	Buffer         uint64           `json:"buffer"`
	MaxPriceAgeSec uint32           `json:"maxPriceAgeSec"`
}

type JupiterPerpsCustodyPricing struct {
	TradeImpactFeeScalar uint64 `json:"tradeImpactFeeScalar"`
	Buffer               uint64 `json:"buffer"`
	SwapSpread           uint64 `json:"swapSpread"`
	MaxLeverage          uint64 `json:"maxLeverage"`
	MaxGlobalLongSizes   uint64 `json:"maxGlobalLongSizes"`
	MaxGlobalShortSizes  uint64 `json:"maxGlobalShortSizes"`
}

type JupiterPerpsCustodyPermissions struct {
	AllowSwap                 bool `json:"allowSwap"`
	AllowAddLiquidity         bool `json:"allowAddLiquidity"`
	AllowRemoveLiquidity      bool `json:"allowRemoveLiquidity"`
	AllowIncreasePosition     bool `json:"allowIncreasePosition"`
	AllowDecreasePosition     bool `json:"allowDecreasePosition"`
	AllowCollateralWithdrawal bool `json:"allowCollateralWithdrawal"`
	AllowLiquidatePosition    bool `json:"allowLiquidatePosition"`
}

type JupiterPerpsCustodyAssets struct {
	FeesReserves             uint64 `json:"feesReserves"`
	Owned                    uint64 `json:"owned"`
	Locked                   uint64 `json:"locked"`
	GuaranteedUsd            uint64 `json:"guaranteedUsd"`
	GlobalShortSizes         uint64 `json:"globalShortSizes"`
	GlobalShortAveragePrices uint64 `json:"globalShortAveragePrices"`
}

type JupiterPerpsCustodyFundingRateState struct {
	CumulativeInterestRate solanabin.Uint128 `json:"cumulativeInterestRate"`
	LastUpdate             int64             `json:"lastUpdate"`
	HourlyFundingDbps      uint64            `json:"hourlyFundingDbps"`
}

type JupiterPerpsCustodyJumpRateState struct {
	MinRateBps            uint64 `json:"minRateBps"`
	MaxRateBps            uint64 `json:"maxRateBps"`
	TargetRateBps         uint64 `json:"targetRateBps"`
	TargetUtilizationRate uint64 `json:"targetUtilizationRate"`
}

type JupiterPerpsCustodyPriceImpactBuffer struct {
	OpenInterest            [60]int64 `json:"openInterest"`
	LastUpdated             int64     `json:"lastUpdated"`
	FeeFactor               uint64    `json:"feeFactor"`
	Exponent                float32   `json:"exponent"`
	DeltaImbalanceThreshold uint64    `json:"deltaImbalanceThreshold"`
	MaxFeeBps               uint64    `json:"maxFeeBps"`
}

// JupiterPoolAccount is the model for Jupiter pool account data.
type JupiterPoolAccount struct {
	Discriminator          [8]byte            `json:"discriminator"`
	Name                   string             `json:"name"`
	Custodies              []solana.PublicKey `json:"custodies"`
	AumUsd                 solanabin.Uint128  `json:"aumUsd"`
	Limit                  JupiterPoolLimit   `json:"limit"`
	Fees                   JupiterPoolFees    `json:"fees"`
	PoolApr                JupiterPoolApr     `json:"poolApr"`
	MaxRequestExecutionSec int64              `json:"maxRequestExecutionSec"`
	Bump                   uint8              `json:"bump"`
	LpTokenBump            uint8              `json:"lpTokenBump"`
	InceptionTime          int64              `json:"inceptionTime"`
	ParameterUpdateOracle  Secp256k1Pubkey    `json:"parameterUpdateOracle"`
}

type JupiterPoolLimit struct {
	MaxAumUsd               solanabin.Uint128 `json:"maxAumUsd"`
	TokenWeightageBufferBps solanabin.Uint128 `json:"tokenWeightageBufferBps"`
	Buffer                  uint64            `json:"buffer"`
}

type JupiterPoolFees struct {
	SwapMultiplier        uint64 `json:"swapMultiplier"`
	StableSwapMultiplier  uint64 `json:"stableSwapMultiplier"`
	AddRemoveLiquidityBps uint64 `json:"addRemoveLiquidityBps"`
	SwapBps               uint64 `json:"swapBps"`
	TaxBps                uint64 `json:"taxBps"`
	StableSwapBps         uint64 `json:"stableSwapBps"`
	StableSwapTaxBps      uint64 `json:"stableSwapTaxBps"`
	LiquidationRewardBps  uint64 `json:"liquidationRewardBps"`
	ProtocolShareBps      uint64 `json:"protocolShareBps"`
}

type JupiterPoolApr struct {
	LastUpdated    int64  `json:"lastUpdated"`
	FeeAprBps      uint64 `json:"feeAprBps"`
	RealizedFeeUsd uint64 `json:"realizedFeeUsd"`
}

type Secp256k1Pubkey struct {
	Prefix uint8     `json:"prefix"`
	Key    [32]uint8 `json:"key"`
}
