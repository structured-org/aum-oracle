package binance

import (
	"context"
	"errors"
	"fmt"
	"slices"
	"sync"
	"time"

	binance "github.com/adshao/go-binance/v2"
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
// to Neutron and Solana AUM contracts.
type Oracle struct {
	binanceClient BinanceClient
	neutronClient NeutronAumContractClient
	solanaClient  SolanaAumContractClient
	config        Config

	logger *zap.Logger
}

// NewOracle creates a new Binance oracle.
func NewOracle(
	binanceClient BinanceClient,
	neutronClient NeutronAumContractClient,
	solanaClient SolanaAumContractClient,
	config Config,
	logger *zap.Logger,
) *Oracle {
	return &Oracle{
		binanceClient: binanceClient,
		neutronClient: neutronClient,
		solanaClient:  solanaClient,
		config:        config,
		logger:        logger,
	}
}

// Run runs the Binance oracle.
func (o *Oracle) Run(ctx context.Context) {
	wg := sync.WaitGroup{}

	wg.Add(1)
	go func() {
		defer wg.Done()
		o.serveBinance(ctx)
	}()
	wg.Add(1)
	go func() {
		defer wg.Done()
		o.serveSolana(ctx)
	}()

	wg.Wait()
}

// serveBinance starts oracle for Binance. It starts query-submission loop that periodically fetches
// data from Binance and submits it to the Neutron AUM contract.
func (o *Oracle) serveBinance(ctx context.Context) {
	l := o.logger.With(zap.String("serving", "binance"))

	// query the next round once at initialisation
	// then the value is reassigned from submission response in the loop
	nextRound, err := o.neutronClient.GetBinanceAumContractNextRound(ctx)
	if err != nil {
		l.Error("failed to get next round", zap.Error(err))
		return
	}

	for {
		timeTillNextRound := time.Duration(nextRound.Timestamp-time.Now().Unix()) * time.Second
		l.Info("waiting for next round",
			zap.Int64("round", nextRound.Round),
			zap.Int64("round_timestamp", nextRound.Timestamp),
			zap.Duration("time_till_next_round", timeTillNextRound),
		)

		select {
		case <-time.NewTimer(timeTillNextRound).C:
			l.Info("new round started",
				zap.Int64("round", nextRound.Round),
				zap.Int64("round_timestamp", nextRound.Timestamp),
			)

			binanceData, err := o.fetchBinanceData(ctx)
			if err != nil {
				l.Error("failed to fetch AUM data", zap.Error(err))
				return
			}
			neutronData, err := binanceData.ToNeutronData()
			if err != nil {
				l.Error("failed to convert Binance data to Neutron data", zap.Error(err))
				return
			}
			neutronData.Round = nextRound.Round

			nextRound, err = o.neutronClient.SubmitBinanceAumData(ctx, neutronData)
			if err != nil {
				l.Error("failed to submit AUM data", zap.Error(err))
				return
			}

			l.Info("submitted AUM data",
				zap.Int64("round", neutronData.Round),
				zap.Any("data", *neutronData),
			)

		case <-ctx.Done():
			l.Info("oracle stopped by context")
			return
		}
	}
}

// serveSolana starts oracle for Solana. It starts query-submission loop that periodically fetches
// data from Binance and submits it to the Solana AUM contract.
func (o *Oracle) serveSolana(ctx context.Context) {
	l := o.logger.With(zap.String("serving", "solana"))

	// query the next round once at initialisation
	// then the value is reassigned from submission response in the loop
	nextRound, err := o.solanaClient.GetBinanceAumContractNextRound(ctx)
	if err != nil {
		l.Error("failed to get next round", zap.Error(err))
		return
	}

	for {
		timeTillNextRound := time.Duration(nextRound.Timestamp-time.Now().Unix()) * time.Second
		l.Info("waiting for next round",
			zap.Int64("round", nextRound.Round),
			zap.Int64("round_timestamp", nextRound.Timestamp),
			zap.Duration("time_till_next_round", timeTillNextRound),
		)

		select {
		case <-time.NewTimer(timeTillNextRound).C:
			l.Info("new round started",
				zap.Int64("round", nextRound.Round),
				zap.Int64("round_timestamp", nextRound.Timestamp),
			)

			binanceData, err := o.fetchBinanceData(ctx)
			if err != nil {
				l.Error("failed to fetch AUM data", zap.Error(err))
				return
			}
			solanaData, err := binanceData.ToSolanaData()
			if err != nil {
				l.Error("failed to convert Binance data to Solana data", zap.Error(err))
				return
			}
			solanaData.Round = nextRound.Round

			nextRound, err = o.solanaClient.SubmitBinanceAumData(ctx, solanaData)
			if err != nil {
				l.Error("failed to submit AUM data", zap.Error(err))
				return
			}

			l.Info("submitted AUM data",
				zap.Int64("round", solanaData.Round),
				zap.Any("data", *solanaData),
			)

		case <-ctx.Done():
			l.Info("oracle stopped by context")
			return
		}
	}
}

// fetchBinanceData concurrently fetches all required data from Binance using the Binance client.
func (o *Oracle) fetchBinanceData(ctx context.Context) (*BinanceData, error) {
	data := &BinanceData{}
	wg := sync.WaitGroup{}
	errsMu := sync.Mutex{}
	errs := make([]error, 0)

	wg.Add(1)
	go func() {
		defer wg.Done()

		umPositions, err := o.binanceClient.GetUmPositions(ctx)
		if err != nil {
			errsMu.Lock()
			errs = append(errs, fmt.Errorf("failed to get UM positions: %w", err))
			errsMu.Unlock()
			return
		}
		for i, position := range umPositions {
			if !slices.Contains(o.config.UmPositionsList, position.Symbol) {
				umPositions = append(umPositions[:i], umPositions[i+1:]...)
			}
		}

		data.UmPositions = umPositions
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()

		spotAccountInfo, err := o.binanceClient.GetSpotAccountInfo(ctx)
		if err != nil {
			errsMu.Lock()
			errs = append(errs, fmt.Errorf("failed to get spot balances: %w", err))
			errsMu.Unlock()
			return
		}

		balances := make([]*binance.Balance, 0)
		for _, balance := range spotAccountInfo.Balances {
			if slices.Contains(o.config.SpotAssetsList, balance.Asset) {
				balances = append(balances, &balance)
			}
		}

		data.SpotBalances = balances
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()

		pmAccount, err := o.binanceClient.GetPMAccountInfo(ctx)
		if err != nil {
			errsMu.Lock()
			errs = append(errs, fmt.Errorf("failed to get PM account info: %w", err))
			errsMu.Unlock()
			return
		}

		data.PmAccountActualEquity = pmAccount.ActualEquity
		data.WithdrawableUsdt = pmAccount.VirtualMaxWithdrawAmount
		data.UniMMR = pmAccount.UniMMR
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()

		pmAccountBalances, err := o.binanceClient.GetPMAccountBalance(ctx)
		if err != nil {
			errsMu.Lock()
			errs = append(errs, fmt.Errorf("failed to get PM account balances: %w", err))
			errsMu.Unlock()
			return
		}
		for _, balance := range pmAccountBalances {
			if balance.Asset == "USDT" {
				data.UmBalanceUsdt = balance.UMWalletBalance
			}
		}
	}()

	wg.Wait()
	if len(errs) > 0 {
		return nil, fmt.Errorf("failed to fetch Binance data: %w", errors.Join(errs...))
	}
	return data, nil
}
