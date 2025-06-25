package neutron

import (
	"context"
	"time"

	"github.com/davecgh/go-spew/spew"
	"go.uber.org/zap"
)

// Client is the Neutron client.
type Client struct {
	logger *zap.Logger
}

// NewClient creates a new Neutron client.
func NewClient(logger *zap.Logger) (*Client, error) {
	return &Client{
		logger: logger,
	}, nil
}

// TODO: use actual values when the client is implemented
var binanceRound = 0
var solanaRound = 0

// GetBinanceAumContractNextRound gets the next consensus round for the Binance AUM contract.
func (c *Client) GetBinanceAumContractNextRound(ctx context.Context) (*NextRound, error) {
	// TODO: use actual values when the client is implemented
	return &NextRound{
		Round:     int64(binanceRound),
		Timestamp: time.Now().Add(time.Minute).Unix(),
	}, nil
}

// SubmitBinanceAumData submits the Binance AUM data to the Neutron AUM contract.
func (c *Client) SubmitBinanceAumData(ctx context.Context, data *BinanceData) (*NextRound, error) {
	// print for debug evaluation. TODO: use actual values when the client is implemented
	spew.Dump("submitted Binance AUM data:", data)

	binanceRound++
	return &NextRound{
		Round:     int64(binanceRound),
		Timestamp: time.Now().Add(time.Minute).Unix(),
	}, nil
}

// GetSolanaAumContractNextRound gets the next consensus round for the Solana AUM contract.
func (c *Client) GetSolanaAumContractNextRound(ctx context.Context) (*NextRound, error) {
	// TODO: use actual values when the client is implemented
	return &NextRound{
		Round:     int64(solanaRound),
		Timestamp: time.Now().Add(time.Second).Unix(),
	}, nil
}

// SubmitSolanaAumData submits the Solana AUM data to the Neutron AUM contract.
func (c *Client) SubmitSolanaAumData(ctx context.Context, data *SolanaData) (*NextRound, error) {
	// print for debug evaluation. TODO: use actual values when the client is implemented
	spew.Dump("submitted Solana AUM data:", data)

	solanaRound++
	return &NextRound{
		Round:     int64(solanaRound),
		Timestamp: time.Now().Add(time.Minute).Unix(),
	}, nil
}
