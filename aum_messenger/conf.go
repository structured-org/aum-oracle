package main

import (
	"flag"
	"fmt"
	"log"
	"os"

	"github.com/gagliardetto/solana-go"
	"github.com/structured-org/aum-messenger/messenger"
	cosmosclient "github.com/structured-org/aum-messenger/pkg/client/cosmos"
	solanaclient "github.com/structured-org/aum-messenger/pkg/client/solana"
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

	if err := cfg.Validate(); err != nil {
		log.Fatalf("config is invalid: %v", err)
	}
	return &cfg
}

type config struct {
	// BinanceUmPositionsList is the list of UM-positions to fetch from Binance.
	BinanceUmPositionsList []string `yaml:"binance_um_positions_list"`
	// BinanceSpotAssetsList is the list of spot assets to fetch from Binance.
	BinanceSpotAssetsList []string `yaml:"binance_spot_assets_list"`

	// JupiterCustodies is the human readable asset name to custody program ID mapping of Jupiter
	// custodies.
	JupiterCustodies map[string]string `yaml:"jupiter_custodies"`
	// JupiterPool is the Jupiter pool pubkey.
	JupiterPool string `yaml:"jupiter_jlp_pool"`
	// SolanaBalancesList is the list of owner to assets mappings of Solana balances. Owners are
	// represented as Solana addresses. Each asset is represented either as SPL token Mint address
	// or SOL for balance in lamports.
	SolanaBalancesList map[string][]string `yaml:"solana_balances_list"`
	// SolanaTokenSupplyList is the list of SPL token Mint addresses to fetch the total supply of.
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

func (c *config) Validate() error {
	if c.JupiterAumContract == "" {
		return fmt.Errorf("jupiter_aum_contract is required")
	}
	if c.BinanceAumContract == "" {
		return fmt.Errorf("binance_aum_contract is required")
	}

	for token, programId := range c.JupiterCustodies {
		if _, err := solana.PublicKeyFromBase58(programId); err != nil {
			return fmt.Errorf("jupiter_custodies %s custody program ID %s is invalid: %w", token, programId, err)
		}
	}
	if c.JupiterPool == "" {
		return fmt.Errorf("jupiter_jlp_pool is required")
	}
	for address, tokens := range c.SolanaBalancesList {
		if _, err := solana.PublicKeyFromBase58(address); err != nil {
			return fmt.Errorf("solana_balances_list owner address %s is invalid: %w", address, err)
		}
		for _, token := range tokens {
			if token == solanaclient.SolanaNativeTokenName {
				continue
			}
			if _, err := solana.PublicKeyFromBase58(token); err != nil {
				return fmt.Errorf(
					"solana_balances_list asset %s is invalid: %w. Make sure to only specify SPL token Mint addresses or SOL",
					token, err,
				)
			}
		}
	}
	for _, token := range c.SolanaTokenSupplyList {
		if _, err := solana.PublicKeyFromBase58(token); err != nil {
			return fmt.Errorf(
				"solana_token_supply_list entry %s is invalid: %w. Make sure to only specify SPL token Mint addresses",
				token, err,
			)
		}
	}

	if c.Clients.Neutron.Mnemonic == "" {
		return fmt.Errorf("neutron client mnemonic is required")
	}
	if c.Clients.Neutron.GasPrices == "" {
		return fmt.Errorf("neutron client gas_prices is required")
	}
	if c.Clients.Neutron.ChainID == "" {
		return fmt.Errorf("neutron client chain_id is required")
	}
	if c.Clients.Neutron.Node == "" {
		return fmt.Errorf("neutron client node is required")
	}

	if !c.MockClients {
		if c.Clients.Solana.RpcEndpoint == "" {
			return fmt.Errorf("solana client rpc_endpoint is required")
		}

		if c.Clients.Binance.ApiKey == "" {
			return fmt.Errorf("binance client api_key is required")
		}
		if c.Clients.Binance.ApiSecret == "" {
			return fmt.Errorf("binance client api_secret is required")
		}
	}

	if c.OperationalConfig.FetchDataTimeout == 0 {
		return fmt.Errorf("operational config fetch_data_timeout is required")
	}

	return nil
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
