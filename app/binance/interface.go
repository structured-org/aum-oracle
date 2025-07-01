package binance

import (
	"context"

	binance "github.com/adshao/go-binance/v2"
	binanceportfolio "github.com/adshao/go-binance/v2/portfolio"
	neutronclient "github.com/structured-org/aum-oracle/client/neutron"
	solanaclient "github.com/structured-org/aum-oracle/client/solana"
)

// BinanceClient is the definition of the expected Binance client interface.
type BinanceClient interface {
	GetUmPositions(ctx context.Context) ([]*binanceportfolio.UMPosition, error)
	GetCmPositions(ctx context.Context) ([]*binanceportfolio.CMPosition, error)
	GetPMAccountInfo(ctx context.Context) (*binanceportfolio.Account, error)
	GetPMAccountBalance(ctx context.Context) ([]*binanceportfolio.Balance, error)
	GetSpotAccountInfo(ctx context.Context) (*binance.Account, error)
}

// NeutronAumContractClient is the definition of the expected Neutron AUM contract client interface.
type NeutronAumContractClient interface {
	// GetBinanceAumContractNextRound gets the next consensus round for the Binance AUM contract.
	GetBinanceAumContractNextRound(ctx context.Context) (*neutronclient.NextRound, error)
	// SubmitBinanceAumData submits the Binance AUM data to the Neutron AUM contract.
	SubmitBinanceAumData(ctx context.Context, data *neutronclient.BinanceData) (*neutronclient.NextRound, error)
}

// SolanaAumContractClient is the definition of the expected Solana AUM contract client interface.
type SolanaAumContractClient interface {
	// GetBinanceAumContractNextRound gets the next consensus round for the Binance AUM contract.
	GetBinanceAumContractNextRound(ctx context.Context) (*solanaclient.NextRound, error)
	// SubmitBinanceAumData submits the Binance AUM data to the Solana AUM contract.
	SubmitBinanceAumData(ctx context.Context, data *solanaclient.BinanceData) (*solanaclient.NextRound, error)
}
