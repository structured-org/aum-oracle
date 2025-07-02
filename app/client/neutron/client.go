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
var jupiterRound = 0

// GetBinanceAumContractNextRound gets the next consensus round for the Binance AUM contract.
func (c *Client) GetBinanceAumContractNextRound(ctx context.Context) (*NextRound, error) {
	// TODO: use actual values when the client is implemented
	return &NextRound{
		Round:     int64(binanceRound),
		Timestamp: time.Now().Add(time.Minute).Unix(),
	}, nil
}

// SubmitBinanceAumData submits the Binance AUM data to the Neutron AUM contract.
func (c *Client) SubmitBinanceAumData(ctx context.Context, data *BinanceAumData) (*NextRound, error) {
	// print for debug evaluation. TODO: use actual values when the client is implemented
	spew.Dump("submitted Binance AUM data:", data)

	binanceRound++
	return &NextRound{
		Round:     int64(binanceRound),
		Timestamp: time.Now().Add(time.Minute).Unix(),
	}, nil
}

// GetJupiterAumContractNextRound gets the next consensus round for the Jupiter AUM contract.
func (c *Client) GetJupiterAumContractNextRound(ctx context.Context) (*NextRound, error) {
	// TODO: use actual values when the client is implemented
	return &NextRound{
		Round:     int64(jupiterRound),
		Timestamp: time.Now().Add(time.Second).Unix(),
	}, nil
}

// SubmitJupiterAumData submits the Jupiter AUM data to the Neutron AUM contract.
func (c *Client) SubmitJupiterAumData(ctx context.Context, data *JupiterAumData) (*NextRound, error) {
	// print for debug evaluation. TODO: use actual values when the client is implemented
	spew.Dump("submitted Jupiter AUM data:", data)

	jupiterRound++
	return &NextRound{
		Round:     int64(jupiterRound),
		Timestamp: time.Now().Add(time.Minute).Unix(),
	}, nil
}
