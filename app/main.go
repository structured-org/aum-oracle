package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

	"github.com/gagliardetto/solana-go"
	binanceclient "github.com/structured-org/aum-oracle/client/binance"
	jupiterclient "github.com/structured-org/aum-oracle/client/jupiter"
	neutronclient "github.com/structured-org/aum-oracle/client/neutron"
	solanaclient "github.com/structured-org/aum-oracle/client/solana"
	"github.com/structured-org/aum-oracle/oracle"
	binanceoracle "github.com/structured-org/aum-oracle/oracle/binance"
	jupiteroracle "github.com/structured-org/aum-oracle/oracle/jupiter"
	clients_mocker "github.com/structured-org/aum-oracle/testutil/clients-mock-controller"
	"go.uber.org/zap"

	nlogger "github.com/neutron-org/neutron-logger"
)

var (
	mainContext             = "main"
	jupiterAumOracleContext = "jupiter_aum_oracle"
	binanceAumOracleContext = "binance_aum_oracle"
	neutronClientContext    = "neutron_client"
)

func main() {
	conf := readConfig()
	logRegistry := initLogRegistry(conf.LoggerLevel)
	logger := logRegistry.Get(mainContext)
	logger.Info("app config", zap.Any("config", conf))

	// auxiliary structs holding client definitions for DI into oracles
	// are populated with either real or mock clients depending on config
	var binanceOracleForNeutronDeps struct {
		binanceClient binanceoracle.BinanceClient
		neutronClient binanceoracle.NeutronAumContractClient
	}
	var binanceOracleForSolanaDeps struct {
		binanceClient binanceoracle.BinanceClient
		solanaClient  binanceoracle.SolanaAumContractClient
	}
	var jupiterOracleForNeutronDeps struct {
		solanaClient  jupiteroracle.SolanaClient
		neutronClient jupiteroracle.NeutronAumContractClient
		jupiterClient jupiteroracle.JupiterClient
	}

	// real unmockable clients
	neutronClient, err := neutronclient.NewClient(logRegistry.Get(neutronClientContext))
	if err != nil {
		logger.Fatal("failed to create neutron client", zap.Error(err))
	}
	solanaClient := solanaclient.NewClient(conf.SolanaRpcEndpoint)

	switch conf.MockClients {
	case true: // test run. populate deps with mock clients and run mock controller server
		mockController := clients_mocker.NewClientsMockController()

		binanceMockClient := mockController.GetMockBinanceClient()
		solanaMockClient := mockController.GetMockSolanaClient()
		jupiterMockClient := mockController.GetMockJupiterClient()

		binanceOracleForNeutronDeps.binanceClient = binanceMockClient
		binanceOracleForNeutronDeps.neutronClient = neutronClient

		binanceOracleForSolanaDeps.binanceClient = binanceMockClient
		binanceOracleForSolanaDeps.solanaClient = solanaClient

		jupiterOracleForNeutronDeps.solanaClient = solanaMockClient
		jupiterOracleForNeutronDeps.neutronClient = neutronClient
		jupiterOracleForNeutronDeps.jupiterClient = jupiterMockClient

		go func() {
			if err := mockController.Start(conf.MockControllerPort); err != nil {
				panic(fmt.Sprintf("failed to start mock controller server: %v", err))
			}
		}()

	case false: // prod run. populate deps with real clients
		jupiterClient := jupiterclient.NewClient(conf.SolanaRpcEndpoint)
		binanceClient := binanceclient.NewClient(conf.BinanceApiKey, conf.BinanceApiSecret)

		binanceOracleForNeutronDeps.binanceClient = binanceClient
		binanceOracleForNeutronDeps.neutronClient = neutronClient

		binanceOracleForSolanaDeps.binanceClient = binanceClient
		binanceOracleForSolanaDeps.solanaClient = solanaClient

		jupiterOracleForNeutronDeps.solanaClient = solanaClient
		jupiterOracleForNeutronDeps.neutronClient = neutronClient
		jupiterOracleForNeutronDeps.jupiterClient = jupiterClient
	}

	// Binance oracles
	binanceOracleConfig := binanceoracle.Config{
		UmPositionsList: conf.BinanceUmPositionsList,
		SpotAssetsList:  conf.BinanceSpotAssetsList,
	}
	binanceOracleForNeutron := binanceoracle.NewBinanceAumOracleForNeutron(
		binanceOracleForNeutronDeps.binanceClient,
		binanceOracleForNeutronDeps.neutronClient,
		binanceOracleConfig,
		logRegistry.Get(binanceAumOracleContext),
	)
	binanceOracleForSolana := binanceoracle.NewBinanceAumOracleForSolana(
		binanceOracleForSolanaDeps.binanceClient,
		binanceOracleForSolanaDeps.solanaClient,
		binanceOracleConfig,
		logRegistry.Get(binanceAumOracleContext),
	)

	// Jupiter oracles
	jupiterCustodies := make(map[string]solana.PublicKey)
	for token, programId := range conf.JupiterCustodies {
		jupiterCustodies[token] = solana.MustPublicKeyFromBase58(programId)
	}
	jupiterConfig := jupiteroracle.JupiterConfig{
		Custodies: jupiterCustodies,
		Token:     solana.MustPublicKeyFromBase58(conf.JupiterJlpToken),
		Pool:      solana.MustPublicKeyFromBase58(conf.JupiterPool),
		Strategy:  solana.MustPublicKeyFromBase58(conf.JupiterStrategyAddress),
	}
	jupiterOracleForNeutron := jupiteroracle.NewJupiterAumOracleForNeutron(
		jupiterOracleForNeutronDeps.solanaClient,
		jupiterOracleForNeutronDeps.neutronClient,
		jupiterOracleForNeutronDeps.jupiterClient,
		jupiterConfig,
		logRegistry.Get(jupiterAumOracleContext),
	)

	ctx, cancel := context.WithCancel(context.Background())
	wg := sync.WaitGroup{}

	wg.Add(1)
	go func() {
		defer wg.Done()
		oracle.RunOracle(ctx, binanceOracleForNeutron)
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()
		// shift oracles in time to avoid simultaneous prints to stdout at debug submission
		// TODO: remove when oracles and clients are fully implemented
		time.Sleep(10 * time.Second)
		go oracle.RunOracle(ctx, binanceOracleForSolana)
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()
		// shift oracles in time to avoid simultaneous prints to stdout at debug submission
		// TODO: remove when oracles and clients are fully implemented
		time.Sleep(10 * time.Second)
		go oracle.RunOracle(ctx, jupiterOracleForNeutron)
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
		jupiterAumOracleContext,
		binanceAumOracleContext,
		neutronClientContext,
	)
	if err != nil {
		log.Fatalf("couldn't initialize loggers registry: %s", err)
	}
	return logRegistry
}
