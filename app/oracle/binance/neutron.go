package binance

import (
	"context"
	"errors"
	"fmt"
	"slices"
	"sync"

	binance "github.com/adshao/go-binance/v2"
	neutronclient "github.com/structured-org/aum-oracle/client/neutron"
	"github.com/structured-org/aum-oracle/oracle"
	"go.uber.org/zap"
)

// BinanceAumOracleForNeutron is an Oracle implementation that is used to fetch data from Binance
// and submit it to the Neutron AUM contract.
type BinanceAumOracleForNeutron struct {
	binanceClient BinanceClient
	neutronClient NeutronAumContractClient
	config        Config
	logger        *zap.Logger
}

// NewBinanceAumOracleForNeutron creates a new Binance AUM oracle for the Neutron network.
func NewBinanceAumOracleForNeutron(
	binanceClient BinanceClient,
	neutronClient NeutronAumContractClient,
	config Config,
	logger *zap.Logger,
) *BinanceAumOracleForNeutron {
	return &BinanceAumOracleForNeutron{
		binanceClient: binanceClient,
		neutronClient: neutronClient,
		config:        config,
		logger:        logger.With(zap.String("network", "neutron")),
	}
}

// GetNextRound retrieves the next round for the Binance AUM oracle.
func (o *BinanceAumOracleForNeutron) GetNextRound(ctx context.Context) (*oracle.NextRound, error) {
	nextRound, err := o.neutronClient.GetBinanceAumContractNextRound(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get next round: %w", err)
	}
	return &oracle.NextRound{
		Round:     nextRound.Round,
		Timestamp: nextRound.Timestamp,
	}, nil
}

// FetchData retrieves the current AUM data from Binance and converts it to the Neutron AUM
// contract data structure.
func (o *BinanceAumOracleForNeutron) FetchData(ctx context.Context) (*neutronclient.BinanceAumData, error) {
	d, err := o.fetchBinanceData(ctx)
	if err != nil {
		return nil, err
	}
	return d.ToNeutronData()
}

func (o *BinanceAumOracleForNeutron) Logger() *zap.Logger {
	return o.logger
}

// SubmitData submits the AUM data to the Neutron AUM contract.
func (o *BinanceAumOracleForNeutron) SubmitData(ctx context.Context, data *neutronclient.BinanceAumData) (*oracle.NextRound, error) {
	nextRound, err := o.neutronClient.SubmitBinanceAumData(ctx, data)
	if err != nil {
		return nil, fmt.Errorf("failed to submit Binance AUM data: %w", err)
	}
	return &oracle.NextRound{
		Round:     nextRound.Round,
		Timestamp: nextRound.Timestamp,
	}, nil
}

// fetchBinanceData concurrently fetches all required data from Binance using the Binance client.
func (o *BinanceAumOracleForNeutron) fetchBinanceData(ctx context.Context) (*BinanceAumData, error) {
	data := &BinanceAumData{}
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
