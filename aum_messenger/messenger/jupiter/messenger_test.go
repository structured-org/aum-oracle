package jupiter

import (
	"context"
	"reflect"
	"testing"
	"time"

	"cosmossdk.io/math"
	solanabin "github.com/gagliardetto/binary"
	solana "github.com/gagliardetto/solana-go"
	solanarpc "github.com/gagliardetto/solana-go/rpc"
	"github.com/golang/mock/gomock"
	msgr "github.com/structured-org/aum-messenger/messenger"
	jupiterclient "github.com/structured-org/aum-messenger/pkg/client/jupiter"
	neutronclient "github.com/structured-org/aum-messenger/pkg/client/neutron"
	mock_jupiter "github.com/structured-org/aum-messenger/testutil/mocks/jupiter-messenger"
	"go.uber.org/zap"
)

var (
	testPubKey1 = solana.MustPublicKeyFromBase58("wczmirTBMa3tGisAeecZcGbzvgi6VAG4Gu8uamtHBf3")
	testPubKey2 = solana.MustPublicKeyFromBase58("3csuXZKah5rgpb8RiwX9XfjrMxcp3u1K9mBdCwL51spj")
	testPubKey3 = solana.MustPublicKeyFromBase58("F6ZjiBm1WgVXzez5vxHeBDgaVPQRfLyFb7GFwXvyZVxD")
	testPubKey4 = solana.MustPublicKeyFromBase58("E1bQJ8eMMn3zmeSewW3HQ8zmJr7KR75JonbwAtWx2bux")
	testPubKey5 = solana.MustPublicKeyFromBase58("92q4Y2xGE39Bm2JgNZLZWuafoBiBR4gCj4igfpwvpgcD")

	msgrOpConfig = msgr.OperationalConfig{
		FailureDelay:     0 * time.Second,
		PreSubmitDelay:   0 * time.Second,
		FetchDataTimeout: 3 * time.Second,
	}
)

func TestMessengerForNeutronRun(t *testing.T) {
	start := uint64(time.Now().UTC().Unix())
	ctrl := gomock.NewController(t)
	defer ctrl.Finish()

	solanaClient := mock_jupiter.NewMockSolanaClient(ctrl)
	neutronAumRecv := mock_jupiter.NewMockNeutronAumReceiverClient(ctrl)
	jupiterClient := mock_jupiter.NewMockJupiterClient(ctrl)
	usdtPubKey := testPubKey1
	usdcPubKey := testPubKey2
	btcPubKey := testPubKey3
	ethPubKey := testPubKey4
	solPubKey := testPubKey5
	jupCfg := JupiterConfig{
		Custodies: map[string]solana.PublicKey{
			"USDT": usdtPubKey,
			"USDC": usdcPubKey,
			"BTC":  btcPubKey,
			"ETH":  ethPubKey,
			"SOL":  solPubKey,
		},
	}

	neutronAumRecv.EXPECT().GetJupiterAumReceiverNextRound(gomock.Any()).Return(&neutronclient.NextRound{
		Round: 1, Timestamp: start + 2,
	}, nil)

	jupiterClient.EXPECT().GetJupiterPoolInfo(gomock.Any(), gomock.Any()).Return(&jupiterclient.JupiterPoolAccount{
		AumUsd: solanabin.Uint128{
			Lo: 5000000 * 1000000,
		},
	}, nil)
	solanaClient.EXPECT().GetTokenSupply(gomock.Any(), gomock.Any()).Return(&solanarpc.UiTokenAmount{
		Amount: "10000000", Decimals: 6,
	}, nil)
	solanaClient.EXPECT().GetTokenAccountBalance(gomock.Any(), gomock.Any(), gomock.Any()).Return(&solanarpc.UiTokenAmount{
		Amount: "2000000", Decimals: 6,
	}, nil)
	jupiterClient.EXPECT().GetJupiterCustodyInfo(gomock.Any(), usdtPubKey).Return(&jupiterclient.JupiterPerpsCustodyAccount{
		Assets: jupiterclient.JupiterPerpsCustodyAssets{Owned: 1000000, Locked: 2000000, GuaranteedUsd: 3000000}, Decimals: 6,
	}, nil)
	jupiterClient.EXPECT().GetJupiterCustodyInfo(gomock.Any(), usdcPubKey).Return(&jupiterclient.JupiterPerpsCustodyAccount{
		Assets: jupiterclient.JupiterPerpsCustodyAssets{Owned: 1000000, Locked: 2000000, GuaranteedUsd: 3000000}, Decimals: 6,
	}, nil)
	jupiterClient.EXPECT().GetJupiterCustodyInfo(gomock.Any(), btcPubKey).Return(&jupiterclient.JupiterPerpsCustodyAccount{
		Assets: jupiterclient.JupiterPerpsCustodyAssets{Owned: 10000, Locked: 20000, GuaranteedUsd: 3000000}, Decimals: 6,
	}, nil)
	jupiterClient.EXPECT().GetJupiterCustodyInfo(gomock.Any(), ethPubKey).Return(&jupiterclient.JupiterPerpsCustodyAccount{
		Assets: jupiterclient.JupiterPerpsCustodyAssets{Owned: 100000, Locked: 200000, GuaranteedUsd: 3000000}, Decimals: 6,
	}, nil)
	jupiterClient.EXPECT().GetJupiterCustodyInfo(gomock.Any(), solPubKey).Return(&jupiterclient.JupiterPerpsCustodyAccount{
		Assets: jupiterclient.JupiterPerpsCustodyAssets{Owned: 500000, Locked: 700000, GuaranteedUsd: 3000000}, Decimals: 6,
	}, nil)

	expectedData := &neutronclient.JupiterAumData{
		CustodyAssets: []neutronclient.JupiterCustodyAsset{
			{Denom: "USDT", Owned: 1000000, Locked: 2000000, GuaranteedUsd: 3000000, Decimals: 6},
			{Denom: "USDC", Owned: 1000000, Locked: 2000000, GuaranteedUsd: 3000000, Decimals: 6},
			{Denom: "BTC", Owned: 10000, Locked: 20000, GuaranteedUsd: 3000000, Decimals: 6},
			{Denom: "ETH", Owned: 100000, Locked: 200000, GuaranteedUsd: 3000000, Decimals: 6},
			{Denom: "SOL", Owned: 500000, Locked: 700000, GuaranteedUsd: 3000000, Decimals: 6},
		},
		AumUsd:                     math.NewUintFromString("5000000"),
		TotalJlpSupply:             math.NewUintFromString("10000000"),
		StrategyJlpBalance:         math.NewUintFromString("2000000"),
		TotalJlpSupplyDecimals:     6,
		StrategyJlpBalanceDecimals: 6,
	}
	expectedData.SortCustodyAssets()
	neutronAumRecv.EXPECT().SubmitJupiterAumData(gomock.Any(), expectedData).Return(&neutronclient.NextRound{
		Round: 2, Timestamp: start + 12,
	}, nil)

	o := NewJupiterAumMessengerForNeutron(
		solanaClient,
		neutronAumRecv,
		jupiterClient,
		jupCfg,
		zap.NewExample(),
	)
	ctx, cancel := context.WithCancel(context.Background())
	go msgr.RunMessenger(ctx, o, msgrOpConfig)

	time.Sleep(4 * time.Second)
	cancel()
}

