package binance

import (
	"context"

	binance "github.com/adshao/go-binance/v2"
	binanceportfolio "github.com/adshao/go-binance/v2/portfolio"
	msgrclient "github.com/structured-org/aum-messenger/client"
	neutronclient "github.com/structured-org/aum-messenger/client/neutron"
	solanaclient "github.com/structured-org/aum-messenger/client/solana"
)

// BinanceClient is the definition of the expected Binance client.
type BinanceClient interface {
	GetUmPositions(ctx context.Context) ([]*binanceportfolio.UMPosition, error)
	GetPMAccountInfo(ctx context.Context) (*binanceportfolio.Account, error)
	GetPMAccountBalance(ctx context.Context) ([]*binanceportfolio.Balance, error)
	GetSpotAccountInfo(ctx context.Context) (*binance.Account, error)
}

// NeutronAumReceiverClient is the definition of the expected Neutron AUM receiver client.
type NeutronAumReceiverClient interface {
	GetBinanceAumReceiverNextRound(ctx context.Context) (*msgrclient.NextRound, error)
	SubmitBinanceAumData(ctx context.Context, data *neutronclient.BinanceAumData) (*msgrclient.NextRound, error)
}

// SolanaAumReceiverClient is the definition of the expected Solana AUM receiver client.
type SolanaAumReceiverClient interface {
	GetBinanceAumReceiverNextRound(ctx context.Context) (*msgrclient.NextRound, error)
	SubmitBinanceAumData(ctx context.Context, data *solanaclient.BinanceAumData) (*msgrclient.NextRound, error)
}
