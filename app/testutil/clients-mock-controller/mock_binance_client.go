package clients_mock_controller

import (
	"context"
	"encoding/json"
	"fmt"
	"sync"
	"time"

	binance "github.com/adshao/go-binance/v2"
	binanceportfolio "github.com/adshao/go-binance/v2/portfolio"
)

type MockBinanceClient struct {
	mu               sync.RWMutex
	umPositions      []*binanceportfolio.UMPosition
	pmAccountInfo    *binanceportfolio.Account
	pmAccountBalance []*binanceportfolio.Balance
	spotAccountInfo  *binance.Account
	timeoutEnabled   bool
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

func (m *MockBinanceClient) GetUmPositions(ctx context.Context) ([]*binanceportfolio.UMPosition, error) {
	m.mu.RLock()
	cp := make([]*binanceportfolio.UMPosition, len(m.umPositions))
	for i, position := range m.umPositions {
		positionCp := *position
		cp[i] = &positionCp
	}
	m.mu.RUnlock()

	if m.timeoutEnabled {
		// Then simulate latency without holding the lock
		if err := withTimeout(ctx, 200*time.Second); err != nil {
			return nil, err
		}
	}

	return cp, nil
}

func (m *MockBinanceClient) SetUmPositions(positions []*binanceportfolio.UMPosition) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.umPositions = positions
}

func (m *MockBinanceClient) GetPMAccountInfo(ctx context.Context) (*binanceportfolio.Account, error) {
	m.mu.RLock()
	cp := *m.pmAccountInfo
	m.mu.RUnlock()

	if m.timeoutEnabled {
		// Then simulate latency without holding the lock
		if err := withTimeout(ctx, 200*time.Second); err != nil {
			return nil, err
		}
	}

	return &cp, nil
}

func (m *MockBinanceClient) SetPMAccountInfo(account *binanceportfolio.Account) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.pmAccountInfo = account
}

func (m *MockBinanceClient) GetPMAccountBalance(ctx context.Context) ([]*binanceportfolio.Balance, error) {
	cp := make([]*binanceportfolio.Balance, len(m.pmAccountBalance))
	for i, balance := range m.pmAccountBalance {
		balanceCp := *balance
		cp[i] = &balanceCp
	}

	if m.timeoutEnabled {
		// Then simulate latency without holding the lock
		if err := withTimeout(ctx, 200*time.Second); err != nil {
			return nil, err
		}
	}

	return cp, nil
}

func (m *MockBinanceClient) SetPMAccountBalance(balances []*binanceportfolio.Balance) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.pmAccountBalance = balances
}

func (m *MockBinanceClient) GetSpotAccountInfo(ctx context.Context) (*binance.Account, error) {
	// Acquire and copy under lock immediately
	m.mu.RLock()
	cp := *m.spotAccountInfo
	m.mu.RUnlock()

	if m.timeoutEnabled {
		// Then simulate latency without holding the lock
		if err := withTimeout(ctx, 200*time.Second); err != nil {
			return nil, err
		}
	}

	return &cp, nil
}

func (m *MockBinanceClient) SetSpotAccountInfo(account *binance.Account) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.spotAccountInfo = account
}

func (m *MockBinanceClient) EnableTimeout() {
	m.timeoutEnabled = true
}

func (m *MockBinanceClient) DisableTimeout() {
	m.timeoutEnabled = false
}
