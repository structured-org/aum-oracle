package jupiter

import (
	"context"
	"errors"
	"fmt"
	stdmath "math"
	"slices"
	"sync"

	"cosmossdk.io/math"
	"github.com/gagliardetto/solana-go"
	solanarpc "github.com/gagliardetto/solana-go/rpc"
	neutronclient "github.com/structured-org/aum-messenger/pkg/client/neutron"
	solanaclient "github.com/structured-org/aum-messenger/pkg/client/solana"
)

// JupiterConfig is the configuration for the Jupiter Oracle Messenger.
type JupiterConfig struct {
	// Custodies is the map of Jupiter custodies represented as token->programId.
	Custodies map[string]solana.PublicKey
	// Pool is the Jupiter JLP pool address.
	Pool solana.PublicKey
	// SolanaBalancesList is the list of address->tokens mappings of Solana balances.
	SolanaBalancesList map[solana.PublicKey][]string
	// SolanaTokenSupplyList is the list of Solana token addresses to fetch the total supply of.
	SolanaTokenSupplyList []solana.PublicKey
}

// fetchJupiterAumData concurrently fetches all required data from the Jupiter protocol.
func fetchJupiterAumData(
	ctx context.Context,
	jupiterClient JupiterClient,
	solanaClient SolanaClient,
	jupiterConfig JupiterConfig,
) (*neutronclient.JupiterAumData, error) {
	result := &neutronclient.JupiterAumData{}

	custodyAssets, err := fetchJupiterCustodyAssets(ctx, jupiterConfig.Custodies, jupiterClient)
	if err != nil {
		return nil, fmt.Errorf("failed to get Jupiter custody assets: %w", err)
	}
	result.CustodyAssets = custodyAssets

	aumUsd, err := fetchJlpAum(ctx, jupiterConfig.Pool, jupiterClient)
	if err != nil {
		return nil, fmt.Errorf("failed to get Jupiter pool aum: %w", err)
	}
	result.AumUsd = aumUsd

	tokenSupplies, tokenDecimals, err := fetchSolanaTokenInfos(
		ctx,
		jupiterConfig.SolanaTokenSupplyList,
		jupiterConfig.SolanaBalancesList,
		solanaClient,
	)
	if err != nil {
		return nil, fmt.Errorf("failed to get Solana token infos: %w", err)
	}
	result.SolanaTokenTotalSupply = tokenSupplies
	result.SolanaTokenDecimals = tokenDecimals

	solanaBalances, err := fetchSolanaBalances(ctx, jupiterConfig.SolanaBalancesList, solanaClient)
	if err != nil {
		return nil, fmt.Errorf("failed to get Solana balances: %w", err)
	}
	result.SolanaBalances = solanaBalances

	result.Organize()
	return result, nil
}

// fetchJupiterCustodyAssets concurrently fetches info about all Jupiter custodies.
func fetchJupiterCustodyAssets(
	ctx context.Context,
	custodies map[string]solana.PublicKey,
	jupiterClient JupiterClient,
) ([]neutronclient.JupiterCustodyAsset, error) {
	wg := sync.WaitGroup{}
	assets := make([]neutronclient.JupiterCustodyAsset, 0, len(custodies))
	mu := sync.Mutex{}
	errsMu := sync.Mutex{}
	errs := make([]error, 0)
	for token, programId := range custodies {
		wg.Add(1)
		go func(token string, programId solana.PublicKey) {
			defer wg.Done()

			custodyInfo, err := jupiterClient.GetJupiterCustodyInfo(ctx, programId)
			if err != nil {
				errsMu.Lock()
				errs = append(errs, fmt.Errorf("failed to get Jupiter %s custody info: %w", token, err))
				errsMu.Unlock()
				return
			}

			mu.Lock()
			assets = append(assets, neutronclient.JupiterCustodyAsset{
				Denom:         token,
				Owned:         custodyInfo.Assets.Owned,
				Locked:        custodyInfo.Assets.Locked,
				GuaranteedUsd: custodyInfo.Assets.GuaranteedUsd,
				Decimals:      custodyInfo.Decimals,
			})
			mu.Unlock()
		}(token, programId)
	}
	wg.Wait()

	if len(errs) > 0 {
		return nil, errors.Join(errs...)
	}
	return assets, nil
}

// fetchJlpAum fetches the Jupiter pool AUM.
func fetchJlpAum(
	ctx context.Context,
	pool solana.PublicKey,
	jupiterClient JupiterClient,
) (math.Uint, error) {
	poolInfo, err := jupiterClient.GetJupiterPoolInfo(ctx, pool)
	if err != nil {
		return math.Uint{}, fmt.Errorf("failed to get Jupiter pool info: %w", err)
	}

	aumUsd, err := math.ParseUint(poolInfo.AumUsd.String())
	if err != nil {
		return math.Uint{}, fmt.Errorf("failed to parse Jupiter pool aumUsd %s: %w", poolInfo.AumUsd.String(), err)
	}
	// Jupiter pool returns aumUsd with 6 decimals.
	return aumUsd.Quo(math.NewUint(uint64(jupiterUsdDecimalsDivisor))), nil
}

