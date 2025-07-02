package binance

import (
	"context"

	binance "github.com/adshao/go-binance/v2"
	binanceportfolio "github.com/adshao/go-binance/v2/portfolio"
	neutronclient "github.com/structured-org/aum-oracle/client/neutron"
	solanaclient "github.com/structured-org/aum-oracle/client/solana"
)

// BinanceClient is the definition of the expected Binance client.
type BinanceClient interface {
	GetUmPositions(ctx context.Context) ([]*binanceportfolio.UMPosition, error)
	GetCmPositions(ctx context.Context) ([]*binanceportfolio.CMPosition, error)
	GetPMAccountInfo(ctx context.Context) (*binanceportfolio.Account, error)
	GetPMAccountBalance(ctx context.Context) ([]*binanceportfolio.Balance, error)
	GetSpotAccountInfo(ctx context.Context) (*binance.Account, error)
}

// NeutronAumContractClient is the definition of the expected Neutron AUM contract client.
type NeutronAumContractClient interface {
	GetBinanceAumContractNextRound(ctx context.Context) (*neutronclient.NextRound, error)
	SubmitBinanceAumData(ctx context.Context, data *neutronclient.BinanceAumData) (*neutronclient.NextRound, error)
}

// SolanaAumContractClient is the definition of the expected Solana AUM contract client.
type SolanaAumContractClient interface {
	GetBinanceAumContractNextRound(ctx context.Context) (*solanaclient.NextRound, error)
	SubmitBinanceAumData(ctx context.Context, data *solanaclient.BinanceAumData) (*solanaclient.NextRound, error)
}
