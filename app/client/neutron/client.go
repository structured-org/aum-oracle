package neutron

import (
	"context"
	"time"

	"github.com/davecgh/go-spew/spew"
	"go.uber.org/zap"
)

type Client struct {
	logger *zap.Logger
}

func NewClient(logger *zap.Logger) (*Client, error) {
	return &Client{
		logger: logger,
	}, nil
}

// TODO: use actual values when the client is implemented
var binanceRound = 0
var solanaRound = 0

func (c *Client) GetBinanceAumContractNextRound(ctx context.Context) (*NextRound, error) {
	// TODO: use actual values when the client is implemented
	return &NextRound{
		Round:     int64(binanceRound),
		Timestamp: time.Now().Add(time.Minute).Unix(),
	}, nil
}

func (c *Client) SubmitBinanceAumData(ctx context.Context, data *BinanceData) (*NextRound, error) {
	// print for debug evaluation. TODO: use actual values when the client is implemented
	spew.Dump("submitted Binance AUM data:", data)

	binanceRound++
	return &NextRound{
		Round:     int64(binanceRound),
		Timestamp: time.Now().Add(time.Minute).Unix(),
	}, nil
}

func (c *Client) GetSolanaAumContractNextRound(ctx context.Context) (*NextRound, error) {
	// TODO: use actual values when the client is implemented
	return &NextRound{
		Round:     int64(solanaRound),
		Timestamp: time.Now().Add(time.Minute).Unix(),
	}, nil
}

func (c *Client) SubmitSolanaAumData(ctx context.Context, data *SolanaData) (*NextRound, error) {
	// print for debug evaluation. TODO: use actual values when the client is implemented
	spew.Dump("submitted Solana AUM data:", data)

	solanaRound++
	return &NextRound{
		Round:     int64(solanaRound),
		Timestamp: time.Now().Add(time.Minute).Unix(),
	}, nil
}
