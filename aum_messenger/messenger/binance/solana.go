package binance

import (
	"context"
	"fmt"

	msgr "github.com/structured-org/aum-messenger/messenger"
	solanaclient "github.com/structured-org/aum-messenger/pkg/client/solana"
	"go.uber.org/zap"
)

// BinanceAumMessengerForSolana is an Oracle Messenger implementation that is used to fetch data
// from Binance and submit it to the Solana AUM contract as the Oracle Receiver.
type BinanceAumMessengerForSolana struct {
	binanceClient BinanceClient
	solanaClient  SolanaAumReceiverClient
	config        Config
	logger        *zap.Logger
}

// NewBinanceAumMessengerForSolana creates a new Binance AUM messenger for the Solana network.
func NewBinanceAumMessengerForSolana(
	binanceClient BinanceClient,
	solanaClient SolanaAumReceiverClient,
	config Config,
	logger *zap.Logger,
) *BinanceAumMessengerForSolana {
	return &BinanceAumMessengerForSolana{
		binanceClient: binanceClient,
		solanaClient:  solanaClient,
		config:        config,
		logger:        logger.With(zap.String("network", "solana")),
	}
}

// GetNextRound retrieves the next round for the Binance AUM receiver contract.
func (o *BinanceAumMessengerForSolana) GetNextRound(ctx context.Context) (*msgr.NextRound, error) {
	nextRound, err := o.solanaClient.GetBinanceAumReceiverNextRound(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get AUM receiver next round: %w", err)
	}
	return &msgr.NextRound{
		Round:     nextRound.Round,
		Timestamp: nextRound.Timestamp,
	}, nil
}

// FetchData retrieves the current AUM data from Binance and converts it to the Solana AUM
// receiver contract data structure.
func (o *BinanceAumMessengerForSolana) FetchData(ctx context.Context) (*solanaclient.BinanceAumData, error) {
	d, err := fetchBinanceData(ctx, o.binanceClient, o.config.UmPositionsList, o.config.SpotAssetsList)
	if err != nil {
		return nil, err
	}
	return d.ToJupiterAumData()
}

func (o *BinanceAumMessengerForSolana) Logger() *zap.Logger {
	return o.logger
}

// SubmitData submits the AUM data to the Solana AUM receiver contract.
func (o *BinanceAumMessengerForSolana) SubmitData(ctx context.Context, data *solanaclient.BinanceAumData) (*msgr.NextRound, error) {
	nextRound, err := o.solanaClient.SubmitBinanceAumData(ctx, data)
	if err != nil {
		return nil, err
	}
	return &msgr.NextRound{
		Round:     nextRound.Round,
		Timestamp: nextRound.Timestamp,
	}, nil
}
