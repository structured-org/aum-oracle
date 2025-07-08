package clients_mock_controller

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"sync"

	binance "github.com/adshao/go-binance/v2"
	binanceportfolio "github.com/adshao/go-binance/v2/portfolio"
)

type MockBinanceClient struct {
	mu               sync.RWMutex
	umPositions      []*binanceportfolio.UMPosition
	cmPositions      []*binanceportfolio.CMPosition
	pmAccountInfo    *binanceportfolio.Account
	pmAccountBalance []*binanceportfolio.Balance
	spotAccountInfo  *binance.Account
}

func NewMockBinanceClient() *MockBinanceClient {
	client := &MockBinanceClient{}
	client.loadDefaultData()
	return client
}

func (m *MockBinanceClient) loadDefaultData() {
	m.mu.Lock()
	defer m.mu.Unlock()

	// Load umPositions
	umPositionsData, err := os.ReadFile("testutil/clients-mock-controller/mock_data/binance/umPositions.json")
	if err != nil {
		panic(fmt.Sprintf("failed to load umPositions.json: %v", err))
	}
	if err = json.Unmarshal(umPositionsData, &m.umPositions); err != nil {
		panic(fmt.Sprintf("failed to load umPositions.json: %v", err))
	}

	// Load pmAccountInfo
	pmAccountInfoData, err := os.ReadFile("testutil/clients-mock-controller/mock_data/binance/pmAccountInfo.json")
	if err != nil {
		panic(fmt.Sprintf("failed to load pmAccountInfo.json: %v", err))
	}
	if err = json.Unmarshal(pmAccountInfoData, &m.pmAccountInfo); err != nil {
		panic(fmt.Sprintf("failed to load pmAccountInfo.json: %v", err))
	}

	// Load pmAccountBalance
	pmAccountBalanceData, err := os.ReadFile("testutil/clients-mock-controller/mock_data/binance/pmAccountBalance.json")
	if err != nil {
		panic(fmt.Sprintf("failed to load pmAccountBalance.json: %v", err))
	}
	if err = json.Unmarshal(pmAccountBalanceData, &m.pmAccountBalance); err != nil {
		panic(fmt.Sprintf("failed to load pmAccountBalance.json: %v", err))
	}

	// Load spotAccountInfo
	spotAccountInfoData, err := os.ReadFile("testutil/clients-mock-controller/mock_data/binance/spotAccountInfo.json")
	if err != nil {
		panic(fmt.Sprintf("failed to load spotAccountInfo.json: %v", err))
	}
	if err = json.Unmarshal(spotAccountInfoData, &m.spotAccountInfo); err != nil {
		panic(fmt.Sprintf("failed to load spotAccountInfo.json: %v", err))
	}
}

func (m *MockBinanceClient) GetUmPositions(_ context.Context) ([]*binanceportfolio.UMPosition, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.umPositions, nil
}

func (m *MockBinanceClient) SetUmPositions(positions []*binanceportfolio.UMPosition) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.umPositions = positions
}

func (m *MockBinanceClient) GetCmPositions(_ context.Context) ([]*binanceportfolio.CMPosition, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.cmPositions, nil
}

func (m *MockBinanceClient) SetCmPositions(positions []*binanceportfolio.CMPosition) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.cmPositions = positions
}

func (m *MockBinanceClient) GetPMAccountInfo(_ context.Context) (*binanceportfolio.Account, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.pmAccountInfo, nil
}

func (m *MockBinanceClient) SetPMAccountInfo(account *binanceportfolio.Account) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.pmAccountInfo = account
}

func (m *MockBinanceClient) GetPMAccountBalance(_ context.Context) ([]*binanceportfolio.Balance, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.pmAccountBalance, nil
}

func (m *MockBinanceClient) SetPMAccountBalance(balances []*binanceportfolio.Balance) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.pmAccountBalance = balances
}

func (m *MockBinanceClient) GetSpotAccountInfo(_ context.Context) (*binance.Account, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.spotAccountInfo, nil
}

func (m *MockBinanceClient) SetSpotAccountInfo(account *binance.Account) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.spotAccountInfo = account
}
