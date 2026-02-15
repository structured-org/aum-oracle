package main

import (
	"context"
	"testing"

	msgr "github.com/structured-org/aum-messenger/messenger"
	"go.uber.org/zap"
)

type fakeMessenger[T any] struct {
	logger *zap.Logger

	nextRound *msgr.NextRound
	err       error

	getNextRoundCalls int
	submitCalls       int
}

func (f *fakeMessenger[T]) GetNextRound(ctx context.Context) (*msgr.NextRound, error) {
	f.getNextRoundCalls++
	return f.nextRound, f.err
}

func (f *fakeMessenger[T]) FetchData(ctx context.Context) (T, error) {
	var zero T
	return zero, nil
}

func (f *fakeMessenger[T]) SubmitData(ctx context.Context, data T) (*msgr.NextRound, error) {
	f.submitCalls++
	return f.nextRound, f.err
}

func (f *fakeMessenger[T]) Logger() *zap.Logger {
	if f.logger == nil {
		return zap.NewNop()
	}
	return f.logger
}

func TestWrapWithSimulationDoesNotSubmit(t *testing.T) {
	inner := &fakeMessenger[string]{
		logger:    zap.NewNop(),
		nextRound: &msgr.NextRound{Round: 2, Timestamp: 123},
	}
	store := &msgr.LastValueStore[string]{}

	sim := msgr.WrapWithSimulation(inner, store)

	nr, err := sim.SubmitData(context.Background(), "hello")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if nr == nil || nr.Round != 2 || nr.Timestamp != 123 {
		t.Fatalf("unexpected next round: %+v", nr)
	}

	if inner.submitCalls != 0 {
		t.Fatalf("inner SubmitData should not be called, got %d", inner.submitCalls)
	}
	if inner.getNextRoundCalls != 1 {
		t.Fatalf("expected GetNextRound to be called once, got %d", inner.getNextRoundCalls)
	}

	v, ok, capturedAt := store.Load()
	if !ok {
		t.Fatal("expected store to have a value")
	}
	if v != "hello" {
		t.Fatalf("unexpected stored value: %q", v)
	}
	if capturedAt.IsZero() {
		t.Fatal("expected capturedAt to be set")
	}

	if p, ok := any(sim).(msgr.SubmissionModeProvider); !ok || p.SubmissionMode() != "captured" {
		t.Fatalf("expected SubmissionModeProvider captured, got ok=%v", ok)
	}
}
