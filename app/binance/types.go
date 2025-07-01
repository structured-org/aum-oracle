package binance

import (
	"fmt"
	"strconv"

	"cosmossdk.io/math"
	binance "github.com/adshao/go-binance/v2"
	binanceportfolio "github.com/adshao/go-binance/v2/portfolio"
	neutronclient "github.com/structured-org/aum-oracle/client/neutron"
	solanaclient "github.com/structured-org/aum-oracle/client/solana"
)

// BinanceData is the data structure for the Binance oracle.
type BinanceData struct {
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

// ToNeutronData converts BinanceData to BinanceData representation that is used by the Neutron
// AUM contract.
func (d *BinanceData) ToNeutronData() (*neutronclient.BinanceData, error) {
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

	return &neutronclient.BinanceData{
		Unimmr:                unimmr,
		Positions:             umPositions,
		UmBalanceUsdt:         umBalanceUsdt,
		SpotBalances:          spotBalances,
		PmAccountActualEquity: pmAccountActualEquity,
		WithdrawableUsdt:      withdrawableUsdt,
	}, nil
}

// umPositionsToNeutronPositions converts d.UmPositions to Binance positions representation
// that is used by the Neutron AUM contract.
func (d *BinanceData) umPositionsToNeutronPositions() ([]neutronclient.BinancePosition, error) {
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
// that is used by the Neutron AUM contract.
func (d *BinanceData) spotBalancesToNeutronBalances() ([]neutronclient.BinanceBalance, error) {
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

// ToSolanaData converts BinanceData to SolanaData representation that is used by the Solana
// AUM contract.
func (d *BinanceData) ToSolanaData() (*solanaclient.BinanceData, error) {
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

	return &solanaclient.BinanceData{
		Unimmr:                unimmr,
		Positions:             umPositions,
		UmBalanceUsdt:         umBalanceUsdt,
		SpotBalances:          spotBalances,
		PmAccountActualEquity: pmAccountActualEquity,
		WithdrawableUsdt:      withdrawableUsdt,
	}, nil
}

// umPositionsToSolanaPositions converts d.UmPositions to Solana positions representation
// that is used by the Solana AUM contract.
func (d *BinanceData) umPositionsToSolanaPositions() ([]solanaclient.BinancePosition, error) {
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
// that is used by the Solana AUM contract.
func (d *BinanceData) spotBalancesToSolanaBalances() ([]solanaclient.BinanceBalance, error) {
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
