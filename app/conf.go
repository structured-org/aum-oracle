package main

import (
	"flag"
	"log"
	"os"

	"gopkg.in/yaml.v3"
)

var configPath = flag.String("config", "config.yaml", "path to config file")

// ReadConfig reads app config from the config file.
func ReadConfig() *Config {
	cfgFile, err := os.Open(*configPath)
	if err != nil {
		log.Fatalf("failed to read config file at %s: %v", *configPath, err)
	}
	defer cfgFile.Close()

	var cfg Config
	if err := yaml.NewDecoder(cfgFile).Decode(&cfg); err != nil {
		log.Fatalf("failed to decode config file: %v", err)
	}
	return &cfg
}

type Config struct {
	SolanaRpcEndpoint string `yaml:"solana_rpc_endpoint"`

	BinanceApiKey    string `yaml:"binance_api_key"`
	BinanceApiSecret string `yaml:"binance_api_secret"`

	JupiterCustodies       map[string]string `yaml:"jupiter_custodies"`
	JupiterJlpToken        string            `yaml:"jupiter_jlp_token"`
	JupiterPool            string            `yaml:"jupiter_jlp_pool"`
	JupiterStrategyAddress string            `yaml:"jupiter_strategy_address"`
}
