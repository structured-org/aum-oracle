package arbitrary_data

import (
	"context"

	solana "github.com/gagliardetto/solana-go"
	msgrclient "github.com/structured-org/aum-messenger/client"
	"go.uber.org/zap"
)

// ArbitraryNeutronDataMessengerForSolana is an Oracle Messenger implementation that is used to fetch data
// from the Jupiter protocol and submit it to the Neutron AUM contract as the Oracle Receiver.
type ArbitraryNeutronDataMessengerForSolana struct {
	solanaClient  SolanaClient
	neutronClient NeutronClient
	queryConfig   NeutronContractQuery

	logger *zap.Logger
}

// NewJupiterAumMessengerForNeutron creates a new Jupiter AUM messenger for the Neutron network.
func NewArbitraryNeutronDataMessengerForSolana(
	solanaClient SolanaClient,
	neutronClient NeutronClient,
	queryConfig NeutronContractQuery,
	logger *zap.Logger,
) *ArbitraryNeutronDataMessengerForSolana {
	return &ArbitraryNeutronDataMessengerForSolana{
		solanaClient:  solanaClient,
		neutronClient: neutronClient,
		queryConfig:   queryConfig,
		logger:        logger.With(zap.String("network", "arbitrary_data")),
	}
}

// GetNextRound retrieves the next round for the Jupiter AUM receiver contract.
func (o *ArbitraryNeutronDataMessengerForSolana) GetNextRound(ctx context.Context) (*msgrclient.NextRound, error) {

	instanceKey := solana.MustPublicKeyFromBase58(o.queryConfig.SolanaInstanceKey)
	nextRound, err := o.solanaClient.GetArbitraryNeutronContractDataNextRound(ctx, o.queryConfig.SolanaProgramID, instanceKey)
	if err != nil {
		return nil, err
	}

	return &msgrclient.NextRound{
		Round:     nextRound.Round,
		Timestamp: nextRound.Timestamp,
	}, nil
}

// FetchData fetches the Arbitrary neutron contract data.
func (o *ArbitraryNeutronDataMessengerForSolana) FetchData(ctx context.Context) (*[]byte, error) {
	o.logger.Info("FetchData",
		zap.String("contract", o.queryConfig.NeutronContractAddress),
		zap.Any("query", o.queryConfig.Query),
	)
	return o.neutronClient.QueryArbitraryNeutronContract(ctx, o.queryConfig.NeutronContractAddress, o.queryConfig.Query)
}

func (o *ArbitraryNeutronDataMessengerForSolana) Logger() *zap.Logger {
	return o.logger
}

// SubmitData submits the Jupiter AUM data to the Neutron AUM receiver contract.
func (o *ArbitraryNeutronDataMessengerForSolana) SubmitData(ctx context.Context, data *[]byte) (*msgrclient.NextRound, error) {
	o.logger.Info("SubmitData",
		zap.Any("data", data))

	instanceKey := solana.MustPublicKeyFromBase58(o.queryConfig.SolanaInstanceKey)
	programID := solana.MustPublicKeyFromBase58(o.queryConfig.SolanaProgramID)

	oraclesList, err := o.neutronClient.GetOraclesList(ctx, o.queryConfig.OraclesListContract)
	if err != nil {
		return nil, err
	}

	nextRound, err := o.solanaClient.SubmitArbitraryNeutronContractData(ctx, programID, instanceKey, oraclesList, data)
	if err != nil {
		return nil, err
	}

	return nextRound, nil
}
