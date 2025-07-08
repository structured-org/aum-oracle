package clients_mock_controller

import (
	"embed"
	"encoding/json"
	"fmt"
	"io"
	"net/http"

	binance "github.com/adshao/go-binance/v2"
	binanceportfolio "github.com/adshao/go-binance/v2/portfolio"
	solana "github.com/gagliardetto/solana-go"
	solanarpc "github.com/gagliardetto/solana-go/rpc"
	jupiterclient "github.com/structured-org/aum-oracle/client/jupiter"
)

//go:embed mock_data
var MockDataFolder embed.FS

// ClientsMockController serves a special HTTP client that can control mock data for the mock clients
type ClientsMockController struct {
	binanceMockClient *MockBinanceClient
	jupiterMockClient *MockJupiterClient
	solanaMockClient  *MockSolanaClient
}

func NewClientsMockController() *ClientsMockController {
	return &ClientsMockController{
		binanceMockClient: NewMockBinanceClient(),
		jupiterMockClient: NewMockJupiterClient(),
		solanaMockClient:  NewMockSolanaClient(),
	}
}

// GetMockBinanceClient returns the singleton mock Binance client.
func (cm *ClientsMockController) GetMockBinanceClient() *MockBinanceClient {
	return cm.binanceMockClient
}

// GetMockJupiterClient returns the singleton mock Jupiter client.
func (cm *ClientsMockController) GetMockJupiterClient() *MockJupiterClient {
	return cm.jupiterMockClient
}

// GetMockSolanaClient returns the singleton mock Solana client.
func (cm *ClientsMockController) GetMockSolanaClient() *MockSolanaClient {
	return cm.solanaMockClient
}

func (cm *ClientsMockController) Start(port int) error {
	http.HandleFunc("/mock/binance/umpositions", cm.handleBinanceUmPositions)
	http.HandleFunc("/mock/binance/cmpositions", cm.handleBinanceCmPositions)
	http.HandleFunc("/mock/binance/pmaccountinfo", cm.handleBinancePMAccountInfo)
	http.HandleFunc("/mock/binance/pmaccountbalance", cm.handleBinancePMAccountBalance)
	http.HandleFunc("/mock/binance/spotaccountinfo", cm.handleBinanceSpotAccountInfo)

	http.HandleFunc("/mock/jupiter/custodyinfo", cm.handleJupiterCustodyInfo)
	http.HandleFunc("/mock/jupiter/poolinfo", cm.handleJupiterPoolInfo)

	http.HandleFunc("/mock/solana/tokensupply", cm.handleSolanaTokenSupply)
	http.HandleFunc("/mock/solana/tokenaccountbalance", cm.handleSolanaTokenAccountBalance)

	return http.ListenAndServe(fmt.Sprintf(":%d", port), nil)
}

func (cm *ClientsMockController) handleBinanceUmPositions(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Only POST method is allowed", http.StatusMethodNotAllowed)
		return
	}
	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "Error reading request body", http.StatusInternalServerError)
		return
	}
	var positions []*binanceportfolio.UMPosition
	if err := json.Unmarshal(body, &positions); err != nil {
		http.Error(w, "Error unmarshaling request body", http.StatusBadRequest)
		return
	}
	cm.binanceMockClient.SetUmPositions(positions)
	w.WriteHeader(http.StatusOK)
	fmt.Fprint(w, "Binance UM Positions updated successfully")
}

func (cm *ClientsMockController) handleBinanceCmPositions(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Only POST method is allowed", http.StatusMethodNotAllowed)
		return
	}
	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "Error reading request body", http.StatusInternalServerError)
		return
	}
	var positions []*binanceportfolio.CMPosition
	if err := json.Unmarshal(body, &positions); err != nil {
		http.Error(w, "Error unmarshaling request body", http.StatusBadRequest)
		return
	}
	cm.binanceMockClient.SetCmPositions(positions)
	w.WriteHeader(http.StatusOK)
	fmt.Fprint(w, "Binance CM Positions updated successfully")
}

func (cm *ClientsMockController) handleBinancePMAccountInfo(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Only POST method is allowed", http.StatusMethodNotAllowed)
		return
	}
	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "Error reading request body", http.StatusInternalServerError)
		return
	}
	var account binanceportfolio.Account
	if err := json.Unmarshal(body, &account); err != nil {
		http.Error(w, "Error unmarshaling request body", http.StatusBadRequest)
		return
	}
	cm.binanceMockClient.SetPMAccountInfo(&account)
	w.WriteHeader(http.StatusOK)
	fmt.Fprint(w, "Binance PM Account Info updated successfully")
}

