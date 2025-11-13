package messenger

import (
	"context"
	"time"

	"go.uber.org/zap"
)

// Messenger is the definition of the Oracle Messenger interface. An Oracle Messenger is an entity
// that gathers off-chain or foreign chain data and submits this data to Oracle receivers every
// round. Messengers serve as bridges between external data sources and blockchain-based data
// consumers.
type Messenger[T any] interface {
	// GetNextRound retrieves information about the next round for data submission. This method
	// helps messengers understand when they need to fetch and submit data to maintain the
	// continuous flow of data to receivers.
	GetNextRound(ctx context.Context) (*NextRound, error)

	// FetchData retrieves current data from off-chain or foreign chain sources. This method is
	// responsible for gathering real-time data from external data providers, APIs, or other
	// blockchain networks.
	FetchData(ctx context.Context) (T, error)

	// SubmitData submits the gathered data to the corresponding receiver for the current round.
	// This method ensures that the blockchain has the most up-to-date information about external
	// data for decision-making and tracking purposes. The method returns information about the
	// next round that should be processed.
	SubmitData(ctx context.Context, data T) (*NextRound, error)

	// Logger returns the logger for the messenger.
	Logger() *zap.Logger
}

// RunMessenger is a utility function that runs a messenger in a loop, fetching and submitting
// data at specified intervals. It handles the necessary context management for the messenger's
// operation.
func RunMessenger[T any](ctx context.Context, msgr Messenger[T], cfg OperationalConfig) {
	var nextRound *NextRound
	var err error
	for {
		select {
		case <-ctx.Done():
			msgr.Logger().Info("messenger stopped by context")
			return
		default:
		}

		if nextRound == nil { // case on initialisation or on failure
			nextRound, err = msgr.GetNextRound(ctx)
			if err != nil {
				msgr.Logger().Error("failed to query next round, having a delay",
					zap.Error(err),
					zap.String("delay", cfg.FailureDelay.String()),
				)
				time.Sleep(cfg.FailureDelay)
				continue
			}
		}

		timeTillNextRound := time.Duration(int64(nextRound.Timestamp)-time.Now().Unix()) * time.Second
		timeTillNextRound = timeTillNextRound + cfg.PreSubmitDelay
		msgr.Logger().Info("waiting for next round",
			zap.Uint64("round", nextRound.Round),
			zap.Uint64("round_timestamp", nextRound.Timestamp),
			zap.Duration("pre_submit_delay", cfg.PreSubmitDelay),
			zap.Duration("time_till_next_round", timeTillNextRound),
		)

		select {
		case <-time.NewTimer(timeTillNextRound).C:
			msgr.Logger().Info("new round started",
				zap.Uint64("round", nextRound.Round),
				zap.Uint64("round_timestamp", nextRound.Timestamp),
			)

			nextRound = processRound(ctx, msgr, nextRound, cfg.FetchDataTimeout)
			if nextRound == nil {
				msgr.Logger().Info("having a delay after round processing failure",
					zap.String("delay", cfg.FailureDelay.String()),
				)
				time.Sleep(cfg.FailureDelay)
				continue
			}

		case <-ctx.Done():
			msgr.Logger().Info("messenger stopped by context")
			return
		}
	}
}

// processRound fetches and submits data for a given round. Returns the next round on successful
// processing as a response from the receiver. If either fetching or submitting data fails, it will
// return nil, meaning that the next round is unknown.
func processRound[T any](
	ctx context.Context,
	msgr Messenger[T],
	round *NextRound,
	fetchTimeout time.Duration,
) *NextRound {
	fetchCtx, fetchCancel := context.WithTimeout(ctx, fetchTimeout)
	defer fetchCancel()

	data, err := msgr.FetchData(fetchCtx)
	if err != nil {
		msgr.Logger().Error("failed to fetch data",
			zap.Uint64("round", round.Round),
			zap.Error(err),
		)
		return nil
	}

	nextRound, err := msgr.SubmitData(ctx, data)
	if err != nil {
		msgr.Logger().Error("failed to submit data",
			zap.Uint64("round", round.Round),
			zap.Any("data", data),
			zap.Error(err),
		)
		return nil
	}

	msgr.Logger().Info("data submitted",
		zap.Uint64("round", round.Round),
		zap.Any("data", data),
	)
	return nextRound
}
