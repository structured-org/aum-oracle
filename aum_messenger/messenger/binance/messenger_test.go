package binance

import (
	"context"
	"reflect"
	"testing"
	"time"

	"cosmossdk.io/math"
	binance "github.com/adshao/go-binance/v2"
	binanceportfolio "github.com/adshao/go-binance/v2/portfolio"
	"github.com/golang/mock/gomock"
	msgr "github.com/structured-org/aum-messenger/messenger"
	neutronclient "github.com/structured-org/aum-messenger/pkg/client/neutron"
	solanaclient "github.com/structured-org/aum-messenger/pkg/client/solana"
	mock_binance "github.com/structured-org/aum-messenger/testutil/mocks/binance-messenger"
	"go.uber.org/zap"
)

var msgrOpConfig = msgr.OperationalConfig{
	FailureDelay:     0 * time.Second,
	PreSubmitDelay:   0 * time.Second,
	FetchDataTimeout: 3 * time.Second,
}

func TestMessengerForNeutronRun(t *testing.T) {
	start := uint64(time.Now().UTC().Unix())
	ctrl := gomock.NewController(t)
	defer ctrl.Finish()

	binanceClient := mock_binance.NewMockBinanceClient(ctrl)
	neutronAumRecv := mock_binance.NewMockNeutronAumReceiverClient(ctrl)

	neutronAumRecv.EXPECT().GetBinanceAumReceiverNextRound(gomock.Any()).Return(&neutronclient.NextRound{
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
	neutronAumRecv.EXPECT().SubmitBinanceAumData(gomock.Any(), expectedData).Return(&neutronclient.NextRound{
		Round: 2, Timestamp: start + 12,
	}, nil)

	config := Config{
		UmPositionsList: []string{"BTCUSDT", "ETHUSDT", "SOLUSDT"},
		SpotAssetsList:  []string{"USDT", "BTC", "ETH", "SOL"},
	}

	o := NewBinanceAumMessengerForNeutron(binanceClient, neutronAumRecv, config, zap.NewExample())
	ctx, cancel := context.WithCancel(context.Background())
	go msgr.RunMessenger(ctx, o, msgrOpConfig)

	time.Sleep(4 * time.Second)
	cancel()
}

func TestMessengerForSolanaRun(t *testing.T) {
	start := uint64(time.Now().UTC().Unix())
	ctrl := gomock.NewController(t)
	defer ctrl.Finish()

	binanceClient := mock_binance.NewMockBinanceClient(ctrl)
	solanaAumRecv := mock_binance.NewMockSolanaAumReceiverClient(ctrl)

	solanaAumRecv.EXPECT().GetBinanceAumReceiverNextRound(gomock.Any()).Return(&solanaclient.NextRound{
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
	solanaAumRecv.EXPECT().SubmitBinanceAumData(gomock.Any(), expectedData).Return(&solanaclient.NextRound{
		Round: 2, Timestamp: start + 12,
	}, nil)

	config := Config{
		UmPositionsList: []string{"BTCUSDT", "ETHUSDT", "SOLUSDT"},
		SpotAssetsList:  []string{"USDT", "BTC", "ETH", "SOL"},
	}

	o := NewBinanceAumMessengerForSolana(binanceClient, solanaAumRecv, config, zap.NewExample())
	ctx, cancel := context.WithCancel(context.Background())
	go msgr.RunMessenger(ctx, o, msgrOpConfig)

	time.Sleep(4 * time.Second)
	cancel()
}

func TestSimulationForNeutronStoresValue(t *testing.T) {
	ctrl := gomock.NewController(t)
	defer ctrl.Finish()

	binanceClient := mock_binance.NewMockBinanceClient(ctrl)
	neutronAumRecv := mock_binance.NewMockNeutronAumReceiverClient(ctrl)

	neutronAumRecv.EXPECT().GetBinanceAumReceiverNextRound(gomock.Any()).Return(&neutronclient.NextRound{
		Round: 2, Timestamp: 1,
	}, nil).AnyTimes()

	binanceClient.EXPECT().GetUmPositions(gomock.Any()).Return([]*binanceportfolio.UMPosition{
		{Symbol: "BTCUSDT", PositionAmt: "0.5", UnrealizedProfit: "100.0"},
	}, nil).AnyTimes()
	binanceClient.EXPECT().GetSpotAccountInfo(gomock.Any()).Return(&binance.Account{
		Balances: []binance.Balance{
			{Asset: "USDT", Free: "10.0", Locked: "5.0"},
		},
	}, nil).AnyTimes()
	binanceClient.EXPECT().GetPMAccountInfo(gomock.Any()).Return(&binanceportfolio.Account{
		ActualEquity:             "200.0",
		VirtualMaxWithdrawAmount: "150.0",
		UniMMR:                   "1.5",
	}, nil).AnyTimes()
	binanceClient.EXPECT().GetPMAccountBalance(gomock.Any()).Return([]*binanceportfolio.Balance{
		{Asset: "USDT", UMWalletBalance: "50.0"},
	}, nil).AnyTimes()

	config := Config{
		UmPositionsList: []string{"BTCUSDT"},
		SpotAssetsList:  []string{"USDT"},
	}

	inner := NewBinanceAumMessengerForNeutron(binanceClient, neutronAumRecv, config, zap.NewNop())
	store := &msgr.LastValueStore[*neutronclient.BinanceAumData]{}
	sim := msgr.WrapWithSimulation(inner, store)

	data, err := sim.FetchData(context.Background())
	if err != nil {
		t.Fatalf("FetchData failed: %v", err)
	}

	_, err = sim.SubmitData(context.Background(), data)
	if err != nil {
		t.Fatalf("SubmitData failed: %v", err)
	}

	v, ok, capturedAt := store.Load()
	if !ok {
		t.Fatal("expected store to have a value")
	}
	if v == nil {
		t.Fatal("expected non-nil stored value")
	}
	if capturedAt.IsZero() {
		t.Fatal("expected capturedAt to be set")
	}

	expected := &neutronclient.BinanceAumData{
		Unimmr: math.LegacyMustNewDecFromStr("1.5"),
		Positions: []neutronclient.BinancePosition{
			{Symbol: "BTCUSDT", Amount: math.LegacyMustNewDecFromStr("0.5"), Pnl: math.LegacyMustNewDecFromStr("100.0")},
		},
		UmBalanceUsdt:         math.LegacyMustNewDecFromStr("50.0"),
		SpotBalances:          []neutronclient.BinanceBalance{{Asset: "USDT", Amount: math.LegacyMustNewDecFromStr("15.0")}},
		PmAccountActualEquity: math.LegacyMustNewDecFromStr("200.0"),
		WithdrawableUsdt:      math.LegacyMustNewDecFromStr("150.0"),
	}

	if !reflect.DeepEqual(v, expected) {
		t.Fatalf("stored value mismatch:\ngot:  %+v\nwant: %+v", v, expected)
	}
}

func TestSimulationForSolanaStoresValue(t *testing.T) {
	ctrl := gomock.NewController(t)
	defer ctrl.Finish()

	binanceClient := mock_binance.NewMockBinanceClient(ctrl)
	solanaAumRecv := mock_binance.NewMockSolanaAumReceiverClient(ctrl)

	solanaAumRecv.EXPECT().GetBinanceAumReceiverNextRound(gomock.Any()).Return(&solanaclient.NextRound{
		Round: 2, Timestamp: 1,
	}, nil).AnyTimes()

	binanceClient.EXPECT().GetUmPositions(gomock.Any()).Return([]*binanceportfolio.UMPosition{
		{Symbol: "BTCUSDT", PositionAmt: "0.5", UnrealizedProfit: "100.0"},
	}, nil).AnyTimes()
	binanceClient.EXPECT().GetSpotAccountInfo(gomock.Any()).Return(&binance.Account{
		Balances: []binance.Balance{
			{Asset: "USDT", Free: "10.0", Locked: "5.0"},
		},
	}, nil).AnyTimes()
	binanceClient.EXPECT().GetPMAccountInfo(gomock.Any()).Return(&binanceportfolio.Account{
		ActualEquity:             "200.0",
		VirtualMaxWithdrawAmount: "150.0",
		UniMMR:                   "1.5",
	}, nil).AnyTimes()
	binanceClient.EXPECT().GetPMAccountBalance(gomock.Any()).Return([]*binanceportfolio.Balance{
		{Asset: "USDT", UMWalletBalance: "50.0"},
	}, nil).AnyTimes()

	config := Config{
		UmPositionsList: []string{"BTCUSDT"},
		SpotAssetsList:  []string{"USDT"},
	}

	inner := NewBinanceAumMessengerForSolana(binanceClient, solanaAumRecv, config, zap.NewNop())
	store := &msgr.LastValueStore[*solanaclient.BinanceAumData]{}
	sim := msgr.WrapWithSimulation(inner, store)

	data, err := sim.FetchData(context.Background())
	if err != nil {
		t.Fatalf("FetchData failed: %v", err)
	}

	_, err = sim.SubmitData(context.Background(), data)
	if err != nil {
		t.Fatalf("SubmitData failed: %v", err)
	}

	v, ok, capturedAt := store.Load()
	if !ok {
		t.Fatal("expected store to have a value")
	}
	if v == nil {
		t.Fatal("expected non-nil stored value")
	}
	if capturedAt.IsZero() {
		t.Fatal("expected capturedAt to be set")
	}

	expected := &solanaclient.BinanceAumData{
		Unimmr: 1.5,
		Positions: []solanaclient.BinancePosition{
			{Symbol: "BTCUSDT", Amount: 0.5, Pnl: 100.0},
		},
		UmBalanceUsdt:         50.0,
		SpotBalances:          []solanaclient.BinanceBalance{{Asset: "USDT", Amount: 15.0}},
		PmAccountActualEquity: 200.0,
		WithdrawableUsdt:      150.0,
	}

	if !reflect.DeepEqual(v, expected) {
		t.Fatalf("stored value mismatch:\ngot:  %+v\nwant: %+v", v, expected)
	}
}
