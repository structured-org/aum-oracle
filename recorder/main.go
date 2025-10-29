package main

import (
	"context"
	"log"
	"os"
	"os/signal"
	"syscall"
	"time"

	sdk "github.com/cosmos/cosmos-sdk/types"
	neutronclient "github.com/structured-org/aum-messenger/pkg/client/neutron"
	"go.uber.org/zap"

	nlogger "github.com/neutron-org/neutron-logger"
)

const (
	mainContext          = "recorder"
	neutronClientContext = "neutron_client"

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

	// Create Neutron client
	neutronClient, err := neutronclient.NewClient(
		conf.Clients.Neutron,
		conf.TwaerContract,
		"", // we don't need jupiter contract
		"", // we don't need binance contract
		logRegistry.Get(neutronClientContext),
	)
	if err != nil {
		logger.Fatal("failed to create neutron client", zap.Error(err))
	}

	ctx, cancel := context.WithCancel(context.Background())

	// Handle graceful shutdown
	go func() {
		sigs := make(chan os.Signal, 1)
		signal.Notify(sigs, syscall.SIGINT, syscall.SIGTERM)

		s := <-sigs
		logger.Info("Received termination signal, gracefully shutting down...",
			zap.String("signal", s.String()))
		cancel()
	}()

	logger.Info("Starting recorder service",
		zap.String("contract", conf.TwaerContract),
		zap.Duration("interval", conf.RecordInterval))

	// Main loop
	ticker := time.NewTicker(conf.RecordInterval)
	defer ticker.Stop()

	// Execute immediately on startup
	if err := executeRecordER(ctx, neutronClient, logger); err != nil {
		logger.Error("Failed to execute record_er", zap.Error(err))
	}

	for {
		select {
		case <-ctx.Done():
			logger.Info("Recorder service stopped")
			return
		case <-ticker.C:
			if err := executeRecordER(ctx, neutronClient, logger); err != nil {
				logger.Error("Failed to execute record_er", zap.Error(err))
			}
		}
	}
}

// executeRecordER executes the record_er message on the TWAER contract
func executeRecordER(ctx context.Context, client *neutronclient.Client, logger *zap.Logger) error {
	logger.Info("Executing record_er")

	if err := client.RecordER(ctx); err != nil {
		return err
	}

	logger.Info("Successfully executed record_er")
	return nil
}

// initLogRegistry initializes loggers registry.
func initLogRegistry(logLevel string) *nlogger.Registry {
	err := os.Setenv("LOGGER_LEVEL", logLevel)
	if err != nil {
		log.Fatalf("failed to set logging level env var: %v", err)
	}

	logRegistry, err := nlogger.NewRegistry(
		mainContext,
		neutronClientContext,
	)
	if err != nil {
		log.Fatalf("couldn't initialize loggers registry: %s", err)
	}
	return logRegistry
}
