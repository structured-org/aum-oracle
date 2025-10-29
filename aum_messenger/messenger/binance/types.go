package binance

import (
	"context"
	"errors"
	"fmt"
	"slices"
	"strconv"
	"sync"

	"cosmossdk.io/math"
	binance "github.com/adshao/go-binance/v2"
	binanceportfolio "github.com/adshao/go-binance/v2/portfolio"
	neutronclient "github.com/structured-org/aum-messenger/pkg/client/neutron"
	solanaclient "github.com/structured-org/aum-messenger/pkg/client/solana"
)

// Config is the configuration for a Binance Oracle Messenger.
type Config struct {
	// UmPositionsList is the list of USD-margined portfolio perpetual futures positions to query
	// information about.
	UmPositionsList []string
	// SpotAssetsList is the list of spot assets to query information about.
	SpotAssetsList []string
}

// BinanceAumData is a data structure that aggregates all data retrieved from Binance and needed
// to compose the result payload for an AUM contract.
type BinanceAumData struct {
	// Unimmr is the Unified Account Maintenance Margin Ratio. It is the overall risk measure of
	// the entire portfolio.
	UniMMR string
	// UmPositions is the list of positions in the Binance Portfolio Margin account.
	UmPositions []*binanceportfolio.UMPosition
	// UmBalanceUsdt is the USDT balance on Portfolio Margin account. Used for perpetual futures
	// funding payments.
	UmBalanceUsdt string
	// SpotBalances is the list of balances on the Binance spot account.
	SpotBalances []*binance.Balance
	// PmAccountActualEquity is the actual equity of the Portfolio Margin account that also
	// represents the total collateral on the account.
	PmAccountActualEquity string
	// WithdrawableUsdt is the max withdrawable amount in USDT allowed without making UniMMR
	// going under 1.05 which is when liquidation happens.
	WithdrawableUsdt string
}

// ToNeutronAumData converts BinanceAumData to BinanceAumData representation that is used by the
// Neutron AUM contract.
func (d *BinanceAumData) ToNeutronAumData() (*neutronclient.BinanceAumData, error) {
	unimmr, err := math.LegacyNewDecFromStr(d.UniMMR)
	if err != nil {
		return nil, fmt.Errorf("failed to parse UniMMR %s: %w", d.UniMMR, err)
	}

	umPositions, err := d.umPositionsToNeutronPositions()
	if err != nil {
		return nil, fmt.Errorf("failed to parse UM positions: %w", err)
	}

	spotBalances, err := d.spotBalancesToNeutronBalances()
	if err != nil {
		return nil, fmt.Errorf("failed to parse spot balances: %w", err)
	}

	pmAccountActualEquity, err := math.LegacyNewDecFromStr(d.PmAccountActualEquity)
	if err != nil {
		return nil, fmt.Errorf("failed to parse PM account actual equity %s: %w", d.PmAccountActualEquity, err)
	}

	withdrawableUsdt, err := math.LegacyNewDecFromStr(d.WithdrawableUsdt)
	if err != nil {
		return nil, fmt.Errorf("failed to parse PM account withdrawable amount %s: %w", d.WithdrawableUsdt, err)
	}

	umBalanceUsdt, err := math.LegacyNewDecFromStr(d.UmBalanceUsdt)
	if err != nil {
		return nil, fmt.Errorf("failed to parse PM account USDT balance %s: %w", d.UmBalanceUsdt, err)
	}

	return &neutronclient.BinanceAumData{
		Unimmr:                unimmr,
		Positions:             umPositions,
		UmBalanceUsdt:         umBalanceUsdt,
		SpotBalances:          spotBalances,
		PmAccountActualEquity: pmAccountActualEquity,
		WithdrawableUsdt:      withdrawableUsdt,
	}, nil
}

