package main

import (
	"context"
	"log"
	"os"
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

// NewAlignedTicker returns a channel that ticks every 'n' duration,
// aligned to 'lastTickTime'.
func NewAlignedTicker(lastTickTime time.Time, n time.Duration) <-chan time.Time {
	tickChan := make(chan time.Time)

	go func() {
		defer close(tickChan)

		now := time.Now().UTC()
		elapsed := now.Sub(lastTickTime)

		// 1. Check if we are already "late" (Immediate Execution)
		if elapsed >= n {
			select {
			case tickChan <- now:
				// If we fired immediately, we reset the anchor to NOW.
				// This ensures the NEXT tick waits for the full 'n' duration,
				// rather than firing again instantly to catch up to a grid.
				lastTickTime = now
			}
		}

		// 2. Calculate alignment for the next tick
		// We recalculate elapsed in case we just updated lastTickTime above.
		elapsed = time.Since(lastTickTime)

		// If we just fired immediately, elapsed is ~0, so wait = n.
		// If we didn't fire, this calculates the remainder of the interval.
		wait := n - (elapsed % n)

		timer := time.NewTimer(wait)

		select {
		case t := <-timer.C:
			// Fire the first aligned tick
			select {
			case tickChan <- t:
			}
		}

		// 3. Switch to standard Ticker for long-term repeating
		ticker := time.NewTicker(n)

		for {
			select {
			case t := <-ticker.C:
				tickChan <- t
			}
		}
	}()

	return tickChan
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

	ctx := context.Background()

	logger.Info("Starting recorder service",
		zap.String("contract", conf.TwaerContract),
		zap.Duration("interval", conf.RecordInterval))

	erWindow, err := neutronClient.QueryERWindowInfo(ctx)
	if err != nil {
		logger.Fatal("failed to query er window info", zap.Error(err))
	}

	twaerInfo, err := neutronClient.QueryTwaerInfo(ctx)
	if err != nil {
		logger.Fatal("failed to query twaer info", zap.Error(err))
	}

	// Main loop
	recordErTicker := NewAlignedTicker(time.Unix(erWindow.WindowEnd, 0).UTC(), conf.RecordInterval)
	publishTicker := NewAlignedTicker(time.Unix(twaerInfo.PublishedAt, 0).UTC(), conf.PublishInterval)

	for {
		select {
		case <-ctx.Done():
			logger.Info("Recorder service stopped")
			return
		case <-recordErTicker:
			if err := neutronClient.RecordER(ctx); err != nil {
				logger.Error("Failed to execute record_er", zap.Error(err))
			}
		case <-publishTicker:
			if err := neutronClient.PublishTwaer(ctx); err != nil {
				logger.Error("Failed to execute publish_twaer", zap.Error(err))
			}
		}
	}
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
