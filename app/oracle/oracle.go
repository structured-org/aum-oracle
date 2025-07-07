package oracle

import (
	"context"
	"fmt"
	"time"

	"go.uber.org/zap"
)

// Oracle is the definition of the AUM oracle interface. An oracle is an entity that
// gathers off-chain or foreign chain data about assets under management (AUM) and
// submits this data to AUM smart contracts every round. Oracles serve as bridges
// between external data sources and blockchain-based AUM tracking systems.
type Oracle[T any] interface {
	// GetNextRound retrieves information about the next round for data submission.
	// This method helps oracles understand when they need to fetch and submit data
	// to maintain the continuous flow of AUM information to smart contracts.
	GetNextRound(ctx context.Context) (*NextRound, error)

	// FetchData retrieves current AUM data from off-chain or foreign chain sources.
	// This method is responsible for gathering real-time asset under management
	// information from external data providers, APIs, or other blockchain networks.
	FetchData(ctx context.Context) (T, error)

	// SubmitData submits the gathered AUM data to the appropriate smart contracts
	// for the current round. This method ensures that the blockchain has the most
	// up-to-date information about assets under management for decision-making
	// and tracking purposes. The method returns information about the next round
	// that should be processed, regardless of whether the current submission was
	// successful or failed. This allows the oracle to continue its operation
	// seamlessly and maintain the round progression even in case of errors.
	SubmitData(ctx context.Context, data T) (*NextRound, error)

	// Logger returns the logger for the oracle.
	Logger() *zap.Logger
}

// RunOracle is a utility function that runs an oracle in a loop, fetching and submitting data at
// specified intervals. It handles the necessary context management for the oracle's operation.
func RunOracle[T any](ctx context.Context, oracle Oracle[T]) error {
	nextRound, err := oracle.GetNextRound(ctx)
	fmt.Printf("err: %s", err)
	if err != nil {
		return fmt.Errorf("failed to get initial round: %w", err)
	}

	for {
		// TODO: think through fetch/submission error cases

		timeTillNextRound := time.Duration(int64(nextRound.Timestamp)-time.Now().Unix()) * time.Second
		oracle.Logger().Info("waiting for next round",
			zap.Uint64("round", nextRound.Round),
			zap.Uint64("round_timestamp", nextRound.Timestamp),
			zap.Duration("time_till_next_round", timeTillNextRound),
		)

		select {
		case <-time.NewTimer(timeTillNextRound).C:
			oracle.Logger().Info("new round started",
				zap.Uint64("round", nextRound.Round),
				zap.Uint64("round_timestamp", nextRound.Timestamp),
			)

			data, err := oracle.FetchData(ctx)
			if err != nil {
				oracle.Logger().Error("failed to fetch data",
					zap.Uint64("round", nextRound.Round),
					zap.Error(err),
				)
				continue
			}

			nextRound, err = oracle.SubmitData(ctx, data)
			if err != nil {
				oracle.Logger().Error("failed to submit AUM data",
					zap.Uint64("round", nextRound.Round),
					zap.Any("data", data),
					zap.Error(err),
				)
				continue
			}

			oracle.Logger().Info("AUM data submitted",
				zap.Uint64("round", nextRound.Round),
				zap.Any("data", data),
			)

		case <-ctx.Done():
			oracle.Logger().Info("oracle stopped by context")
			return nil
		}
	}
}
