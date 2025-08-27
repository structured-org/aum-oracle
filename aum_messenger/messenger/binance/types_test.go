package binance

import (
	"slices"
	"testing"

	binance "github.com/adshao/go-binance/v2"
	binanceportfolio "github.com/adshao/go-binance/v2/portfolio"
)

func TestFilterPositions(t *testing.T) {
	requiredSymbols := []string{"BTCUSDT", "ETHUSDT", "SOLUSDT"}

	tests := []struct {
		name        string
		positions   []*binanceportfolio.UMPosition
		expectedLen int
	}{
		{
			name: "filters positions with required symbols only",
			positions: []*binanceportfolio.UMPosition{
				{Symbol: "XRPUSDT"}, // should be filtered out
				{Symbol: "BTCUSDT"},
				{Symbol: "ETHUSDT"},
				{Symbol: "BNBUSDT"}, // should be filtered out
				{Symbol: "SOLUSDT"},
				{Symbol: "ADAUSDT"}, // should be filtered out
			},
			expectedLen: 3,
		},
		{
			name: "returns empty slice when no positions match",
			positions: []*binanceportfolio.UMPosition{
				{Symbol: "BNBUSDT"},
				{Symbol: "ADAUSDT"},
				{Symbol: "DOTUSDT"},
			},
			expectedLen: 0,
		},
		{
			name: "returns all positions when all match required symbols",
			positions: []*binanceportfolio.UMPosition{
				{Symbol: "BTCUSDT"},
				{Symbol: "ETHUSDT"},
				{Symbol: "SOLUSDT"},
			},
			expectedLen: 3,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := filterPositions(tt.positions, requiredSymbols)
			if len(result) != tt.expectedLen {
				t.Errorf("got %d positions, expected %d", len(result), tt.expectedLen)
			}

			// Verify that all returned positions have symbols in the required list
			for _, position := range result {
				if !slices.Contains(requiredSymbols, position.Symbol) {
					t.Errorf("got position with symbol %s, which is not in required symbols", position.Symbol)
				}
			}
		})
	}
}

func TestFilterBalances(t *testing.T) {
	requiredAssets := []string{"USDT", "BTC", "ETH", "SOL"}

	tests := []struct {
		name        string
		balances    []binance.Balance
		expectedLen int
	}{
		{
			name: "filters balances with required assets only",
			balances: []binance.Balance{
				{Asset: "XRP"}, // should be filtered out
				{Asset: "BTC"},
				{Asset: "ETH"},
				{Asset: "LTC"}, // should be filtered out
				{Asset: "USDT"},
				{Asset: "SOL"},
				{Asset: "BNB"}, // should be filtered out
			},
			expectedLen: 4,
		},
		{
			name: "returns empty slice when no balances match",
			balances: []binance.Balance{
				{Asset: "LTC"},
				{Asset: "BNB"},
				{Asset: "ADA"},
			},
			expectedLen: 0,
		},
		{
			name: "returns all balances when all match required assets",
			balances: []binance.Balance{
				{Asset: "BTC"},
				{Asset: "ETH"},
				{Asset: "USDT"},
				{Asset: "SOL"},
			},
			expectedLen: 4,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := filterBalances(tt.balances, requiredAssets)
			if len(result) != tt.expectedLen {
				t.Errorf("got %d balances, expected %d", len(result), tt.expectedLen)
			}

			// Verify that all returned balances have assets in the required list
			for _, balance := range result {
				if !slices.Contains(requiredAssets, balance.Asset) {
					t.Errorf("got balance with asset %s, which is not in required assets", balance.Asset)
				}
			}
		})
	}
}
