package main

import (
	"flag"
	"log"
	"os"

	cosmosclient "github.com/structured-org/aum-messenger/client/cosmos"
	"github.com/structured-org/aum-messenger/messenger"
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
	// JupiterJlpToken is the Jupiter JLP token pubkey.
	JupiterJlpToken string `yaml:"jupiter_jlp_token"`
	// JupiterPool is the Jupiter pool pubkey.
	JupiterPool string `yaml:"jupiter_jlp_pool"`
	// JupiterStrategyAddress is the Jupiter strategy address pubkey.
	JupiterStrategyAddress string `yaml:"jupiter_strategy_address"`

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
	// SpotUrl is the spot url for the Binance API.
	SpotUrl string `yaml:"spot_url"`
	// PmUrl is the pm url for the Binance API.
	PmUrl string `yaml:"pm_url"`
	// BinanceApiKey is the API key for the Binance API.
	ApiKey string `yaml:"api_key"`
	// BinanceApiSecret is the API secret for the Binance API.
	ApiSecret string `yaml:"api_secret"`
}
