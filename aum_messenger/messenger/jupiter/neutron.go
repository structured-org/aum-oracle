package jupiter

import (
	"context"
	"fmt"

	msgrclient "github.com/structured-org/aum-messenger/client"
	neutronclient "github.com/structured-org/aum-messenger/client/neutron"
	"go.uber.org/zap"
)

// JupiterAumMessengerForNeutron is an Oracle Messenger implementation that is used to fetch data
// from the Jupiter protocol and submit it to the Neutron AUM contract as the Oracle Receiver.
type JupiterAumMessengerForNeutron struct {
	solanaClient  SolanaClient
	neutronClient NeutronAumReceiverClient
	jupiterClient JupiterClient
	jupiterConfig JupiterConfig

	logger *zap.Logger
}

// NewJupiterAumMessengerForNeutron creates a new Jupiter AUM messenger for the Neutron network.
func NewJupiterAumMessengerForNeutron(
	solanaClient SolanaClient,
	neutronClient NeutronAumReceiverClient,
	jupiterClient JupiterClient,
	jupiterConfig JupiterConfig,
	logger *zap.Logger,
) *JupiterAumMessengerForNeutron {
	return &JupiterAumMessengerForNeutron{
		solanaClient:  solanaClient,
		neutronClient: neutronClient,
		jupiterClient: jupiterClient,
		jupiterConfig: jupiterConfig,
		logger:        logger.With(zap.String("network", "neutron")),
	}
}

// GetNextRound retrieves the next round for the Jupiter AUM receiver contract.
func (o *JupiterAumMessengerForNeutron) GetNextRound(ctx context.Context) (*msgrclient.NextRound, error) {
	nextRound, err := o.neutronClient.GetJupiterAumReceiverNextRound(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get AUM receiver next round: %w", err)
	}
	return &msgrclient.NextRound{
		Round:     nextRound.Round,
		Timestamp: nextRound.Timestamp,
	}, nil
}

// FetchData fetches the Jupiter AUM data from the Jupiter protocol.
func (o *JupiterAumMessengerForNeutron) FetchData(ctx context.Context) (*neutronclient.JupiterAumData, error) {
	return fetchJupiterAumData(ctx, o.jupiterClient, o.solanaClient, o.jupiterConfig)
}

func (o *JupiterAumMessengerForNeutron) Logger() *zap.Logger {
	return o.logger
}

// SubmitData submits the Jupiter AUM data to the Neutron AUM receiver contract.
func (o *JupiterAumMessengerForNeutron) SubmitData(ctx context.Context, data *neutronclient.JupiterAumData) (*msgrclient.NextRound, error) {
	nextRound, err := o.neutronClient.SubmitJupiterAumData(ctx, data)
	if err != nil {
		return nil, err
	}
	return &msgrclient.NextRound{
		Round:     nextRound.Round,
		Timestamp: nextRound.Timestamp,
	}, nil
}
