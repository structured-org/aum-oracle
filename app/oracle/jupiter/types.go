package jupiter

import (
	"context"
	"errors"
	"fmt"
	"sync"

	"cosmossdk.io/math"
	solana "github.com/gagliardetto/solana-go"
	neutronclient "github.com/structured-org/aum-oracle/client/neutron"
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

// fetchJupiterAumData concurrently fetches all required data from the Jupiter protocol.
func fetchJupiterAumData(
	ctx context.Context,
	jupiterClient JupiterClient,
	solanaClient SolanaClient,
	jupiterConfig JupiterConfig,
) (*neutronclient.JupiterAumData, error) {
	data := &neutronclient.JupiterAumData{}
	wg := sync.WaitGroup{}
	errsMu := sync.Mutex{}
	errs := make([]error, 0)

	custodiesMu := sync.Mutex{}
	for token, programId := range jupiterConfig.Custodies {
		wg.Add(1)
		go func(token string, programId solana.PublicKey) {
			defer wg.Done()

			custodyInfo, err := jupiterClient.GetJupiterCustodyInfo(ctx, programId)
			if err != nil {
				errsMu.Lock()
				errs = append(errs, fmt.Errorf("failed to get Jupiter custody info: %w", err))
				errsMu.Unlock()
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

		poolInfo, err := jupiterClient.GetJupiterPoolInfo(ctx, jupiterConfig.Pool)
		if err != nil {
			errsMu.Lock()
			errs = append(errs, fmt.Errorf("failed to get Jupiter pool info: %w", err))
			errsMu.Unlock()
			return
		}

		// Jupiter pool returns aum with 6 decimals.
		// TODO: remove hardcoded value and find out how to get the correct value
		data.AumUsd = math.NewUintFromString(poolInfo.AumUsd.String()).Quo(math.NewUint(1000000))
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()

		supply, err := solanaClient.GetTokenSupply(ctx, jupiterConfig.Token)
		if err != nil {
			errsMu.Lock()
			errs = append(errs, fmt.Errorf("failed to get Jupiter token supply: %w", err))
			errsMu.Unlock()
			return
		}

		data.TotalJlpSupply = math.NewUintFromString(supply.Amount)
		data.TotalJlpSupplyDecimals = supply.Decimals
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()

		supply, err := solanaClient.GetTokenAccountBalance(ctx, jupiterConfig.Token, jupiterConfig.Strategy)
		if err != nil {
			errsMu.Lock()
			errs = append(errs, fmt.Errorf("failed to get strategy JLP balance: %w", err))
			errsMu.Unlock()
			return
		}

		data.StrategyJlpBalance = math.NewUintFromString(supply.Amount)
		data.StrategyJlpBalanceDecimals = supply.Decimals
	}()

	wg.Wait()
	if len(errs) > 0 {
		return nil, fmt.Errorf("some queries failed: %w", errors.Join(errs...))
	}
	data.SortCustodyAssets()
	return data, nil
}
