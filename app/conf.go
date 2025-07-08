package main

import (
	"flag"
	"log"
	"os"

	"gopkg.in/yaml.v3"
)

var configPath = flag.String("config", "config.yaml", "path to config file")

// readConfig reads app config from the config file.
func readConfig() *config {
	cfgFile, err := os.Open(*configPath)
	if err != nil {
		log.Fatalf("failed to read config file at %s: %v", *configPath, err)
	}
	defer cfgFile.Close()

	var cfg config
	if err := yaml.NewDecoder(cfgFile).Decode(&cfg); err != nil {
		log.Fatalf("failed to decode config file: %v", err)
	}
	return &cfg
}

type config struct {
	// SolanaRpcEndpoint is the endpoint of the Solana RPC.
	SolanaRpcEndpoint string `yaml:"solana_rpc_endpoint"`

	// BinanceApiKey is the API key for the Binance API.
	BinanceApiKey string `yaml:"binance_api_key"`
	// BinanceApiSecret is the API secret for the Binance API.
	BinanceApiSecret string `yaml:"binance_api_secret"`
	// BinanceUmPositionsList is the list of UM-positions to fetch from Binance.
	BinanceUmPositionsList []string `yaml:"binance_um_positions_list"`
	// BinanceSpotAssetsList is the list of spot assets to fetch from Binance.
	BinanceSpotAssetsList []string `yaml:"binance_spot_assets_list"`

	// JupiterCustodies is the token->programId mapping of Jupiter custodies.
	JupiterCustodies map[string]string `yaml:"jupiter_custodies"`
	// JupiterJlpToken is the Jupiter JLP token pubkey.
	JupiterJlpToken string `yaml:"jupiter_jlp_token"`
	// JupiterPool is the Jupiter pool pubkey.
	JupiterPool string `yaml:"jupiter_jlp_pool"`
	// JupiterStrategyAddress is the Jupiter strategy address pubkey.
	JupiterStrategyAddress string `yaml:"jupiter_strategy_address"`

	// LoggerLevel is the level of the logger.
	LoggerLevel string `yaml:"logger_level"`

	// MockClients tells does the oracle us mocked Solana, Jupiter and Binance client or real ones
	MockClients        bool `yaml:"mock_clients"`
	MockControllerPort int  `yaml:"mock_controller_port"`
}
