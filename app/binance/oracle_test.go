package binance

import (
	"context"
	"testing"
	"time"

	"cosmossdk.io/math"
	binance "github.com/adshao/go-binance/v2"
	binancefutures "github.com/adshao/go-binance/v2/futures"
	binanceportfolio "github.com/adshao/go-binance/v2/portfolio"
	"github.com/golang/mock/gomock"
	neutronclient "github.com/structured-org/aum-oracle/client/neutron"
	mock_binance "github.com/structured-org/aum-oracle/testutil/mocks/binance"
	"go.uber.org/zap"
)

func TestOracleRun(t *testing.T) {
	start := time.Now().UTC().Unix()
	ctrl := gomock.NewController(t)
	defer ctrl.Finish()

	binanceClient := mock_binance.NewMockBinanceClient(ctrl)
	neutronClient := mock_binance.NewMockNeutronClient(ctrl)

	neutronClient.EXPECT().GetBinanceAumContractNextRound(gomock.Any()).Return(&neutronclient.NextRound{
		Round: 1, Timestamp: start + 2,
	}, nil)

	binanceClient.EXPECT().GetUmPositions(gomock.Any()).Return([]*binancefutures.PositionRisk{
		{Symbol: "BTCUSDT", PositionAmt: "0.148", UnRealizedProfit: "1985.41474317"},
	}, nil)
	binanceClient.EXPECT().GetSpotAccountInfo(gomock.Any()).Return(&binance.Account{
		Balances: []binance.Balance{
			{Asset: "USDT", Free: "15.44528780", Locked: "15.44528780"},
			{Asset: "BTC", Free: "0.10000000", Locked: "0.05000000"},
		},
	}, nil)
	binanceClient.EXPECT().GetPMAccountInfo(gomock.Any()).Return(&binanceportfolio.Account{
		ActualEquity:             "8.16774569",
		VirtualMaxWithdrawAmount: "32.69173084",
		UniMMR:                   "12.10275117",
	}, nil)
	binanceClient.EXPECT().GetPMAccountBalance(gomock.Any()).Return([]*binanceportfolio.Balance{
		{Asset: "USDT", UMWalletBalance: "-0.00307684"},
	}, nil)

	expectedData := &neutronclient.BinanceData{
		Round:  1,
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
	neutronClient.EXPECT().SubmitBinanceAumData(gomock.Any(), expectedData).Return(&neutronclient.NextRound{
		Round: 2, Timestamp: start + 12,
	}, nil)

	oracle := NewOracle(binanceClient, neutronClient, zap.NewExample())
	ctx, cancel := context.WithCancel(context.Background())

	go oracle.Run(ctx)

	time.Sleep(5 * time.Second)
	cancel()
}