// umPositionsToNeutronPositions converts d.UmPositions to Binance positions representation
// that is used by the Neutron AUM receiver contract.
func (d *BinanceAumData) umPositionsToNeutronPositions() ([]neutronclient.BinancePosition, error) {
	positions := make([]neutronclient.BinancePosition, 0)
	for _, position := range d.UmPositions {
		amount, err := math.LegacyNewDecFromStr(position.PositionAmt)
		if err != nil {
			return nil, fmt.Errorf("failed to parse UM position %s amount %s: %w", position.Symbol, position.PositionAmt, err)
		}

		pnl, err := math.LegacyNewDecFromStr(position.UnrealizedProfit)
		if err != nil {
			return nil, fmt.Errorf("failed to parse UM position %s PnL %s: %w", position.Symbol, position.UnrealizedProfit, err)
		}

		positions = append(positions, neutronclient.BinancePosition{
			Symbol: position.Symbol,
			Amount: amount,
			Pnl:    pnl,
		})
	}

	return positions, nil
}

// spotBalancesToNeutronBalances converts d.SpotBalances to Binance balances representation
// that is used by the Neutron AUM receiver contract.
func (d *BinanceAumData) spotBalancesToNeutronBalances() ([]neutronclient.BinanceBalance, error) {
	balances := make([]neutronclient.BinanceBalance, 0)
	for _, balance := range d.SpotBalances {
		free, err := math.LegacyNewDecFromStr(balance.Free)
		if err != nil {
			return nil, fmt.Errorf("failed to parse spot balance %s free amount %s: %w", balance.Asset, balance.Free, err)
		}

		locked, err := math.LegacyNewDecFromStr(balance.Locked)
		if err != nil {
			return nil, fmt.Errorf("failed to parse spot balance %s locked amount %s: %w", balance.Asset, balance.Locked, err)
		}

		balances = append(balances, neutronclient.BinanceBalance{
			Asset:  balance.Asset,
			Amount: free.Add(locked),
		})
	}

	return balances, nil
}

// ToJupiterAumData converts BinanceAumData to JupiterAumData representation that is used by the
// Jupiter AUM receiver contract.
func (d *BinanceAumData) ToJupiterAumData() (*solanaclient.BinanceAumData, error) {
	unimmr, err := strconv.ParseFloat(d.UniMMR, 64)
	if err != nil {
		return nil, fmt.Errorf("failed to parse UniMMR %s: %w", d.UniMMR, err)
	}

	umPositions, err := d.umPositionsToSolanaPositions()
	if err != nil {
		return nil, fmt.Errorf("failed to parse UM positions: %w", err)
	}

	spotBalances, err := d.spotBalancesToSolanaBalances()
	if err != nil {
		return nil, fmt.Errorf("failed to parse spot balances: %w", err)
	}

	pmAccountActualEquity, err := strconv.ParseFloat(d.PmAccountActualEquity, 64)
	if err != nil {
		return nil, fmt.Errorf("failed to parse PM account actual equity %s: %w", d.PmAccountActualEquity, err)
	}

	withdrawableUsdt, err := strconv.ParseFloat(d.WithdrawableUsdt, 64)
	if err != nil {
		return nil, fmt.Errorf("failed to parse PM account withdrawable amount %s: %w", d.WithdrawableUsdt, err)
	}

	umBalanceUsdt, err := strconv.ParseFloat(d.UmBalanceUsdt, 64)
	if err != nil {
		return nil, fmt.Errorf("failed to parse PM account USDT balance %s: %w", d.UmBalanceUsdt, err)
	}

	return &solanaclient.BinanceAumData{
		Unimmr:                unimmr,
		Positions:             umPositions,
		UmBalanceUsdt:         umBalanceUsdt,
		SpotBalances:          spotBalances,
		PmAccountActualEquity: pmAccountActualEquity,
		WithdrawableUsdt:      withdrawableUsdt,
	}, nil
}

// umPositionsToSolanaPositions converts d.UmPositions to Solana positions representation
// that is used by the Solana AUM receiver contract.
func (d *BinanceAumData) umPositionsToSolanaPositions() ([]solanaclient.BinancePosition, error) {
	positions := make([]solanaclient.BinancePosition, 0)
	for _, position := range d.UmPositions {
		amount, err := strconv.ParseFloat(position.PositionAmt, 64)
		if err != nil {
			return nil, fmt.Errorf("failed to parse UM position %s amount %s: %w", position.Symbol, position.PositionAmt, err)
		}

		pnl, err := strconv.ParseFloat(position.UnrealizedProfit, 64)
		if err != nil {
			return nil, fmt.Errorf("failed to parse UM position %s PnL %s: %w", position.Symbol, position.UnrealizedProfit, err)
		}

		positions = append(positions, solanaclient.BinancePosition{
			Symbol: position.Symbol,
			Amount: amount,
			Pnl:    pnl,
		})
	}

	return positions, nil
}

