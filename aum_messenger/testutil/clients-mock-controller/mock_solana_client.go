package clients_mock_controller

import (
	"context"
	"encoding/json"
	"fmt"
	"sync"
	"time"

	"github.com/gagliardetto/solana-go"
	solanatoken "github.com/gagliardetto/solana-go/programs/token"
	solanarpc "github.com/gagliardetto/solana-go/rpc"
)

type MockSolanaClient struct {
	mu                  sync.RWMutex
	tokenMint           *solanatoken.Mint
	nativeBalance       *solanarpc.UiTokenAmount
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

	// Load tokenMint
	tokenMintData, err := MockDataFolder.ReadFile("mock_data/solana/tokenMint.json")
	if err != nil {
		panic(fmt.Sprintf("failed to load default tokenMint: %v", err))
	}
	if err := json.Unmarshal(tokenMintData, &m.tokenMint); err != nil {
		panic(fmt.Sprintf("failed to unmarshal default tokenMint: %v", err))
	}

	// Load tokenAccountBalance
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

	// Load nativeBalance
	nativeBalanceData, err := MockDataFolder.ReadFile("mock_data/solana/nativeAccountBalance.json")
	if err != nil {
		panic(fmt.Sprintf("failed to load default nativeBalance: %v", err))
	}
	if err := json.Unmarshal(nativeBalanceData, &m.nativeBalance); err != nil {
		panic(fmt.Sprintf("failed to unmarshal default nativeBalance: %v", err))
	}
}

func (m *MockSolanaClient) GetTokenMint(ctx context.Context, _ solana.PublicKey) (*solanatoken.Mint, error) {
	var cp *solanatoken.Mint
	func() {
		m.mu.RLock()
		defer m.mu.RUnlock()
		cp = m.tokenMint
	}()

	if m.timeoutEnabled {
		// Then simulate latency without holding the lock
		if err := withTimeout(ctx, 1*time.Hour); err != nil {
			return nil, err
		}
	}

	return cp, nil
}

func (m *MockSolanaClient) SetTokenMint(tokenMint *solanatoken.Mint) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.tokenMint = tokenMint
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

	if m.timeoutEnabled {
		// Then simulate latency without holding the lock
		if err := withTimeout(ctx, 1*time.Hour); err != nil {
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

func (m *MockSolanaClient) GetNativeBalance(ctx context.Context, _ solana.PublicKey) (*solanarpc.UiTokenAmount, error) {
	var res *solanarpc.UiTokenAmount

	func() {
		m.mu.RLock()
		defer m.mu.RUnlock()
		res = m.nativeBalance
	}()

	if m.timeoutEnabled {
		// Then simulate latency without holding the lock
		if err := withTimeout(ctx, 1*time.Hour); err != nil {
			return nil, err
		}
	}

	return res, nil
}

func (m *MockSolanaClient) SetNativeBalance(nativeBalance *solanarpc.UiTokenAmount) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.nativeBalance = nativeBalance
}
