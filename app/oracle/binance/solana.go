package binance

import (
	"context"
	"fmt"

	solanaclient "github.com/structured-org/aum-oracle/client/solana"
	"github.com/structured-org/aum-oracle/oracle"
	"go.uber.org/zap"
)

// BinanceAumOracleForSolana is an Oracle implementation that is used to fetch data from Binance
// and submit it to the Solana AUM contract.
type BinanceAumOracleForSolana struct {
	binanceClient BinanceClient
	solanaClient  SolanaAumContractClient
	config        Config
	logger        *zap.Logger
}

// NewBinanceAumOracleForSolana creates a new Binance AUM oracle for the Solana network.
func NewBinanceAumOracleForSolana(
	binanceClient BinanceClient,
	solanaClient SolanaAumContractClient,
	config Config,
	logger *zap.Logger,
) *BinanceAumOracleForSolana {
	return &BinanceAumOracleForSolana{
		binanceClient: binanceClient,
		solanaClient:  solanaClient,
		config:        config,
		logger:        logger.With(zap.String("network", "solana")),
	}
}

// GetNextRound retrieves the next round for the Binance AUM oracle.
func (o *BinanceAumOracleForSolana) GetNextRound(ctx context.Context) (*oracle.NextRound, error) {
	nextRound, err := o.solanaClient.GetBinanceAumContractNextRound(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get AUM contract next round: %w", err)
	}
	return &oracle.NextRound{
		Round:     nextRound.Round,
		Timestamp: nextRound.Timestamp,
	}, nil
}

// FetchData retrieves the current AUM data from Binance and converts it to the Solana AUM
// contract data structure.
func (o *BinanceAumOracleForSolana) FetchData(ctx context.Context) (*solanaclient.BinanceAumData, error) {
	d, err := fetchBinanceData(ctx, o.binanceClient, o.config.UmPositionsList, o.config.SpotAssetsList)
	if err != nil {
		return nil, err
	}
	return d.ToJupiterAumData()
}

func (o *BinanceAumOracleForSolana) Logger() *zap.Logger {
	return o.logger
}

// SubmitData submits the AUM data to the Solana AUM contract.
func (o *BinanceAumOracleForSolana) SubmitData(ctx context.Context, data *solanaclient.BinanceAumData) (*oracle.NextRound, error) {
	nextRound, err := o.solanaClient.SubmitBinanceAumData(ctx, data)
	if err != nil {
		return nil, err
	}
	return &oracle.NextRound{
		Round:     nextRound.Round,
		Timestamp: nextRound.Timestamp,
	}, nil
}
