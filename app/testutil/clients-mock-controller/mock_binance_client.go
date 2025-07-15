package clients_mock_controller

import (
	"context"
	"encoding/json"
	"fmt"
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
	umPositionsData, err := MockDataFolder.ReadFile("mock_data/binance/umPositions.json")
	if err != nil {
		panic(fmt.Sprintf("failed to load umPositions.json: %v", err))
	}
	if err = json.Unmarshal(umPositionsData, &m.umPositions); err != nil {
		panic(fmt.Sprintf("failed to unmarshal umPositions.json: %v", err))
	}

	// Load pmAccountInfo
	pmAccountInfoData, err := MockDataFolder.ReadFile("mock_data/binance/pmAccountInfo.json")
	if err != nil {
		panic(fmt.Sprintf("failed to load pmAccountInfo.json: %v", err))
	}
	if err = json.Unmarshal(pmAccountInfoData, &m.pmAccountInfo); err != nil {
		panic(fmt.Sprintf("failed to unmarshal pmAccountInfo.json: %v", err))
	}

	// Load pmAccountBalance
	pmAccountBalanceData, err := MockDataFolder.ReadFile("mock_data/binance/pmAccountBalance.json")
	if err != nil {
		panic(fmt.Sprintf("failed to load pmAccountBalance.json: %v", err))
	}
	if err = json.Unmarshal(pmAccountBalanceData, &m.pmAccountBalance); err != nil {
		panic(fmt.Sprintf("failed to unmarshal pmAccountBalance.json: %v", err))
	}

	// Load spotAccountInfo
	spotAccountInfoData, err := MockDataFolder.ReadFile("mock_data/binance/spotAccountInfo.json")
	if err != nil {
		panic(fmt.Sprintf("failed to load spotAccountInfo.json: %v", err))
	}
	if err = json.Unmarshal(spotAccountInfoData, &m.spotAccountInfo); err != nil {
		panic(fmt.Sprintf("failed to unmarshal spotAccountInfo.json: %v", err))
	}
}

func (m *MockBinanceClient) GetUmPositions(_ context.Context) ([]*binanceportfolio.UMPosition, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	cp := make([]*binanceportfolio.UMPosition, len(m.umPositions))
	for i, position := range m.umPositions {
		positionCp := *position
		cp[i] = &positionCp
	}
	return cp, nil
}

func (m *MockBinanceClient) SetUmPositions(positions []*binanceportfolio.UMPosition) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.umPositions = positions
}

func (m *MockBinanceClient) GetCmPositions(_ context.Context) ([]*binanceportfolio.CMPosition, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	cp := make([]*binanceportfolio.CMPosition, len(m.cmPositions))
	for i, position := range m.cmPositions {
		positionCp := *position
		cp[i] = &positionCp
	}
	return cp, nil
}

func (m *MockBinanceClient) SetCmPositions(positions []*binanceportfolio.CMPosition) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.cmPositions = positions
}

func (m *MockBinanceClient) GetPMAccountInfo(_ context.Context) (*binanceportfolio.Account, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	cp := *m.pmAccountInfo
	return &cp, nil
}

func (m *MockBinanceClient) SetPMAccountInfo(account *binanceportfolio.Account) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.pmAccountInfo = account
}

func (m *MockBinanceClient) GetPMAccountBalance(_ context.Context) ([]*binanceportfolio.Balance, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	cp := make([]*binanceportfolio.Balance, len(m.pmAccountBalance))
	for i, balance := range m.pmAccountBalance {
		balanceCp := *balance
		cp[i] = &balanceCp
	}
	return cp, nil
}

func (m *MockBinanceClient) SetPMAccountBalance(balances []*binanceportfolio.Balance) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.pmAccountBalance = balances
}

func (m *MockBinanceClient) GetSpotAccountInfo(_ context.Context) (*binance.Account, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	cp := *m.spotAccountInfo
	return &cp, nil
}

func (m *MockBinanceClient) SetSpotAccountInfo(account *binance.Account) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.spotAccountInfo = account
}
