package solana

import (
	"context"
	"sync"
	"time"

	"cosmossdk.io/math"
	solana "github.com/gagliardetto/solana-go"
	neutronclient "github.com/structured-org/aum-oracle/client/neutron"
	"go.uber.org/zap"
)

// JupiterConfig is the configuration for the Jupiter oracle.
type JupiterConfig struct {
	// Custodies is the map of Jupiter custodies represented as token->programId.
	Custodies map[string]solana.PublicKey
	// Token is the Jupiter JLP token address.
	Token solana.PublicKey
	// Pool is the Jupiter JLP pool address.
	Pool solana.PublicKey
	// Strategy is the Jupiter JLP strategy address.
	Strategy solana.PublicKey
}

// Oracle is the Solana oracle. It is responsible for fetching data from the Solana blockchain and
// submitting it to the Neutron client.
type Oracle struct {
	solanaClient  SolanaClient
	neutronClient NeutronClient
	jupiterClient JupiterClient
	jupiterConfig JupiterConfig

	logger *zap.Logger
}

// NewOracle creates a new Solana oracle.
func NewOracle(
	solanaClient SolanaClient,
	neutronClient NeutronClient,
	jupiterClient JupiterClient,
	jupiterConfig JupiterConfig,
	logger *zap.Logger,
) *Oracle {
	return &Oracle{
		solanaClient:  solanaClient,
		neutronClient: neutronClient,
		jupiterClient: jupiterClient,
		jupiterConfig: jupiterConfig,
		logger:        logger,
	}
}

// Run runs the Solana oracle. It starts query-submission loop that periodically fetches data from
// the Solana blockchain and submits it using the Neutron client.
func (o *Oracle) Run(ctx context.Context) {
	// query the next round once at initialisation
	// then the value is reassigned from submission response in the loop
	nextRound, err := o.neutronClient.GetSolanaAumContractNextRound(ctx)
	if err != nil {
		o.logger.Error("failed to get next round", zap.Error(err))
		return
	}

	for {
		timeTillNextRound := time.Duration(nextRound.Timestamp-time.Now().Unix()) * time.Second
		o.logger.Info("waiting for next round",
			zap.Int64("round", nextRound.Round),
			zap.Int64("round_timestamp", nextRound.Timestamp),
			zap.Duration("time_till_next_round", timeTillNextRound),
		)

		select {
		case <-time.NewTimer(timeTillNextRound).C:
			o.logger.Info("new round started",
				zap.Int64("round", nextRound.Round),
				zap.Int64("round_timestamp", nextRound.Timestamp),
			)

			data, err := o.fetchSolanaData(ctx)
			if err != nil {
				o.logger.Error("failed to fetch AUM data", zap.Error(err))
				return
			}
			data.Round = nextRound.Round

			nextRound, err = o.neutronClient.SubmitSolanaAumData(ctx, data)
			if err != nil {
				o.logger.Error("failed to submit AUM data", zap.Error(err))
				return
			}

			o.logger.Info("submitted AUM data",
				zap.Int64("round", data.Round),
				zap.Any("data", *data),
			)

		case <-ctx.Done():
			o.logger.Info("oracle stopped by context")
			return
		}
	}
}

// fetchSolanaData concurrently fetches all required data from the Solana blockchain.
func (o *Oracle) fetchSolanaData(ctx context.Context) (*neutronclient.SolanaData, error) {
	data := &neutronclient.SolanaData{}
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
