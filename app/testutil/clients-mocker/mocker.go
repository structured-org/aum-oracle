package clients_mocker

import (
	"context"
	"sync"

	binance2 "github.com/adshao/go-binance/v2"
	"github.com/adshao/go-binance/v2/portfolio"
	"github.com/gagliardetto/solana-go"
	"github.com/gagliardetto/solana-go/rpc"
	"github.com/golang/mock/gomock"
	jupiterclient "github.com/structured-org/aum-oracle/client/jupiter"
	"github.com/structured-org/aum-oracle/oracle/binance"
	"github.com/structured-org/aum-oracle/oracle/jupiter"
	mock_binance "github.com/structured-org/aum-oracle/testutil/mocks/binance-oracle"
	mock_jupiter "github.com/structured-org/aum-oracle/testutil/mocks/jupiter-oracle"
)

type TestReporter struct{}

func (tr *TestReporter) Errorf(format string, args ...interface{}) {}
func (tr *TestReporter) Fatalf(format string, args ...interface{}) {}

type Mocker struct {
	BinanceMockClient mock_binance.MockBinanceClient
	JupiterMockClient mock_jupiter.MockJupiterClient
	SolanaMockClient  mock_jupiter.MockSolanaClient

	sync.RWMutex
	binanceMockData map[string]interface{}
	solanaMockData  map[string]interface{}
	jupiterMockData map[string]map[string]interface{}
}

func NewMocker() *Mocker {
	m := &Mocker{
		BinanceMockClient: *mock_binance.NewMockBinanceClient(gomock.NewController(&TestReporter{})),
		JupiterMockClient: *mock_jupiter.NewMockJupiterClient(gomock.NewController(&TestReporter{})),
		SolanaMockClient:  *mock_jupiter.NewMockSolanaClient(gomock.NewController(&TestReporter{})),
	}

	m.BinanceMockClient.EXPECT().GetUmPositions(gomock.Any()).DoAndReturn(func(_ context.Context) ([]*portfolio.UMPosition, error) {
		m.RLock()
		return m.binanceMockData["umPositions"].([]*portfolio.UMPosition), nil
	})

	m.BinanceMockClient.EXPECT().GetPMAccountBalance(gomock.Any()).DoAndReturn(func(_ context.Context) ([]*portfolio.Balance, error) {
		m.RLock()
		return m.binanceMockData["pmAccountBalance"].([]*portfolio.Balance), nil
	})

	m.BinanceMockClient.EXPECT().GetPMAccountInfo(gomock.Any()).DoAndReturn(func(_ context.Context) (*portfolio.Account, error) {
		m.RLock()
		return m.binanceMockData["pmAccountInfo"].(*portfolio.Account), nil
	})

	m.BinanceMockClient.EXPECT().GetSpotAccountInfo(gomock.Any()).DoAndReturn(func(_ context.Context) (*binance2.Account, error) {
		m.RLock()
		return m.binanceMockData["spotAccountInfo"].(*binance2.Account), nil
	})

	m.SolanaMockClient.EXPECT().GetTokenSupply(gomock.Any(), gomock.Any()).DoAndReturn(func(_ context.Context, _ solana.PublicKey) (*rpc.UiTokenAmount, error) {
		m.RLock()
		return m.solanaMockData["tokenSupply"].(*rpc.UiTokenAmount), nil
	})

	m.SolanaMockClient.EXPECT().GetTokenSupply(gomock.Any(), gomock.Any()).DoAndReturn(func(_ context.Context, _ solana.PublicKey) (*rpc.UiTokenAmount, error) {
		m.RLock()
		return m.solanaMockData["accountBalance"].(*rpc.UiTokenAmount), nil
	})

	m.JupiterMockClient.EXPECT().GetJupiterCustodyInfo(gomock.Any(), gomock.Any()).DoAndReturn(func(_ context.Context, custody solana.PublicKey) (*jupiterclient.JupiterPerpsCustodyAccount, error) {
		m.RLock()
		return m.jupiterMockData["custody"][custody.String()].(*jupiterclient.JupiterPerpsCustodyAccount), nil
	})

	m.JupiterMockClient.EXPECT().GetJupiterPoolInfo(gomock.Any(), gomock.Any()).DoAndReturn(func(_ context.Context, pool solana.PublicKey) (*jupiterclient.JupiterPoolAccount, error) {
		m.RLock()
		return m.jupiterMockData["custody"][pool.String()].(*jupiterclient.JupiterPoolAccount), nil
	})

	return m
}

func (m *Mocker) BinanceClient() binance.BinanceClient {
	return &m.BinanceMockClient
}

func (m *Mocker) SolanaClient() jupiter.SolanaClient {
	return &m.SolanaMockClient
}

func (m *Mocker) JupiterClient() jupiter.JupiterClient {
	return &m.JupiterMockClient
}
