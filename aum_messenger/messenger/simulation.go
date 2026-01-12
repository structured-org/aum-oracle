package messenger

import (
	"context"
	"sync"
	"time"

	"go.uber.org/zap"
)

// LastValueStore keeps the last value in memory.
// It is intentionally in-memory only (no persistence).
type LastValueStore[T any] struct {
	mu         sync.RWMutex
	hasValue   bool
	lastValue  T
	capturedAt time.Time
}

func (s *LastValueStore[T]) Store(value T) {
	s.mu.Lock()
	defer s.mu.Unlock()

	s.hasValue = true
	s.lastValue = value
	s.capturedAt = time.Now().UTC()
}

func (s *LastValueStore[T]) Load() (value T, ok bool, capturedAt time.Time) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	return s.lastValue, s.hasValue, s.capturedAt
}

// SubmissionModeProvider is an optional interface that allows a Messenger to describe
// what "SubmitData" actually did. This is mainly used for simulation mode.
type SubmissionModeProvider interface {
	SubmissionMode() string
}

type simulationMessenger[T any] struct {
	inner Messenger[T]
	store *LastValueStore[T]
}

type captureMessenger[T any] struct {
	inner Messenger[T]
	store *LastValueStore[T]
}

// WrapWithSimulation returns a Messenger that does not submit data on-chain.
// Instead it stores the last collected value in memory and returns the next round
// by querying the receiver.
func WrapWithSimulation[T any](inner Messenger[T], store *LastValueStore[T]) Messenger[T] {
	if store == nil {
		store = &LastValueStore[T]{}
	}
	return &simulationMessenger[T]{
		inner: inner,
		store: store,
	}
}

// WrapWithCapture stores the last value in memory, while still delegating SubmitData
// to the underlying messenger (i.e. submissions continue to happen as usual).
func WrapWithCapture[T any](inner Messenger[T], store *LastValueStore[T]) Messenger[T] {
	if store == nil {
		store = &LastValueStore[T]{}
	}
	return &captureMessenger[T]{
		inner: inner,
		store: store,
	}
}

func (s *simulationMessenger[T]) GetNextRound(ctx context.Context) (*NextRound, error) {
	return s.inner.GetNextRound(ctx)
}

func (s *simulationMessenger[T]) FetchData(ctx context.Context) (T, error) {
	return s.inner.FetchData(ctx)
}

func (s *simulationMessenger[T]) SubmitData(ctx context.Context, data T) (*NextRound, error) {
	s.store.Store(data)

	s.inner.Logger().Info("simulation enabled: captured data, skipping submit")

	// Keep round scheduling working by querying the receiver for the next round.
	return s.inner.GetNextRound(ctx)
}

func (s *simulationMessenger[T]) Logger() *zap.Logger {
	return s.inner.Logger().With(zap.Bool("simulation", true))
}

func (s *simulationMessenger[T]) SubmissionMode() string {
	return "captured"
}

func (c *captureMessenger[T]) GetNextRound(ctx context.Context) (*NextRound, error) {
	return c.inner.GetNextRound(ctx)
}

func (c *captureMessenger[T]) FetchData(ctx context.Context) (T, error) {
	return c.inner.FetchData(ctx)
}

func (c *captureMessenger[T]) SubmitData(ctx context.Context, data T) (*NextRound, error) {
	nextRound, err := c.inner.SubmitData(ctx, data)
	if err != nil {
		return nil, err
	}
	c.store.Store(data)
	return nextRound, nil
}

func (c *captureMessenger[T]) Logger() *zap.Logger {
	return c.inner.Logger().With(zap.Bool("capture_last", true))
}
