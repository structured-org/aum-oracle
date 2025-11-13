package main

import (
	"flag"
	"log"
	"os"

	"github.com/structured-org/aum-messenger/messenger"
	cosmosclient "github.com/structured-org/aum-messenger/pkg/client/cosmos"
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
	// BinanceUmPositionsList is the list of UM-positions to fetch from Binance.
	BinanceUmPositionsList []string `yaml:"binance_um_positions_list"`
	// BinanceSpotAssetsList is the list of spot assets to fetch from Binance.
	BinanceSpotAssetsList []string `yaml:"binance_spot_assets_list"`

	// JupiterCustodies is the token->programId mapping of Jupiter custodies.
	JupiterCustodies map[string]string `yaml:"jupiter_custodies"`
	// JupiterPool is the Jupiter pool pubkey.
	JupiterPool string `yaml:"jupiter_jlp_pool"`
	// SolanaBalancesList is the list of address->tokens mappings of Solana balances.
	SolanaBalancesList map[string][]string `yaml:"solana_balances_list"`
	// SolanaTokenSupplyList is the list of Solana token addresses to fetch the total supply of.
	SolanaTokenSupplyList []string `yaml:"solana_token_supply_list"`

	// Clients is the configuration for different clients.
	Clients clientsConfig `yaml:"clients"`

	// Jupiter AUM Oracle Receiver contract address.
	JupiterAumContract string `yaml:"jupiter_aum_contract"`
	// Binance AUM Oracle Receiver contract address.
	BinanceAumContract string `yaml:"binance_aum_contract"`

	// OperationalConfig is the configuration for the messenger's operational parameters.
	OperationalConfig messenger.OperationalConfig `yaml:"operational_config"`

	// LoggerLevel is the level of the logger.
	LoggerLevel string `yaml:"logger_level"`

	// MockClients tells do the Oracle Messengers use mocked Solana, Jupiter and Binance client or
	// real ones.
	MockClients bool `yaml:"mock_clients"`
	// MockControllerPort is the port of the mock controller server.
	MockControllerPort int `yaml:"mock_controller_port"`
}

type clientsConfig struct {
	Neutron cosmosclient.Config `yaml:"neutron"`
	Solana  solanaConfig        `yaml:"solana"`
	Binance binanceConfig       `yaml:"binance"`
}

type solanaConfig struct {
	RpcEndpoint string `yaml:"rpc_endpoint"`
}

type binanceConfig struct {
	// BinanceApiKey is the API key for the Binance API.
	ApiKey string `yaml:"api_key"`
	// BinanceApiSecret is the API secret for the Binance API.
	ApiSecret string `yaml:"api_secret"`
}
