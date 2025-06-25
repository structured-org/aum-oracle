package main

import (
	"context"
	"log"
	"os"
	"time"

	"github.com/gagliardetto/solana-go"
	nlogger "github.com/neutron-org/neutron-logger"
	binanceoracle "github.com/structured-org/aum-oracle/binance"
	binanceclient "github.com/structured-org/aum-oracle/client/binance"
	neutronclient "github.com/structured-org/aum-oracle/client/neutron"
	solanaclient "github.com/structured-org/aum-oracle/client/solana"
	solanaoracle "github.com/structured-org/aum-oracle/solana"
	"go.uber.org/zap"
)

var (
	mainContext             = "main"
	solanaAumOracleContext  = "solana_aum_oracle"
	binanceAumOracleContext = "binance_aum_oracle"
	neutronClientContext    = "neutron_client"
)

func main() {
	conf := readConfig()
	logRegistry := initLogRegistry(conf.LoggerLevel)
	logger := logRegistry.Get(mainContext)
	logger.Info("app config", zap.Any("config", conf))

	jupiterCustodies := make(map[string]solana.PublicKey)
	for token, programId := range conf.JupiterCustodies {
		jupiterCustodies[token] = solana.MustPublicKeyFromBase58(programId)
	}
	jupiterConfig := solanaoracle.JupiterConfig{
		Custodies: jupiterCustodies,
		Token:     solana.MustPublicKeyFromBase58(conf.JupiterJlpToken),
		Pool:      solana.MustPublicKeyFromBase58(conf.JupiterPool),
		Strategy:  solana.MustPublicKeyFromBase58(conf.JupiterStrategyAddress),
	}

	solanaClient := solanaclient.NewClient(conf.SolanaRpcEndpoint)
	neutronClient, err := neutronclient.NewClient(logRegistry.Get(neutronClientContext))
	if err != nil {
		logger.Fatal("failed to create neutron client", zap.Error(err))
	}
	binanceClient := binanceclient.NewClient(conf.BinanceApiKey, conf.BinanceApiSecret)

	binanceOracle := binanceoracle.NewOracle(binanceClient, neutronClient, logRegistry.Get(binanceAumOracleContext))
	solanaOracle := solanaoracle.NewOracle(solanaClient, neutronClient, jupiterConfig, logRegistry.Get(solanaAumOracleContext))

	ctx := context.Background()
	go binanceOracle.Run(ctx)
	// shift oracles in time to avoid simultaneous prints to stdout at debug submission
	// TODO: remove when neutron client is implemented
	time.Sleep(10 * time.Second)
	go solanaOracle.Run(ctx)

	// run for 1 hour. TODO: add SIGINT SIGTERM handling
	time.Sleep(time.Minute * 60)
}

// initLogRegistry initializes loggers registry.
func initLogRegistry(logLevel string) *nlogger.Registry {
	err := os.Setenv("LOGGER_LEVEL", logLevel)
	if err != nil {
		log.Fatalf("failed to set logging level env var: %v", err)
	}

	logRegistry, err := nlogger.NewRegistry(
		mainContext,
		solanaAumOracleContext,
		binanceAumOracleContext,
		neutronClientContext,
	)
	if err != nil {
		log.Fatalf("couldn't initialize loggers registry: %s", err)
	}
	return logRegistry
}
