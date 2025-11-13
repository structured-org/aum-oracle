package binance

import (
	"context"
	"fmt"

	msgr "github.com/structured-org/aum-messenger/messenger"
	neutronclient "github.com/structured-org/aum-messenger/pkg/client/neutron"
	"go.uber.org/zap"
)

// BinanceAumMessengerForNeutron is an Oracle Messenger implementation that is used to fetch data
// from Binance and submit it to the Neutron AUM contract as the Oracle Receiver.
type BinanceAumMessengerForNeutron struct {
	binanceClient BinanceClient
	neutronClient NeutronAumReceiverClient
	config        Config
	logger        *zap.Logger
}

// NewBinanceAumMessengerForNeutron creates a new Binance AUM messenger for the Neutron network.
func NewBinanceAumMessengerForNeutron(
	binanceClient BinanceClient,
	neutronClient NeutronAumReceiverClient,
	config Config,
	logger *zap.Logger,
) *BinanceAumMessengerForNeutron {
	return &BinanceAumMessengerForNeutron{
		binanceClient: binanceClient,
		neutronClient: neutronClient,
		config:        config,
		logger:        logger.With(zap.String("network", "neutron")),
	}
}

// GetNextRound retrieves the next round for the Binance AUM receiver contract.
func (o *BinanceAumMessengerForNeutron) GetNextRound(ctx context.Context) (*msgr.NextRound, error) {
	nextRound, err := o.neutronClient.GetBinanceAumReceiverNextRound(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get AUM receiver next round: %w", err)
	}
	return &msgr.NextRound{
		Round:     nextRound.Round,
		Timestamp: nextRound.Timestamp,
	}, nil
}

// FetchData retrieves the current AUM data from Binance and converts it to the Neutron AUM
// receiver contract data structure.
func (o *BinanceAumMessengerForNeutron) FetchData(ctx context.Context) (*neutronclient.BinanceAumData, error) {
	d, err := fetchBinanceData(ctx, o.binanceClient, o.config.UmPositionsList, o.config.SpotAssetsList)
	if err != nil {
		return nil, err
	}
	return d.ToNeutronAumData()
}

func (o *BinanceAumMessengerForNeutron) Logger() *zap.Logger {
	return o.logger
}

// SubmitData submits the AUM data to the Neutron AUM receiver contract.
func (o *BinanceAumMessengerForNeutron) SubmitData(ctx context.Context, data *neutronclient.BinanceAumData) (*msgr.NextRound, error) {
	nextRound, err := o.neutronClient.SubmitBinanceAumData(ctx, data)
	if err != nil {
		return nil, err
	}
	return &msgr.NextRound{
		Round:     nextRound.Round,
		Timestamp: nextRound.Timestamp,
	}, nil
}
