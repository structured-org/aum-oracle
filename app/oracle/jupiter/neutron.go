package jupiter

import (
	"context"
	"fmt"

	neutronclient "github.com/structured-org/aum-oracle/client/neutron"
	"github.com/structured-org/aum-oracle/oracle"
	"go.uber.org/zap"
)

// JupiterAumOracleForNeutron is an Oracle implementation that is used to fetch data from the
// Jupiter protocol and submit it to the Neutron AUM contract.
type JupiterAumOracleForNeutron struct {
	solanaClient  SolanaClient
	neutronClient NeutronAumContractClient
	jupiterClient JupiterClient
	jupiterConfig JupiterConfig

	logger *zap.Logger
}

// NewJupiterAumOracleForNeutron creates a new Jupiter AUM oracle for the Neutron network.
func NewJupiterAumOracleForNeutron(
	solanaClient SolanaClient,
	neutronClient NeutronAumContractClient,
	jupiterClient JupiterClient,
	jupiterConfig JupiterConfig,
	logger *zap.Logger,
) *JupiterAumOracleForNeutron {
	return &JupiterAumOracleForNeutron{
		solanaClient:  solanaClient,
		neutronClient: neutronClient,
		jupiterClient: jupiterClient,
		jupiterConfig: jupiterConfig,
		logger:        logger.With(zap.String("network", "neutron")),
	}
}

// GetNextRound retrieves the next round for the Jupiter AUM oracle.
func (o *JupiterAumOracleForNeutron) GetNextRound(ctx context.Context) (*oracle.NextRound, error) {
	nextRound, err := o.neutronClient.GetJupiterAumContractNextRound(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get AUM contract next round: %w", err)
	}
	return &oracle.NextRound{
		Round:     nextRound.Round,
		Timestamp: nextRound.Timestamp,
	}, nil
}

// FetchData fetches the Jupiter AUM data from the Jupiter protocol.
func (o *JupiterAumOracleForNeutron) FetchData(ctx context.Context) (*neutronclient.JupiterAumData, error) {
	return fetchJupiterAumData(ctx, o.jupiterClient, o.solanaClient, o.jupiterConfig)
}

func (o *JupiterAumOracleForNeutron) Logger() *zap.Logger {
	return o.logger
}

// SubmitData submits the Jupiter AUM data to the Neutron AUM contract.
func (o *JupiterAumOracleForNeutron) SubmitData(ctx context.Context, data *neutronclient.JupiterAumData) (*oracle.NextRound, error) {
	nextRound, err := o.neutronClient.SubmitJupiterAumData(ctx, data)
	if err != nil {
		return nil, err
	}
	return &oracle.NextRound{
		Round:     nextRound.Round,
		Timestamp: nextRound.Timestamp,
	}, nil
}
