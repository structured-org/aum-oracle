package jupiter

import (
	"context"
	"testing"
	"time"

	"cosmossdk.io/math"
	solanabin "github.com/gagliardetto/binary"
	solana "github.com/gagliardetto/solana-go"
	solanatoken "github.com/gagliardetto/solana-go/programs/token"
	solanarpc "github.com/gagliardetto/solana-go/rpc"
	"github.com/golang/mock/gomock"
	jupiterclient "github.com/structured-org/aum-messenger/client/jupiter"
	neutronclient "github.com/structured-org/aum-messenger/client/neutron"
	msgr "github.com/structured-org/aum-messenger/messenger"
	mock_jupiter "github.com/structured-org/aum-messenger/testutil/mocks/jupiter-messenger"
	"go.uber.org/zap"
)

var (
	testPubKey1    = solana.MustPublicKeyFromBase58("wczmirTBMa3tGisAeecZcGbzvgi6VAG4Gu8uamtHBf3")
	testPubKey2    = solana.MustPublicKeyFromBase58("3csuXZKah5rgpb8RiwX9XfjrMxcp3u1K9mBdCwL51spj")
	testPubKey3    = solana.MustPublicKeyFromBase58("F6ZjiBm1WgVXzez5vxHeBDgaVPQRfLyFb7GFwXvyZVxD")
	testPubKey4    = solana.MustPublicKeyFromBase58("E1bQJ8eMMn3zmeSewW3HQ8zmJr7KR75JonbwAtWx2bux")
	testPubKey5    = solana.MustPublicKeyFromBase58("92q4Y2xGE39Bm2JgNZLZWuafoBiBR4gCj4igfpwvpgcD")
	strategyPubKey = solana.MustPublicKeyFromBase58("8HqpYJ8F8BHdpHbUDbdfbjAPKZxdeN4vMXUkPDhtoy59")
	jlpTokenPubKey = solana.MustPublicKeyFromBase58("EJmzZhQBX19G8xXLzx2eTUFCgTE9hH8igxNYoas7ekYE")

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
		SolanaBalancesList: map[solana.PublicKey][]string{
			strategyPubKey: {jlpTokenPubKey.String(), "SOL"},
		},
		SolanaTokenSupplyList: []solana.PublicKey{jlpTokenPubKey},
	}

	neutronAumRecv.EXPECT().GetJupiterAumReceiverNextRound(gomock.Any()).Return(&neutronclient.NextRound{
		Round: 1, Timestamp: start + 2,
	}, nil)

	jupiterClient.EXPECT().GetJupiterPoolInfo(gomock.Any(), gomock.Any()).Return(&jupiterclient.JupiterPoolAccount{
		AumUsd: solanabin.Uint128{
			Lo: 5000000 * 1000000,
		},
	}, nil)
	solanaClient.EXPECT().GetTokenMint(gomock.Any(), gomock.Any()).Return(&solanatoken.Mint{
		Supply: 10000000, Decimals: 6,
	}, nil)
	solanaClient.EXPECT().GetTokenAccountBalance(gomock.Any(), gomock.Any(), gomock.Any()).Return(&solanarpc.UiTokenAmount{
		Amount: "2000000", Decimals: 6,
	}, nil)
	solanaClient.EXPECT().GetNativeBalance(gomock.Any(), gomock.Any()).Return(&solanarpc.UiTokenAmount{
		Amount: "1000000", Decimals: 9,
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
		AumUsd: math.NewUintFromString("5000000"),
		SolanaBalances: []neutronclient.SolanaBalance{
			{Address: strategyPubKey.String(), Asset: jlpTokenPubKey.String(), Amount: math.NewUintFromString("2000000")},
			{Address: strategyPubKey.String(), Asset: "SOL", Amount: math.NewUintFromString("1000000")},
		},
		SolanaTokenTotalSupply: []neutronclient.SolanaTokenTotalSupply{
			{Asset: jlpTokenPubKey.String(), TotalSupply: math.NewUintFromString("10000000")},
		},
		SolanaTokenDecimals: []neutronclient.SolanaTokenDecimals{
			{Asset: jlpTokenPubKey.String(), Decimals: 6},
			{Asset: "SOL", Decimals: 9},
		},
	}
	expectedData.Organize()
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
