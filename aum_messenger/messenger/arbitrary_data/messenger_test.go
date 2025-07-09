package arbitrary_data

import (
	"context"
	"testing"
	"time"

	solana "github.com/gagliardetto/solana-go"
	"github.com/golang/mock/gomock"
	msgrclient "github.com/structured-org/aum-messenger/client"
	msgr "github.com/structured-org/aum-messenger/messenger"
	mock_arbitrarydata "github.com/structured-org/aum-messenger/testutil/mocks/arbitrarydata-messenger"
	"go.uber.org/zap"
)

var (
	programId     = "3csuXZKah5rgpb8RiwX9XfjrMxcp3u1K9mBdCwL51spj"
	instanceKey   = solana.MustPublicKeyFromBase58("wczmirTBMa3tGisAeecZcGbzvgi6VAG4Gu8uamtHBf3")
	arbitraryData = []byte("arbitrary data")
	contract      = "arbitrary_contract"
	query         = "{\"get_round_info\":{}}"
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

	solanaClient := mock_arbitrarydata.NewMockSolanaClient(ctrl)
	neutronClient := mock_arbitrarydata.NewMockNeutronClient(ctrl)

	solanaClient.EXPECT().GetArbitraryNeutronContractDataNextRound(gomock.Any(), programId, instanceKey).Return(&msgrclient.NextRound{
		Round: 1, Timestamp: start + 2,
	}, nil)

	neutronClient.EXPECT().QueryArbitraryNeutronContract(gomock.Any(), contract, query).Return(&arbitraryData, nil)

	solanaClient.EXPECT().SubmitArbitraryNeutronContractData(gomock.Any(), solana.MustPublicKeyFromBase58(programId), instanceKey, solana.MustPublicKeyFromBase58(programId), &arbitraryData).Return(&msgrclient.NextRound{
		Round: 2, Timestamp: start + 12,
	}, nil)

	queryConfig := NeutronContractQuery{
		SolanaProgramID:        programId,
		SolanaInstanceKey:      instanceKey.String(),
		NeutronContractAddress: contract,
		Query:                  query,
		OraclesListContract:    "contract address",
	}

	o := NewArbitraryNeutronDataMessengerForSolana(
		solanaClient,
		neutronClient,
		queryConfig,
		zap.NewExample(),
	)
	ctx, cancel := context.WithCancel(context.Background())
	go msgr.RunMessenger(ctx, o, msgrOpConfig)

	time.Sleep(9 * time.Second)
	cancel()
}
