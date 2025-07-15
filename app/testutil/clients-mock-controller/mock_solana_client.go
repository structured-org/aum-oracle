package clients_mock_controller

import (
	"context"
	"encoding/json"
	"fmt"
	"sync"

	solana "github.com/gagliardetto/solana-go"
	solanarpc "github.com/gagliardetto/solana-go/rpc"
)

type MockSolanaClient struct {
	mu                  sync.RWMutex
	tokenSupply         *solanarpc.UiTokenAmount
	tokenAccountBalance map[solana.PublicKey]*solanarpc.UiTokenAmount // Map token address to balance
}

func NewMockSolanaClient() *MockSolanaClient {
	client := &MockSolanaClient{
		tokenAccountBalance: make(map[solana.PublicKey]*solanarpc.UiTokenAmount),
	}
	client.loadDefaultData()
	return client
}

func (m *MockSolanaClient) loadDefaultData() {
	m.mu.Lock()
	defer m.mu.Unlock()

	// Load tokenSupply
	tokenSupplyData, err := MockDataFolder.ReadFile("mock_data/solana/tokenSupply.json")
	if err != nil {
		panic(fmt.Sprintf("failed to load default tokenSupply: %v", err))
	}
	if err := json.Unmarshal(tokenSupplyData, &m.tokenSupply); err != nil {
		panic(fmt.Sprintf("failed to unmarshal default tokenSupply: %v", err))
	}

	tokenAccountBalanceData, err := MockDataFolder.ReadFile("mock_data/solana/tokenAccountBalance.json")
	if err != nil {
		panic(fmt.Sprintf("failed to load default tokenAccountBalance: %v", err))
	}

	var tokenAccountBalance solanarpc.UiTokenAmount
	if err := json.Unmarshal(tokenAccountBalanceData, &tokenAccountBalance); err != nil {
		panic(fmt.Sprintf("failed to unmarshal default tokenAccountBalance: %v", err))
	}
	dummyTokenKey, _ := solana.PublicKeyFromBase58("11111111111111111111111111111111") // Placeholder
	m.tokenAccountBalance[dummyTokenKey] = &tokenAccountBalance
}

func (m *MockSolanaClient) GetTokenSupply(_ context.Context, _ solana.PublicKey) (*solanarpc.UiTokenAmount, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	cp := *m.tokenSupply
	return &cp, nil
}

func (m *MockSolanaClient) SetTokenSupply(supply *solanarpc.UiTokenAmount) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.tokenSupply = supply
}

func (m *MockSolanaClient) GetTokenAccountBalance(_ context.Context, token solana.PublicKey, account solana.PublicKey) (*solanarpc.UiTokenAmount, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	if balance, ok := m.tokenAccountBalance[token]; ok { // Simplified: just check token
		cp := *balance
		return &cp, nil
	}
	// If the specific token/account combination isn't mocked, return the default loaded one if available.
	for _, balance := range m.tokenAccountBalance {
		cp := *balance
		return &cp, nil
	}
	return nil, fmt.Errorf("token account balance not found for token %s and account %s", token.String(), account.String())
}

func (m *MockSolanaClient) SetTokenAccountBalance(token solana.PublicKey, balance *solanarpc.UiTokenAmount) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.tokenAccountBalance[token] = balance
}
