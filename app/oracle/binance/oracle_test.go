package binance

import (
	"context"
	"testing"
	"time"

	"cosmossdk.io/math"
	binance "github.com/adshao/go-binance/v2"
	binanceportfolio "github.com/adshao/go-binance/v2/portfolio"
	"github.com/golang/mock/gomock"
	neutronclient "github.com/structured-org/aum-oracle/client/neutron"
	solanaclient "github.com/structured-org/aum-oracle/client/solana"
	"github.com/structured-org/aum-oracle/oracle"
	mock_binance "github.com/structured-org/aum-oracle/testutil/mocks/binance-oracle"
	"go.uber.org/zap"
)

func TestOracleForNeutronRun(t *testing.T) {
	start := time.Now().UTC().Unix()
	ctrl := gomock.NewController(t)
	defer ctrl.Finish()

	binanceClient := mock_binance.NewMockBinanceClient(ctrl)
	neutronAum := mock_binance.NewMockNeutronAumContractClient(ctrl)

	neutronAum.EXPECT().GetBinanceAumContractNextRound(gomock.Any()).Return(&neutronclient.NextRound{
		Round: 1, Timestamp: start + 2,
	}, nil)

	binanceClient.EXPECT().GetUmPositions(gomock.Any()).Return([]*binanceportfolio.UMPosition{
		{Symbol: "BTCUSDT", PositionAmt: "0.148", UnrealizedProfit: "1985.41474317"},
		{Symbol: "XRPUSDT", PositionAmt: "-1643.52695386", UnrealizedProfit: "8362.86549337"},
	}, nil).AnyTimes()
	binanceClient.EXPECT().GetSpotAccountInfo(gomock.Any()).Return(&binance.Account{
		Balances: []binance.Balance{
			{Asset: "USDT", Free: "15.44528780", Locked: "15.44528780"},
			{Asset: "BTC", Free: "0.10000000", Locked: "0.05000000"},
			{Asset: "XRP", Free: "26.16936963", Locked: "0.10000000"},
		},
	}, nil).AnyTimes()
	binanceClient.EXPECT().GetPMAccountInfo(gomock.Any()).Return(&binanceportfolio.Account{
		ActualEquity:             "8.16774569",
		VirtualMaxWithdrawAmount: "32.69173084",
		UniMMR:                   "12.10275117",
	}, nil).AnyTimes()
	binanceClient.EXPECT().GetPMAccountBalance(gomock.Any()).Return([]*binanceportfolio.Balance{
		{Asset: "USDT", UMWalletBalance: "-0.00307684"},
	}, nil).AnyTimes()

	expectedData := &neutronclient.BinanceAumData{
		Unimmr: math.LegacyMustNewDecFromStr("12.10275117"),
		Positions: []neutronclient.BinancePosition{
			{Symbol: "BTCUSDT", Amount: math.LegacyMustNewDecFromStr("0.148"), Pnl: math.LegacyMustNewDecFromStr("1985.41474317")},
		},
		UmBalanceUsdt: math.LegacyMustNewDecFromStr("-0.00307684"),
		SpotBalances: []neutronclient.BinanceBalance{
			{Asset: "USDT", Amount: math.LegacyMustNewDecFromStr("30.89057560")},
			{Asset: "BTC", Amount: math.LegacyMustNewDecFromStr("0.15000000")},
		},
		PmAccountActualEquity: math.LegacyMustNewDecFromStr("8.16774569"),
		WithdrawableUsdt:      math.LegacyMustNewDecFromStr("32.69173084"),
	}
	neutronAum.EXPECT().SubmitBinanceAumData(gomock.Any(), expectedData).Return(&neutronclient.NextRound{
		Round: 2, Timestamp: start + 12,
	}, nil)

	config := Config{
		UmPositionsList: []string{"BTCUSDT", "ETHUSDT", "SOLUSDT"},
		SpotAssetsList:  []string{"USDT", "BTC", "ETH", "SOL"},
	}

	o := NewBinanceAumOracleForNeutron(binanceClient, neutronAum, config, zap.NewExample())
	ctx, cancel := context.WithCancel(context.Background())
	go oracle.RunOracle(ctx, o)

	time.Sleep(5 * time.Second)
	cancel()
}

func TestOracleForSolanaRun(t *testing.T) {
	start := time.Now().UTC().Unix()
	ctrl := gomock.NewController(t)
	defer ctrl.Finish()

	binanceClient := mock_binance.NewMockBinanceClient(ctrl)
	solanaAum := mock_binance.NewMockSolanaAumContractClient(ctrl)

	solanaAum.EXPECT().GetBinanceAumContractNextRound(gomock.Any()).Return(&solanaclient.NextRound{
		Round: 1, Timestamp: start + 2,
	}, nil)

	binanceClient.EXPECT().GetUmPositions(gomock.Any()).Return([]*binanceportfolio.UMPosition{
		{Symbol: "BTCUSDT", PositionAmt: "0.148", UnrealizedProfit: "1985.41474317"},
		{Symbol: "XRPUSDT", PositionAmt: "-1643.52695386", UnrealizedProfit: "8362.86549337"},
	}, nil).AnyTimes()
	binanceClient.EXPECT().GetSpotAccountInfo(gomock.Any()).Return(&binance.Account{
		Balances: []binance.Balance{
			{Asset: "USDT", Free: "15.44528780", Locked: "15.44528780"},
			{Asset: "BTC", Free: "0.10000000", Locked: "0.05000000"},
			{Asset: "XRP", Free: "26.16936963", Locked: "0.10000000"},
		},
	}, nil).AnyTimes()
	binanceClient.EXPECT().GetPMAccountInfo(gomock.Any()).Return(&binanceportfolio.Account{
		ActualEquity:             "8.16774569",
		VirtualMaxWithdrawAmount: "32.69173084",
		UniMMR:                   "12.10275117",
	}, nil).AnyTimes()
	binanceClient.EXPECT().GetPMAccountBalance(gomock.Any()).Return([]*binanceportfolio.Balance{
		{Asset: "USDT", UMWalletBalance: "-0.00307684"},
	}, nil).AnyTimes()

	expectedData := &solanaclient.BinanceAumData{
		Unimmr: 12.10275117,
		Positions: []solanaclient.BinancePosition{
			{Symbol: "BTCUSDT", Amount: 0.148, Pnl: 1985.41474317},
		},
		UmBalanceUsdt: -0.00307684,
		SpotBalances: []solanaclient.BinanceBalance{
			{Asset: "USDT", Amount: 30.89057560},
			// summing floats is not precise: https://blog.stackademic.com/those-annoying-golang-floats-d65b20ea82f3
			{Asset: "BTC", Amount: 0.15000000000000002},
		},
		PmAccountActualEquity: 8.16774569,
		WithdrawableUsdt:      32.69173084,
	}
	solanaAum.EXPECT().SubmitBinanceAumData(gomock.Any(), expectedData).Return(&solanaclient.NextRound{
		Round: 2, Timestamp: start + 12,
	}, nil)

	config := Config{
		UmPositionsList: []string{"BTCUSDT", "ETHUSDT", "SOLUSDT"},
		SpotAssetsList:  []string{"USDT", "BTC", "ETH", "SOL"},
	}

	o := NewBinanceAumOracleForSolana(binanceClient, solanaAum, config, zap.NewExample())
	ctx, cancel := context.WithCancel(context.Background())
	go oracle.RunOracle(ctx, o)

	time.Sleep(5 * time.Second)
	cancel()
}
