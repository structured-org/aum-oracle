package binance

import (
	"context"
	"slices"
	"sync"
	"time"

	"cosmossdk.io/math"
	neutronclient "github.com/structured-org/aum-oracle/client/neutron"
	"go.uber.org/zap"
)

// Config is the configuration for the Binance oracle.
type Config struct {
	// UmPositionsList is the list of USD-margined portfolio perpetual futures positions to query
	// information about.
	UmPositionsList []string
	// SpotAssetsList is the list of spot assets to query information about.
	SpotAssetsList []string
}

// Oracle is the Binance oracle. It is responsible for fetching data from Binance and submitting it
// to the Neutron client.
type Oracle struct {
	binanceClient BinanceClient
	neutronClient NeutronClient
	config        Config

	logger *zap.Logger
}

// NewOracle creates a new Binance oracle.
func NewOracle(
	binanceClient BinanceClient,
	neutronClient NeutronClient,
	config Config,
	logger *zap.Logger,
) *Oracle {
	return &Oracle{
		binanceClient: binanceClient,
		neutronClient: neutronClient,
		config:        config,
		logger:        logger,
	}
}

// Run runs the Binance oracle. It starts query-submission loop that periodically fetches data from
// Binance and submits it using the Neutron client.
func (o *Oracle) Run(ctx context.Context) {
	// query the next round once at initialisation
	// then the value is reassigned from submission response in the loop
	nextRound, err := o.neutronClient.GetBinanceAumContractNextRound(ctx)
	if err != nil {
		o.logger.Error("failed to get next round", zap.Error(err))
		return
	}

	for {
		timeTillNextRound := time.Duration(nextRound.Timestamp-time.Now().Unix()) * time.Second
		o.logger.Info("waiting for next round",
			zap.Int64("round", nextRound.Round),
			zap.Int64("round_timestamp", nextRound.Timestamp),
			zap.Duration("time_till_next_round", timeTillNextRound),
		)

		select {
		case <-time.NewTimer(timeTillNextRound).C:
			o.logger.Info("new round started",
				zap.Int64("round", nextRound.Round),
				zap.Int64("round_timestamp", nextRound.Timestamp),
			)

			data, err := o.fetchBinanceData(ctx)
			if err != nil {
				o.logger.Error("failed to fetch AUM data", zap.Error(err))
				return
			}
			data.Round = nextRound.Round

			nextRound, err = o.neutronClient.SubmitBinanceAumData(ctx, data)
			if err != nil {
				o.logger.Error("failed to submit AUM data", zap.Error(err))
				return
			}

			o.logger.Info("submitted AUM data",
				zap.Int64("round", data.Round),
				zap.Any("data", *data),
			)

		case <-ctx.Done():
			o.logger.Info("oracle stopped by context")
			return
		}
	}
}

// fetchBinanceData concurrently fetches all required data from Binance using the Binance client.
func (o *Oracle) fetchBinanceData(ctx context.Context) (*neutronclient.BinanceData, error) {
	data := &neutronclient.BinanceData{}
	wg := sync.WaitGroup{}

	wg.Add(1)
	go func() {
		defer wg.Done()

		positions, err := o.getUmPositions(ctx)
		if err != nil {
			o.logger.Error("failed to get UM positions", zap.Error(err))
			return
		}
		data.Positions = positions
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()

		balances, err := o.getSpotBalances(ctx)
		if err != nil {
			o.logger.Error("failed to get spot balances", zap.Error(err))
			return
		}
		data.SpotBalances = balances
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()

		pmAccount, err := o.binanceClient.GetPMAccountInfo(ctx)
		if err != nil {
			o.logger.Error("failed to get PM account info", zap.Error(err))
			return
		}
		data.PmAccountActualEquity = math.LegacyMustNewDecFromStr(pmAccount.ActualEquity)
		data.WithdrawableUsdt = math.LegacyMustNewDecFromStr(pmAccount.VirtualMaxWithdrawAmount)
		data.Unimmr = math.LegacyMustNewDecFromStr(pmAccount.UniMMR)
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()

		pmAccountBalances, err := o.binanceClient.GetPMAccountBalance(ctx)
		if err != nil {
			o.logger.Error("failed to get PM account balances", zap.Error(err))
			return
		}
		for _, balance := range pmAccountBalances {
			if balance.Asset == "USDT" {
				data.UmBalanceUsdt = math.LegacyMustNewDecFromStr(balance.UMWalletBalance)
			}
		}
	}()

	wg.Wait()
	return data, nil
}

// getUmPositions gets the USD-margined portfolio perpetual futures positions.
func (o *Oracle) getUmPositions(ctx context.Context) ([]neutronclient.BinancePosition, error) {
	umPositions, err := o.binanceClient.GetUmPositions(ctx)
	if err != nil {
		return nil, err
	}

	positions := make([]neutronclient.BinancePosition, 0)
	for _, position := range umPositions {
		if !slices.Contains(o.config.UmPositionsList, position.Symbol) {
			continue
		}

		positions = append(positions, neutronclient.BinancePosition{
			Symbol: position.Symbol,
			Amount: math.LegacyMustNewDecFromStr(position.PositionAmt),
			Pnl:    math.LegacyMustNewDecFromStr(position.UnrealizedProfit),
		})
	}

	return positions, nil
}

// getSpotBalances gets the spot balances.
func (o *Oracle) getSpotBalances(ctx context.Context) ([]neutronclient.BinanceBalance, error) {
	spotBalances, err := o.binanceClient.GetSpotAccountInfo(ctx)
	if err != nil {
		return nil, err
	}

	balances := make([]neutronclient.BinanceBalance, 0)
	for _, balance := range spotBalances.Balances {
		if !slices.Contains(o.config.SpotAssetsList, balance.Asset) {
			continue
		}

		balances = append(balances, neutronclient.BinanceBalance{
			Asset:  balance.Asset,
			Amount: math.LegacyMustNewDecFromStr(balance.Free).Add(math.LegacyMustNewDecFromStr(balance.Locked)),
		})
	}

	return balances, nil
}