// spotBalancesToSolanaBalances converts d.SpotBalances to Solana balances representation
// that is used by the Solana AUM receiver contract.
func (d *BinanceAumData) spotBalancesToSolanaBalances() ([]solanaclient.BinanceBalance, error) {
	balances := make([]solanaclient.BinanceBalance, 0)
	for _, balance := range d.SpotBalances {
		free, err := strconv.ParseFloat(balance.Free, 64)
		if err != nil {
			return nil, fmt.Errorf("failed to parse spot balance %s free amount %s: %w", balance.Asset, balance.Free, err)
		}

		locked, err := strconv.ParseFloat(balance.Locked, 64)
		if err != nil {
			return nil, fmt.Errorf("failed to parse spot balance %s locked amount %s: %w", balance.Asset, balance.Locked, err)
		}

		balances = append(balances, solanaclient.BinanceBalance{
			Asset:  balance.Asset,
			Amount: free + locked,
		})
	}

	return balances, nil
}

// fetchBinanceData concurrently fetches all required data from Binance using the Binance client.
func fetchBinanceData(
	ctx context.Context,
	binanceClient BinanceClient,
	umPositionsList []string,
	spotAssetsList []string,
) (*BinanceAumData, error) {
	data := &BinanceAumData{}
	wg := sync.WaitGroup{}
	errsMu := sync.Mutex{}
	errs := make([]error, 0)

	wg.Add(1)
	go func() {
		defer wg.Done()

		umPositions, err := binanceClient.GetUmPositions(ctx)
		if err != nil {
			errsMu.Lock()
			errs = append(errs, fmt.Errorf("failed to get UM positions: %w", err))
			errsMu.Unlock()
			return
		}

		data.UmPositions = filterPositions(umPositions, umPositionsList)
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()

		spotAccountInfo, err := binanceClient.GetSpotAccountInfo(ctx)
		if err != nil {
			errsMu.Lock()
			errs = append(errs, fmt.Errorf("failed to get spot balances: %w", err))
			errsMu.Unlock()
			return
		}

		data.SpotBalances = filterBalances(spotAccountInfo.Balances, spotAssetsList)
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()

		pmAccount, err := binanceClient.GetPMAccountInfo(ctx)
		if err != nil {
			errsMu.Lock()
			errs = append(errs, fmt.Errorf("failed to get PM account info: %w", err))
			errsMu.Unlock()
			return
		}

		data.PmAccountActualEquity = pmAccount.ActualEquity
		data.WithdrawableUsdt = pmAccount.VirtualMaxWithdrawAmount
		data.UniMMR = pmAccount.UniMMR
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()

		pmAccountBalances, err := binanceClient.GetPMAccountBalance(ctx)
		if err != nil {
			errsMu.Lock()
			errs = append(errs, fmt.Errorf("failed to get PM account balances: %w", err))
			errsMu.Unlock()
			return
		}
		for _, balance := range pmAccountBalances {
			if balance.Asset == "USDT" {
				data.UmBalanceUsdt = balance.UMWalletBalance
			}
		}
	}()

	wg.Wait()
	if len(errs) > 0 {
		return nil, fmt.Errorf("some queries failed: %w", errors.Join(errs...))
	}
	return data, nil
}

// filterPositions filters out positions that are not in the requiredSymbols list.
func filterPositions(positions []*binanceportfolio.UMPosition, requiredSymbols []string) []*binanceportfolio.UMPosition {
	filteredPositions := make([]*binanceportfolio.UMPosition, 0, len(positions))
	for _, position := range positions {
		if slices.Contains(requiredSymbols, position.Symbol) {
			filteredPositions = append(filteredPositions, position)
		}
	}
	return filteredPositions
}

// filterBalances filters out balances that are not in the requiredAssets list.
func filterBalances(balances []binance.Balance, requiredAssets []string) []*binance.Balance {
	filteredBalances := make([]*binance.Balance, 0, len(balances))
	for _, balance := range balances {
		if slices.Contains(requiredAssets, balance.Asset) {
			filteredBalances = append(filteredBalances, &balance)
		}
	}
	return filteredBalances
}
