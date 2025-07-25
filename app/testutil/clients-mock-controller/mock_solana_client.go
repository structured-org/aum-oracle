package clients_mock_controller

import (
	"context"
	"encoding/json"
	"fmt"
	"sync"
	"time"

	"github.com/gagliardetto/solana-go"
	solanarpc "github.com/gagliardetto/solana-go/rpc"
)

type MockSolanaClient struct {
	mu                  sync.RWMutex
	tokenSupply         *solanarpc.UiTokenAmount
	tokenAccountBalance map[solana.PublicKey]*solanarpc.UiTokenAmount // Map token address to balance
	timeoutEnabled      bool
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

func (m *MockSolanaClient) GetTokenSupply(ctx context.Context, _ solana.PublicKey) (*solanarpc.UiTokenAmount, error) {
	var cp *solanarpc.UiTokenAmount
	func() {
		m.mu.RLock()
		defer m.mu.RUnlock()
		cp = m.tokenSupply
	}()

	fmt.Printf("#GetTokenSupply m.timeoutEnabled %v\n", m.timeoutEnabled)
	if m.timeoutEnabled {
		// Then simulate latency without holding the lock
		if err := withTimeout(ctx, 200*time.Second); err != nil {
			return nil, err
		}
	}

	return cp, nil
}

func (m *MockSolanaClient) SetTokenSupply(supply *solanarpc.UiTokenAmount) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.tokenSupply = supply
}

func (m *MockSolanaClient) GetTokenAccountBalance(ctx context.Context, token solana.PublicKey, account solana.PublicKey) (*solanarpc.UiTokenAmount, error) {
	var res *solanarpc.UiTokenAmount

	func() {
		// Acquire and copy under lock immediately
		m.mu.RLock()
		defer m.mu.RUnlock()
		if balance, ok := m.tokenAccountBalance[token]; ok { // Simplified: just check token
			res = balance
		}
		// If the specific token/account combination isn't mocked, return the default loaded one if available.
		if res == nil {
			for _, balance := range m.tokenAccountBalance {
				res = balance
				break
			}
		}
	}()
	
	fmt.Printf("#GetTokenAccountBalance m.timeoutEnabled %v\n", m.timeoutEnabled)
	if m.timeoutEnabled {
		// Then simulate latency without holding the lock
		if err := withTimeout(ctx, 200*time.Second); err != nil {
			return nil, err
		}
	}

	if res != nil {
		return res, nil
	}

	return nil, fmt.Errorf("token account balance not found for token %s and account %s", token.String(), account.String())
}

func (m *MockSolanaClient) SetTokenAccountBalance(token solana.PublicKey, balance *solanarpc.UiTokenAmount) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.tokenAccountBalance[token] = balance
}

func (m *MockSolanaClient) EnableTimeout() {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.timeoutEnabled = true
}

func (m *MockSolanaClient) DisableTimeout() {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.timeoutEnabled = false
}
