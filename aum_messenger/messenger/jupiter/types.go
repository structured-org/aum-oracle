package jupiter

import (
	"context"
	"errors"
	"fmt"
	stdmath "math"
	"sync"

	"cosmossdk.io/math"
	"github.com/gagliardetto/solana-go"
	neutronclient "github.com/structured-org/aum-messenger/pkg/client/neutron"
)

// JupiterConfig is the configuration for the Jupiter Oracle Messenger.
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

		aumUsd, err := math.ParseUint(poolInfo.AumUsd.String())
		if err != nil {
			errsMu.Lock()
			errs = append(errs, fmt.Errorf("failed to parse Jupiter pool aumUsd %s: %w", poolInfo.AumUsd.String(), err))
			errsMu.Unlock()
			return
		}
		// Jupiter pool returns aumUsd with 6 decimals.
		data.AumUsd = aumUsd.Quo(math.NewUint(uint64(jupiterUsdDecimalsDivisor)))
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

// jupiterUsdDecimalsDivisor is the divisor for the Jupiter USD values. According to Jupiter
// team, all USD values in Jupiter are scaled to 6 decimal places (presumably to comply with
// USDC/USDT as they also have 6 decimals).
var jupiterUsdDecimalsDivisor = stdmath.Pow10(6)
