package main

import (
	"flag"
	"log"
	"os"
	"time"

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
	// TwaerContract is the address of the TWAER contract where record_er will be executed
	TwaerContract string `yaml:"twaer_contract"`

	// RecordInterval is the interval between record_er executions (e.g., "30s", "1m", "5m")
	RecordInterval time.Duration `yaml:"record_interval"`

	// PublishInterval is the interval between publish_twaer executions (e.g., "30s", "1m", "5m")
	PublishInterval time.Duration `yaml:"publish_interval"`

	// Clients is the configuration for different clients
	Clients clientsConfig `yaml:"clients"`

	// LoggerLevel is the level of the logger
	LoggerLevel string `yaml:"logger_level"`
}

type clientsConfig struct {
	Neutron cosmosclient.Config `yaml:"neutron"`
}