func TestSimulationForNeutronStoresValue(t *testing.T) {
	ctrl := gomock.NewController(t)
	defer ctrl.Finish()

	solanaClient := mock_jupiter.NewMockSolanaClient(ctrl)
	neutronAumRecv := mock_jupiter.NewMockNeutronAumReceiverClient(ctrl)
	jupiterClient := mock_jupiter.NewMockJupiterClient(ctrl)

	usdtPubKey := testPubKey1
	btcPubKey := testPubKey3

	jupCfg := JupiterConfig{
		Custodies: map[string]solana.PublicKey{
			"USDT": usdtPubKey,
			"BTC":  btcPubKey,
		},
	}

	neutronAumRecv.EXPECT().GetJupiterAumReceiverNextRound(gomock.Any()).Return(&neutronclient.NextRound{
		Round: 2, Timestamp: 1,
	}, nil).AnyTimes()

	jupiterClient.EXPECT().GetJupiterPoolInfo(gomock.Any(), gomock.Any()).Return(&jupiterclient.JupiterPoolAccount{
		AumUsd: solanabin.Uint128{Lo: 1000000},
	}, nil).AnyTimes()
	solanaClient.EXPECT().GetTokenSupply(gomock.Any(), gomock.Any()).Return(&solanarpc.UiTokenAmount{
		Amount: "500000", Decimals: 6,
	}, nil).AnyTimes()
	solanaClient.EXPECT().GetTokenAccountBalance(gomock.Any(), gomock.Any(), gomock.Any()).Return(&solanarpc.UiTokenAmount{
		Amount: "100000", Decimals: 6,
	}, nil).AnyTimes()
	jupiterClient.EXPECT().GetJupiterCustodyInfo(gomock.Any(), usdtPubKey).Return(&jupiterclient.JupiterPerpsCustodyAccount{
		Assets: jupiterclient.JupiterPerpsCustodyAssets{Owned: 10000, Locked: 5000, GuaranteedUsd: 15000}, Decimals: 6,
	}, nil).AnyTimes()
	jupiterClient.EXPECT().GetJupiterCustodyInfo(gomock.Any(), btcPubKey).Return(&jupiterclient.JupiterPerpsCustodyAccount{
		Assets: jupiterclient.JupiterPerpsCustodyAssets{Owned: 2000, Locked: 1000, GuaranteedUsd: 3000}, Decimals: 8,
	}, nil).AnyTimes()

	inner := NewJupiterAumMessengerForNeutron(solanaClient, neutronAumRecv, jupiterClient, jupCfg, zap.NewNop())
	store := &msgr.LastValueStore[*neutronclient.JupiterAumData]{}
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

	expected := &neutronclient.JupiterAumData{
		CustodyAssets: []neutronclient.JupiterCustodyAsset{
			{Denom: "BTC", Owned: 2000, Locked: 1000, GuaranteedUsd: 3000, Decimals: 8},
			{Denom: "USDT", Owned: 10000, Locked: 5000, GuaranteedUsd: 15000, Decimals: 6},
		},
		AumUsd:                     math.NewUintFromString("1"),
		TotalJlpSupply:             math.NewUintFromString("500000"),
		StrategyJlpBalance:         math.NewUintFromString("100000"),
		TotalJlpSupplyDecimals:     6,
		StrategyJlpBalanceDecimals: 6,
	}

	if !reflect.DeepEqual(v, expected) {
		t.Fatalf("stored value mismatch:\ngot:  %+v\nwant: %+v", v, expected)
	}
}
