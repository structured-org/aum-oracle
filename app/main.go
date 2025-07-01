package main

import (
	"context"
	"log"
	"os"
	"os/signal"
	"sync"
	"syscall"
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

	solanaClient := solanaclient.NewClient(conf.SolanaRpcEndpoint)
	neutronClient, err := neutronclient.NewClient(logRegistry.Get(neutronClientContext))
	if err != nil {
		logger.Fatal("failed to create neutron client", zap.Error(err))
	}
	binanceClient := binanceclient.NewClient(conf.BinanceApiKey, conf.BinanceApiSecret)

	binanceOracleConfig := binanceoracle.Config{
		UmPositionsList: conf.BinanceUmPositionsList,
		SpotAssetsList:  conf.BinanceSpotAssetsList,
	}
	binanceOracle := binanceoracle.NewOracle(
		binanceClient,
		neutronClient,
		solanaClient,
		binanceOracleConfig,
		logRegistry.Get(binanceAumOracleContext),
	)

	jupiterConfig := solanaoracle.JupiterConfig{
		Custodies: jupiterCustodies,
		Token:     solana.MustPublicKeyFromBase58(conf.JupiterJlpToken),
		Pool:      solana.MustPublicKeyFromBase58(conf.JupiterPool),
		Strategy:  solana.MustPublicKeyFromBase58(conf.JupiterStrategyAddress),
	}
	solanaOracle := solanaoracle.NewOracle(
		solanaClient,
		neutronClient,
		jupiterConfig,
		logRegistry.Get(solanaAumOracleContext),
	)

	ctx, cancel := context.WithCancel(context.Background())
	wg := sync.WaitGroup{}

	wg.Add(1)
	go func() {
		defer wg.Done()
		binanceOracle.Run(ctx)
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()
		// shift oracles in time to avoid simultaneous prints to stdout at debug submission
		// TODO: remove when neutron client is implemented
		time.Sleep(10 * time.Second)
		go solanaOracle.Run(ctx)
	}()

	go func() {
		sigs := make(chan os.Signal, 1)
		signal.Notify(sigs, syscall.SIGINT, syscall.SIGTERM)

		s := <-sigs
		logger.Info("Received termination signal, gracefully shutting down...",
			zap.String("signal", s.String()))
		cancel()
	}()

	wg.Wait()
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
