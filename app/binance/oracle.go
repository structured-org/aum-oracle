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

var (
	umPositionsList = []string{"BTCUSDT", "ETHUSDT", "SOLUSDT"}
	spotAssetsList  = []string{"USDT", "BTC", "ETH", "SOL"}
)

type Oracle struct {
	binanceClient BinanceClient
	neutronClient NeutronClient

	log *zap.Logger
}

func NewOracle(binanceClient BinanceClient, neutronClient NeutronClient, log *zap.Logger) *Oracle {
	return &Oracle{
		binanceClient: binanceClient,
		neutronClient: neutronClient,
		log:           log.With(zap.String("context", "binance_aum_oracle")),
	}
}

func (o *Oracle) Run(ctx context.Context) {
	// query the next round once at initialisation
	// then the value is reassigned from submission response in the loop
	nextRound, err := o.neutronClient.GetBinanceAumContractNextRound(ctx)
	if err != nil {
		o.log.Error("failed to get next round", zap.Error(err))
		return
	}

	for {
		timeTillNextRound := time.Duration(nextRound.Timestamp-time.Now().Unix()) * time.Second
		o.log.Info("waiting for next round",
			zap.Int64("round", nextRound.Round),
			zap.Int64("round_timestamp", nextRound.Timestamp),
			zap.Duration("time_till_next_round", timeTillNextRound),
		)

		select {
		case <-time.NewTimer(timeTillNextRound).C:
			o.log.Info("new round started",
				zap.Int64("round", nextRound.Round),
				zap.Int64("round_timestamp", nextRound.Timestamp),
			)

			data, err := o.fetchBinanceData(ctx)
			if err != nil {
				o.log.Error("failed to fetch AUM data", zap.Error(err))
				return
			}
			data.Round = nextRound.Round

			nextRound, err = o.neutronClient.SubmitBinanceAumData(ctx, data)
			if err != nil {
				o.log.Error("failed to submit AUM data", zap.Error(err))
				return
			}

			o.log.Info("submitted AUM data",
				zap.Int64("round", data.Round),
				zap.Any("data", *data),
			)

		case <-ctx.Done():
			o.log.Info("oracle stopped by context")
			return
		}
	}
}

func (o *Oracle) fetchBinanceData(ctx context.Context) (*neutronclient.BinanceData, error) {
	data := &neutronclient.BinanceData{}
	wg := sync.WaitGroup{}

	wg.Add(1)
	go func() {
		defer wg.Done()

		positions, err := o.getUmPositions(ctx)
		if err != nil {
			o.log.Error("failed to get UM positions", zap.Error(err))
			return
		}
		data.Positions = positions
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()

		balances, err := o.getSpotBalances(ctx)
		if err != nil {
			o.log.Error("failed to get spot balances", zap.Error(err))
			return
		}
		data.SpotBalances = balances
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()

		pmAccount, err := o.binanceClient.GetPMAccountInfo(ctx)
		if err != nil {
			o.log.Error("failed to get PM account info", zap.Error(err))
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
			o.log.Error("failed to get PM account balances", zap.Error(err))
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

func (o *Oracle) getUmPositions(ctx context.Context) ([]neutronclient.BinancePosition, error) {
	umPositions, err := o.binanceClient.GetUmPositions(ctx)
	if err != nil {
		return nil, err
	}

	positions := make([]neutronclient.BinancePosition, 0)
	for _, position := range umPositions {
		if !slices.Contains(umPositionsList, position.Symbol) {
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

func (o *Oracle) getSpotBalances(ctx context.Context) ([]neutronclient.BinanceBalance, error) {
	spotBalances, err := o.binanceClient.GetSpotAccountInfo(ctx)
	if err != nil {
		return nil, err
	}

	balances := make([]neutronclient.BinanceBalance, 0)
	for _, balance := range spotBalances.Balances {
		if !slices.Contains(spotAssetsList, balance.Asset) {
			continue
		}

		balances = append(balances, neutronclient.BinanceBalance{
			Asset:  balance.Asset,
			Amount: math.LegacyMustNewDecFromStr(balance.Free).Add(math.LegacyMustNewDecFromStr(balance.Locked)),
		})
	}

	return balances, nil
}
