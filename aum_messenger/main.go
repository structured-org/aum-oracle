package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/signal"
	"sync"
	"syscall"

	sdk "github.com/cosmos/cosmos-sdk/types"
	"github.com/gagliardetto/solana-go"
	msgr "github.com/structured-org/aum-messenger/messenger"
	binancemsgr "github.com/structured-org/aum-messenger/messenger/binance"
	jupitermsgr "github.com/structured-org/aum-messenger/messenger/jupiter"
	binanceclient "github.com/structured-org/aum-messenger/pkg/client/binance"
	jupiterclient "github.com/structured-org/aum-messenger/pkg/client/jupiter"
	neutronclient "github.com/structured-org/aum-messenger/pkg/client/neutron"
	solanaclient "github.com/structured-org/aum-messenger/pkg/client/solana"
	clients_mocker "github.com/structured-org/aum-messenger/testutil/clients-mock-controller"
	"go.uber.org/zap"

	nlogger "github.com/neutron-org/neutron-logger"
)

const (
	mainContext           = "main"
	jupiterAumMsgrContext = "jupiter_aum_messenger"
	binanceAumMsgrContext = "binance_aum_messenger"
	neutronClientContext  = "neutron_client"

	chainBechAddressPrefix = "neutron"
	chainBechPubPrefix     = "neutronpub"
)

func init() {
	config := sdk.GetConfig()
	config.SetBech32PrefixForAccount(chainBechAddressPrefix, chainBechPubPrefix)
	config.Seal()
}

func main() {
	conf := readConfig()
	logRegistry := initLogRegistry(conf.LoggerLevel)
	logger := logRegistry.Get(mainContext)

	// auxiliary structs holding client definitions for DI into messengers
	// are populated with either real or mock clients depending on config
	var binanceMsgrForNeutronDeps struct {
		binanceClient binancemsgr.BinanceClient
		neutronClient binancemsgr.NeutronAumReceiverClient
	}
	var binanceMsgrForSolanaDeps struct {
		binanceClient binancemsgr.BinanceClient
		solanaClient  binancemsgr.SolanaAumReceiverClient
	}
	var jupiterMsgrForNeutronDeps struct {
		solanaClient  jupitermsgr.SolanaClient
		neutronClient jupitermsgr.NeutronAumReceiverClient
		jupiterClient jupitermsgr.JupiterClient
	}

	// real unmockable clients
	neutronClient, err := neutronclient.NewClient(
		conf.Clients.Neutron,
		"",
		conf.JupiterAumContract,
		conf.BinanceAumContract,
		logRegistry.Get(neutronClientContext),
	)
	if err != nil {
		logger.Fatal("failed to create neutron client", zap.Error(err))
	}
	solanaClient := solanaclient.NewClient(conf.Clients.Solana.RpcEndpoint)

	switch conf.MockClients {
	case true: // test run. populate deps with mock clients and run mock controller server
		mockController := clients_mocker.NewClientsMockController()

		binanceMockClient := mockController.GetMockBinanceClient()
		solanaMockClient := mockController.GetMockSolanaClient()
		jupiterMockClient := mockController.GetMockJupiterClient()

		binanceMsgrForNeutronDeps.binanceClient = binanceMockClient
		binanceMsgrForNeutronDeps.neutronClient = neutronClient

		binanceMsgrForSolanaDeps.binanceClient = binanceMockClient
		binanceMsgrForSolanaDeps.solanaClient = solanaClient

		jupiterMsgrForNeutronDeps.solanaClient = solanaMockClient
		jupiterMsgrForNeutronDeps.neutronClient = neutronClient
		jupiterMsgrForNeutronDeps.jupiterClient = jupiterMockClient

		go func() {
			if err := mockController.Start(conf.MockControllerPort); err != nil {
				panic(fmt.Sprintf("failed to start mock controller server: %v", err))
			}
		}()

	case false: // prod run. populate deps with real clients
		jupiterClient := jupiterclient.NewClient(conf.Clients.Solana.RpcEndpoint)
		binanceClient := binanceclient.NewClient(conf.Clients.Binance.ApiKey, conf.Clients.Binance.ApiSecret)

		binanceMsgrForNeutronDeps.binanceClient = binanceClient
		binanceMsgrForNeutronDeps.neutronClient = neutronClient

		binanceMsgrForSolanaDeps.binanceClient = binanceClient
		binanceMsgrForSolanaDeps.solanaClient = solanaClient

		jupiterMsgrForNeutronDeps.solanaClient = solanaClient
		jupiterMsgrForNeutronDeps.neutronClient = neutronClient
		jupiterMsgrForNeutronDeps.jupiterClient = jupiterClient
	}

	// Binance messengers
	binanceMsgrConfig := binancemsgr.Config{
		UmPositionsList: conf.BinanceUmPositionsList,
		SpotAssetsList:  conf.BinanceSpotAssetsList,
	}
	binanceMsgrForNeutron := binancemsgr.NewBinanceAumMessengerForNeutron(
		binanceMsgrForNeutronDeps.binanceClient,
		binanceMsgrForNeutronDeps.neutronClient,
		binanceMsgrConfig,
		logRegistry.Get(binanceAumMsgrContext),
	)
	binanceMsgrForSolana := binancemsgr.NewBinanceAumMessengerForSolana(
		binanceMsgrForSolanaDeps.binanceClient,
		binanceMsgrForSolanaDeps.solanaClient,
		binanceMsgrConfig,
		logRegistry.Get(binanceAumMsgrContext),
	)

	// Jupiter messengers
	jupiterCustodies := make(map[string]solana.PublicKey)
	for token, programId := range conf.JupiterCustodies {
		jupiterCustodies[token] = solana.MustPublicKeyFromBase58(programId)
	}
	jupiterConfig := jupitermsgr.JupiterConfig{
		Custodies: jupiterCustodies,
		Token:     solana.MustPublicKeyFromBase58(conf.JupiterJlpToken),
		Pool:      solana.MustPublicKeyFromBase58(conf.JupiterPool),
		Strategy:  solana.MustPublicKeyFromBase58(conf.JupiterStrategyAddress),
	}
	jupiterMsgrForNeutron := jupitermsgr.NewJupiterAumMessengerForNeutron(
		jupiterMsgrForNeutronDeps.solanaClient,
		jupiterMsgrForNeutronDeps.neutronClient,
		jupiterMsgrForNeutronDeps.jupiterClient,
		jupiterConfig,
		logRegistry.Get(jupiterAumMsgrContext),
	)

	ctx, cancel := context.WithCancel(context.Background())
	wg := sync.WaitGroup{}

	wg.Add(1)
	go func() {
		defer wg.Done()
		logger.Info("running binance messenger for neutron")
		msgr.RunMessenger(ctx, binanceMsgrForNeutron, conf.OperationalConfig)
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()
		logger.Info("running binance messenger for solana")
		msgr.RunMessenger(ctx, binanceMsgrForSolana, conf.OperationalConfig)
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()
		logger.Info("running jupiter messenger for neutron")
		msgr.RunMessenger(ctx, jupiterMsgrForNeutron, conf.OperationalConfig)
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
		jupiterAumMsgrContext,
		binanceAumMsgrContext,
		neutronClientContext,
	)
	if err != nil {
		log.Fatalf("couldn't initialize loggers registry: %s", err)
	}
	return logRegistry
}
