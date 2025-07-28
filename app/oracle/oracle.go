package oracle

import (
	"context"
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
	// that should be processed.
	SubmitData(ctx context.Context, data T) (*NextRound, error)

	// Logger returns the logger for the oracle.
	Logger() *zap.Logger
}

// failureDelay is the delay taken when an oracle fails to fetch or submit data to prevent
// the oracle from spamming.
// TODO: make this configurable.
var failureDelay = 10 * time.Second

// preSubmitDelay is an additional delay taken before submitting data to prevent the oracle from
// submitting data too soon because of possible time desync.
// TODO: make this configurable.
var preSubmitDelay = 5 * time.Second

// fetchTimeout is the timeout for the fetch data operation
var fetchTimeout = 3 * time.Second

// RunOracle is a utility function that runs an oracle in a loop, fetching and submitting data at
// specified intervals. It handles the necessary context management for the oracle's operation.
func RunOracle[T any](ctx context.Context, oracle Oracle[T]) {
	var nextRound *NextRound
	var err error
	for {
		if nextRound == nil { // case on initialisation or on failure
			nextRound, err = oracle.GetNextRound(ctx)
			if err != nil {
				oracle.Logger().Error("failed to query next round, having a delay",
					zap.Error(err),
					zap.String("delay", failureDelay.String()),
				)
				time.Sleep(failureDelay)
				continue
			}
			oracle.Logger().Info("fetched next round from contract",
				zap.Uint64("round", nextRound.Round),
				zap.Uint64("round_timestamp", nextRound.Timestamp),
				zap.Duration("pre_submit_delay", preSubmitDelay),
			)
		}

		timeTillNextRound := time.Duration(int64(nextRound.Timestamp)-time.Now().Unix()) * time.Second
		timeTillNextRound = timeTillNextRound + preSubmitDelay
		oracle.Logger().Info("waiting for next round",
			zap.Uint64("round", nextRound.Round),
			zap.Uint64("round_timestamp", nextRound.Timestamp),
			zap.Duration("pre_submit_delay", preSubmitDelay),
			zap.Duration("time_till_next_round", timeTillNextRound),
		)

		select {
		case <-time.NewTimer(timeTillNextRound).C:
			oracle.Logger().Info("new round started",
				zap.Uint64("round", nextRound.Round),
				zap.Uint64("round_timestamp", nextRound.Timestamp),
			)

			nextRound = processRound(ctx, oracle, nextRound)
			if nextRound == nil {
				oracle.Logger().Info("having a delay after round processing failure",
					zap.String("delay", failureDelay.String()),
				)
				time.Sleep(failureDelay)
				continue
			}

		case <-ctx.Done():
			oracle.Logger().Info("oracle stopped by context")
			return
		}
	}
}

// processRound fetches and submits data for a given round. Returns the next round on successful
// processing as a response from the oracle. If either fetching or submitting data fails, it will
// return nil, meaning that the next round is unknown.
func processRound[T any](ctx context.Context, oracle Oracle[T], round *NextRound) *NextRound {
	fetchCtx, fetchCancel := context.WithTimeout(ctx, fetchTimeout)
	defer fetchCancel()

	data, err := oracle.FetchData(fetchCtx)
	if err != nil {
		oracle.Logger().Error("failed to fetch data",
			zap.Uint64("round", round.Round),
			zap.Error(err),
		)
		return nil
	}

	nextRound, err := oracle.SubmitData(ctx, data)
	if err != nil {
		oracle.Logger().Error("failed to submit data",
			zap.Uint64("round", round.Round),
			zap.Any("data", data),
			zap.Error(err),
		)
		return nil
	}

	oracle.Logger().Info("AUM data submitted",
		zap.Uint64("round", round.Round),
		zap.Any("data", data),
	)
	return nextRound
}
