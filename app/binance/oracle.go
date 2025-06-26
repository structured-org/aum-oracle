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

		pmAccountActualEquity, err := math.LegacyNewDecFromStr(pmAccount.ActualEquity)
		if err != nil {
			o.logger.Error("failed to parse PM account actual equity",
				zap.String("actual_equity", pmAccount.ActualEquity),
				zap.Error(err),
			)
		}
		withdrawableUsdt, err := math.LegacyNewDecFromStr(pmAccount.VirtualMaxWithdrawAmount)
		if err != nil {
			o.logger.Error("failed to parse PM account withdrawable amount",
				zap.String("withdrawable_amount", pmAccount.VirtualMaxWithdrawAmount),
				zap.Error(err),
			)
			return
		}
		unimmr, err := math.LegacyNewDecFromStr(pmAccount.UniMMR)
		if err != nil {
			o.logger.Error("failed to parse PM account UniMMR",
				zap.String("uni_mmr", pmAccount.UniMMR),
				zap.Error(err),
			)
			return
		}

		data.PmAccountActualEquity = pmAccountActualEquity
		data.WithdrawableUsdt = withdrawableUsdt
		data.Unimmr = unimmr
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
				umBalanceUsdt, err := math.LegacyNewDecFromStr(balance.UMWalletBalance)
				if err != nil {
					o.logger.Error("failed to parse PM account USDT balance",
						zap.String("usdt_balance", balance.UMWalletBalance),
						zap.Error(err),
					)
					return
				}

				data.UmBalanceUsdt = umBalanceUsdt
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

		amount, err := math.LegacyNewDecFromStr(position.PositionAmt)
		if err != nil {
			o.logger.Error("failed to parse UM position amount",
				zap.String("symbol", position.Symbol),
				zap.String("amount", position.PositionAmt),
				zap.Error(err),
			)
			continue
		}

		pnl, err := math.LegacyNewDecFromStr(position.UnrealizedProfit)
		if err != nil {
			o.logger.Error("failed to parse UM position PNL",
				zap.String("symbol", position.Symbol),
				zap.String("pnl", position.UnrealizedProfit),
				zap.Error(err),
			)
			continue
		}

		positions = append(positions, neutronclient.BinancePosition{
			Symbol: position.Symbol,
			Amount: amount,
			Pnl:    pnl,
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

		free, err := math.LegacyNewDecFromStr(balance.Free)
		if err != nil {
			o.logger.Error("failed to parse spot balance free amount",
				zap.String("asset", balance.Asset),
				zap.String("free", balance.Free),
				zap.Error(err),
			)
			continue
		}

		locked, err := math.LegacyNewDecFromStr(balance.Locked)
		if err != nil {
			o.logger.Error("failed to parse spot balance locked amount",
				zap.String("asset", balance.Asset),
				zap.String("locked", balance.Locked),
				zap.Error(err),
			)
			continue
		}

		balances = append(balances, neutronclient.BinanceBalance{
			Asset:  balance.Asset,
			Amount: free.Add(locked),
		})
	}

	return balances, nil
}
