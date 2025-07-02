package binance

import (
	"context"
	"fmt"

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
		return nil, fmt.Errorf("failed to get AUM contract next round: %w", err)
	}
	return &oracle.NextRound{
		Round:     nextRound.Round,
		Timestamp: nextRound.Timestamp,
	}, nil
}

// FetchData retrieves the current AUM data from Binance and converts it to the Neutron AUM
// contract data structure.
func (o *BinanceAumOracleForNeutron) FetchData(ctx context.Context) (*neutronclient.BinanceAumData, error) {
	d, err := fetchBinanceData(ctx, o.binanceClient, o.config.UmPositionsList, o.config.SpotAssetsList)
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
		return nil, err
	}
	return &oracle.NextRound{
		Round:     nextRound.Round,
		Timestamp: nextRound.Timestamp,
	}, nil
}
