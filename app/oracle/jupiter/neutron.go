package jupiter

import (
	"context"
	"fmt"
	"sync"

	"cosmossdk.io/math"
	solana "github.com/gagliardetto/solana-go"
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
		return nil, fmt.Errorf("failed to get next round: %w", err)
	}
	return &oracle.NextRound{
		Round:     nextRound.Round,
		Timestamp: nextRound.Timestamp,
	}, nil
}

// FetchData fetches the Jupiter AUM data from the Jupiter protocol.
func (o *JupiterAumOracleForNeutron) FetchData(ctx context.Context) (*neutronclient.JupiterAumData, error) {
	return o.fetchJupiterAumData(ctx)
}

func (o *JupiterAumOracleForNeutron) Logger() *zap.Logger {
	return o.logger
}

// SubmitData submits the Jupiter AUM data to the Neutron AUM contract.
func (o *JupiterAumOracleForNeutron) SubmitData(ctx context.Context, data *neutronclient.JupiterAumData) (*oracle.NextRound, error) {
	nextRound, err := o.neutronClient.SubmitJupiterAumData(ctx, data)
	if err != nil {
		return nil, fmt.Errorf("failed to submit Jupiter AUM data: %w", err)
	}
	return &oracle.NextRound{
		Round:     nextRound.Round,
		Timestamp: nextRound.Timestamp,
	}, nil
}

// fetchJupiterAumData concurrently fetches all required data from the Jupiter protocol.
func (o *JupiterAumOracleForNeutron) fetchJupiterAumData(ctx context.Context) (*neutronclient.JupiterAumData, error) {
	data := &neutronclient.JupiterAumData{}
	wg := sync.WaitGroup{}

	custodiesMu := sync.Mutex{}
	for token, programId := range o.jupiterConfig.Custodies {
		wg.Add(1)
		go func(token string, programId solana.PublicKey) {
			defer wg.Done()

			custodyInfo, err := o.jupiterClient.GetJupiterCustodyInfo(ctx, programId)
			if err != nil {
				o.logger.Error("failed to get Jupiter custody info",
					zap.String("custody_token", token),
					zap.String("custody_program_id", programId.String()),
					zap.Error(err))
				return
			}

			custodiesMu.Lock()
			data.CustodyAssets = append(data.CustodyAssets, neutronclient.JupiterCustodyAsset{
				Denom:         token,
				Owned:         custodyInfo.Assets.Owned,
				Locked:        custodyInfo.Assets.Locked,
				GuaranteedUsd: custodyInfo.Assets.GuaranteedUsd,
				Decimals:      custodyInfo.Decimals,
			})
			custodiesMu.Unlock()
		}(token, programId)
	}

	wg.Add(1)
	go func() {
		defer wg.Done()

		poolInfo, err := o.jupiterClient.GetJupiterPoolInfo(ctx, o.jupiterConfig.Pool)
		if err != nil {
			o.logger.Error("failed to get Jupiter pool info",
				zap.String("pool_program_id", o.jupiterConfig.Pool.String()),
				zap.Error(err),
			)
			return
		}

		// Jupiter pool returns aum with 6 decimals.
		// TODO: remove hardcoded value and find out how to get the correct value
		data.AumUsd = math.NewUintFromString(poolInfo.AumUsd.String()).Quo(math.NewUint(1000000))
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()

		supply, err := o.solanaClient.GetTokenSupply(ctx, o.jupiterConfig.Token)
		if err != nil {
			o.logger.Error("failed to get Jupiter token supply",
				zap.String("token_program_id", o.jupiterConfig.Token.String()),
				zap.Error(err),
			)
			return
		}

		data.TotalJlpSupply = math.NewUintFromString(supply.Amount)
		data.JlpTokenDecimals = supply.Decimals
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()

		supply, err := o.solanaClient.GetTokenAccountBalance(ctx, o.jupiterConfig.Token, o.jupiterConfig.Strategy)
		if err != nil {
			o.logger.Error("failed to get strategy JLP balance",
				zap.String("token_program_id", o.jupiterConfig.Token.String()),
				zap.String("strategy_account", o.jupiterConfig.Strategy.String()),
				zap.Error(err),
			)
			return
		}

		data.StrategyJlpBalance = math.NewUintFromString(supply.Amount)
	}()

	wg.Wait()
	data.SortCustodyAssets()
	return data, nil
}
