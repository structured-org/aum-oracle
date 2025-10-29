package clients_mock_controller

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"
	"sync"

	solana "github.com/gagliardetto/solana-go"
	jupiterclient "github.com/structured-org/aum-messenger/pkg/client/jupiter"
)

type MockJupiterClient struct {
	mu          sync.RWMutex
	custodyInfo map[solana.PublicKey]*jupiterclient.JupiterPerpsCustodyAccount
	poolInfo    *jupiterclient.JupiterPoolAccount
}

func NewMockJupiterClient() *MockJupiterClient {
	client := &MockJupiterClient{
		custodyInfo: make(map[solana.PublicKey]*jupiterclient.JupiterPerpsCustodyAccount),
	}
	client.loadDefaultData()
	return client
}

func (m *MockJupiterClient) loadDefaultData() {
	m.mu.Lock()
	defer m.mu.Unlock()

	// Load poolInfo
	poolInfoData, err := MockDataFolder.ReadFile("mock_data/jupiter/pool.json")
	if err != nil {
		panic(fmt.Sprintf("failed to read mock_data/jupiter/pool.json: %v", err))
	}
	if err = json.Unmarshal(poolInfoData, &m.poolInfo); err != nil {
		panic(fmt.Sprintf("failed to unmarshal mock_data/jupiter/pool.json: %v", err.Error()))
	}

	custodyFiles := []string{
		"mock_data/jupiter/custody_4vkNeXiYEUizLdrpdPS1eC2mccyM4NUPRtERrk6ZETkk.json",
		"mock_data/jupiter/custody_5Pv3gM9JrFFH883SWAhvJC9RPYmo8UNxuFtv5bMMALkm.json",
		"mock_data/jupiter/custody_7xS2gz2bTp3fwCC7knJvUWTEU9Tycczu6VhJYKgi1wdz.json",
		"mock_data/jupiter/custody_AQCGyheWPLeo6Qp9WpYS9m3Qj479t7R636N9ey1rEjEn.json",
		"mock_data/jupiter/custody_G18jKKXQwBbrHeiK3C9MRXhkHsLHf7XgCSisykV46EZa.json",
	}

	for _, filePath := range custodyFiles {
		custodyData, err := MockDataFolder.ReadFile(filePath)
		if err != nil {
			panic(fmt.Sprintf("failed to load mock_data/jupiter/custody.json: %v", err.Error()))
		}
		var custodyAccount jupiterclient.JupiterPerpsCustodyAccount
		if err := json.Unmarshal(custodyData, &custodyAccount); err != nil {
			panic(fmt.Sprintf("failed to unmarshal custody data from %s: %v", filePath, err))
		}

		fileName := strings.TrimSuffix(strings.SplitAfter(filePath, "mock_data/jupiter/custody_")[1], ".json")
		pubKey, err := solana.PublicKeyFromBase58(fileName)
		if err != nil {
			panic(fmt.Sprintf("failed to parse public key from filename %s: %v", fileName, err))
		}
		m.custodyInfo[pubKey] = &custodyAccount
	}
}

func (m *MockJupiterClient) GetJupiterCustodyInfo(_ context.Context, custody solana.PublicKey) (*jupiterclient.JupiterPerpsCustodyAccount, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	if info, ok := m.custodyInfo[custody]; ok {
		cp := *info
		return &cp, nil
	}
	return nil, fmt.Errorf("custody info not found for %s", custody.String())
}

func (m *MockJupiterClient) SetJupiterCustodyInfo(custody solana.PublicKey, info *jupiterclient.JupiterPerpsCustodyAccount) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.custodyInfo[custody] = info
}

func (m *MockJupiterClient) GetJupiterPoolInfo(_ context.Context, _ solana.PublicKey) (*jupiterclient.JupiterPoolAccount, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	cp := *m.poolInfo
	return &cp, nil
}

func (m *MockJupiterClient) SetJupiterPoolInfo(info *jupiterclient.JupiterPoolAccount) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.poolInfo = info
}
