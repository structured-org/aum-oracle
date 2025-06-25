package binance

import (
	"context"

	binance "github.com/adshao/go-binance/v2"
	binanceportfolio "github.com/adshao/go-binance/v2/portfolio"
	neutronclient "github.com/structured-org/aum-oracle/client/neutron"
)

type BinanceClient interface {
	GetUmPositions(ctx context.Context) ([]*binanceportfolio.UMPosition, error)
	GetCmPositions(ctx context.Context) ([]*binanceportfolio.CMPosition, error)
	GetPMAccountInfo(ctx context.Context) (*binanceportfolio.Account, error)
	GetPMAccountBalance(ctx context.Context) ([]*binanceportfolio.Balance, error)
	GetSpotAccountInfo(ctx context.Context) (*binance.Account, error)
}

type NeutronClient interface {
	GetBinanceAumContractNextRound(ctx context.Context) (*neutronclient.NextRound, error)
	SubmitBinanceAumData(ctx context.Context, data *neutronclient.BinanceData) (*neutronclient.NextRound, error)
}