// fetchSolanaTokenInfos concurrently fetches total supply and decimals of tokens from the
// solanaTotalSupplyList and decimals of tokens from the solanaBalancesList. Doesn't fetch
// total supply for SOL.
func fetchSolanaTokenInfos(
	ctx context.Context,
	solanaTotalSupplyList []solana.PublicKey,
	solanaBalancesList map[solana.PublicKey][]string,
	solanaClient SolanaClient,
) ([]neutronclient.SolanaTokenTotalSupply, []neutronclient.SolanaTokenDecimals, error) {
	uniqueTokens := make(map[string]struct{})
	for _, token := range solanaTotalSupplyList {
		uniqueTokens[token.String()] = struct{}{}
	}
	for _, tokens := range solanaBalancesList {
		for _, token := range tokens {
			uniqueTokens[token] = struct{}{}
		}
	}

	wg := sync.WaitGroup{}
	mu := sync.Mutex{}
	supplies := make([]neutronclient.SolanaTokenTotalSupply, 0, len(uniqueTokens))
	decimals := make([]neutronclient.SolanaTokenDecimals, 0, len(uniqueTokens))
	errsMu := sync.Mutex{}
	errs := make([]error, 0)
	for token := range uniqueTokens {
		if token == solanaclient.SolanaNativeTokenName {
			mu.Lock()
			decimals = append(decimals, neutronclient.SolanaTokenDecimals{
				Asset:    token,
				Decimals: solanaclient.SolanaNativeTokenDecimals,
			})
			mu.Unlock()
			continue
		}

		wg.Add(1)
		go func(token string) {
			defer wg.Done()

			tokenPubKey, err := solana.PublicKeyFromBase58(token)
			if err != nil {
				errsMu.Lock()
				errs = append(errs, fmt.Errorf("failed to parse token %s from base58: %w", token, err))
				errsMu.Unlock()
				return
			}

			mint, err := solanaClient.GetTokenMint(ctx, tokenPubKey)
			if err != nil {
				errsMu.Lock()
				errs = append(errs, fmt.Errorf("failed to get Solana token %s mint: %w", token, err))
				errsMu.Unlock()
				return
			}

			mu.Lock()
			decimals = append(decimals, neutronclient.SolanaTokenDecimals{
				Asset:    token,
				Decimals: mint.Decimals,
			})
			if slices.Contains(solanaTotalSupplyList, tokenPubKey) {
				supplies = append(supplies, neutronclient.SolanaTokenTotalSupply{
					Asset:       token,
					TotalSupply: math.NewUint(mint.Supply),
				})
			}
			mu.Unlock()
		}(token)
	}
	wg.Wait()

	if len(errs) > 0 {
		return nil, nil, errors.Join(errs...)
	}

	return supplies, decimals, nil
}

// fetchSolanaBalances concurrently fetches balances according to the solanaBalancesList.
func fetchSolanaBalances(
	ctx context.Context,
	solanaBalancesList map[solana.PublicKey][]string,
	solanaClient SolanaClient,
) ([]neutronclient.SolanaBalance, error) {
	wg := sync.WaitGroup{}
	mu := sync.Mutex{}
	balances := make([]neutronclient.SolanaBalance, 0, len(solanaBalancesList))
	errsMu := sync.Mutex{}
	errs := make([]error, 0)
	for address, tokens := range solanaBalancesList {
		for _, token := range tokens {
			wg.Add(1)
			go func(address solana.PublicKey, token string) {
				defer wg.Done()

				var balance *solanarpc.UiTokenAmount
				var outErr error
				switch token {
				case solanaclient.SolanaNativeTokenName:
					nativeBalance, err := solanaClient.GetNativeBalance(ctx, address)
					if err != nil {
						outErr = fmt.Errorf("failed to get native balance of %s: %w", address.String(), err)
						break
					}
					balance = nativeBalance

				default:
					tokenPubKey, err := solana.PublicKeyFromBase58(token)
					if err != nil {
						outErr = fmt.Errorf("failed to parse token %s from base58: %w", token, err)
						break
					}
					tokenBalance, err := solanaClient.GetTokenAccountBalance(ctx, tokenPubKey, address)
					if err != nil {
						outErr = fmt.Errorf("failed to get account %s balance in token %s: %w", address.String(), token, err)
						break
					}
					balance = tokenBalance
				}

				if outErr != nil {
					errsMu.Lock()
					errs = append(errs, outErr)
					errsMu.Unlock()
					return
				}

				mu.Lock()
				balances = append(balances, neutronclient.SolanaBalance{
					Address: address.String(),
					Asset:   token,
					Amount:  math.NewUintFromString(balance.Amount),
				})
				mu.Unlock()
			}(address, token)
		}
	}
	wg.Wait()

	if len(errs) > 0 {
		return nil, errors.Join(errs...)
	}
	return balances, nil
}

// jupiterUsdDecimalsDivisor is the divisor for the Jupiter USD values. According to Jupiter
// team, all USD values in Jupiter are scaled to 6 decimal places (presumably to comply with
// USDC/USDT as they also have 6 decimals).
var jupiterUsdDecimalsDivisor = uint64(stdmath.Pow10(6))