func (cm *ClientsMockController) handleBinancePMAccountBalance(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Only POST method is allowed", http.StatusMethodNotAllowed)
		return
	}
	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "Error reading request body", http.StatusInternalServerError)
		return
	}
	var balances []*binanceportfolio.Balance
	if err := json.Unmarshal(body, &balances); err != nil {
		http.Error(w, "Error unmarshaling request body", http.StatusBadRequest)
		return
	}
	cm.binanceMockClient.SetPMAccountBalance(balances)
	w.WriteHeader(http.StatusOK)
	fmt.Fprint(w, "Binance PM Account Balance updated successfully")
}

func (cm *ClientsMockController) handleBinanceSpotAccountInfo(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Only POST method is allowed", http.StatusMethodNotAllowed)
		return
	}
	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "Error reading request body", http.StatusInternalServerError)
		return
	}
	var account binance.Account
	if err := json.Unmarshal(body, &account); err != nil {
		http.Error(w, "Error unmarshaling request body", http.StatusBadRequest)
		return
	}
	cm.binanceMockClient.SetSpotAccountInfo(&account)
	w.WriteHeader(http.StatusOK)
	fmt.Fprint(w, "Binance Spot Account Info updated successfully")
}

func (cm *ClientsMockController) handleJupiterCustodyInfo(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Only POST method is allowed", http.StatusMethodNotAllowed)
		return
	}
	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "Error reading request body", http.StatusInternalServerError)
		return
	}
	var req struct {
		PublicKey string                                   `json:"publicKey"`
		Data      jupiterclient.JupiterPerpsCustodyAccount `json:"data"`
	}
	if err := json.Unmarshal(body, &req); err != nil {
		http.Error(w, "Error unmarshaling request body", http.StatusBadRequest)
		return
	}
	pubKey, err := solana.PublicKeyFromBase58(req.PublicKey)
	if err != nil {
		http.Error(w, "Invalid public key", http.StatusBadRequest)
		return
	}
	cm.jupiterMockClient.SetJupiterCustodyInfo(pubKey, &req.Data)
	w.WriteHeader(http.StatusOK)
	fmt.Fprint(w, "Jupiter Custody Info updated successfully")
}

func (cm *ClientsMockController) handleJupiterPoolInfo(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Only POST method is allowed", http.StatusMethodNotAllowed)
		return
	}
	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "Error reading request body", http.StatusInternalServerError)
		return
	}
	var poolInfo jupiterclient.JupiterPoolAccount
	if err := json.Unmarshal(body, &poolInfo); err != nil {
		http.Error(w, "Error unmarshaling request body", http.StatusBadRequest)
		return
	}
	cm.jupiterMockClient.SetJupiterPoolInfo(&poolInfo)
	w.WriteHeader(http.StatusOK)
	fmt.Fprint(w, "Jupiter Pool Info updated successfully")
}

func (cm *ClientsMockController) handleSolanaTokenSupply(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Only POST method is allowed", http.StatusMethodNotAllowed)
		return
	}
	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "Error reading request body", http.StatusInternalServerError)
		return
	}
	var tokenSupply solanarpc.UiTokenAmount
	if err := json.Unmarshal(body, &tokenSupply); err != nil {
		http.Error(w, "Error unmarshaling request body", http.StatusBadRequest)
		return
	}
	cm.solanaMockClient.SetTokenSupply(&tokenSupply)
	w.WriteHeader(http.StatusOK)
	fmt.Fprint(w, "Solana Token Supply updated successfully")
}

func (cm *ClientsMockController) handleSolanaTokenAccountBalance(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Only POST method is allowed", http.StatusMethodNotAllowed)
		return
	}
	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "Error reading request body", http.StatusInternalServerError)
		return
	}
	var req struct {
		PublicKey string                  `json:"publicKey"` // This would be the token's public key
		Data      solanarpc.UiTokenAmount `json:"data"`
	}
	if err := json.Unmarshal(body, &req); err != nil {
		http.Error(w, "Error unmarshaling request body", http.StatusBadRequest)
		return
	}
	pubKey, err := solana.PublicKeyFromBase58(req.PublicKey)
	if err != nil {
		http.Error(w, "Invalid public key", http.StatusBadRequest)
		return
	}
	cm.solanaMockClient.SetTokenAccountBalance(pubKey, &req.Data)
	w.WriteHeader(http.StatusOK)
	fmt.Fprint(w, "Solana Token Account Balance updated successfully")
}
