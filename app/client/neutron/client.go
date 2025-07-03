package neutron

import (
	"context"
	json2 "encoding/json"
	"fmt"
	"github.com/structured-org/aum-oracle/client/tm"
	"time"

	"github.com/davecgh/go-spew/spew"
	"go.uber.org/zap"
)

// Client is the Neutron client.
type Client struct {
	logger             *zap.Logger
	client             *tm.Client
	jupiterAumContract string
}

// NewClient creates a new Neutron client.
func NewClient(conf tm.ClientConfig, jupiterAumContract string, logger *zap.Logger) (*Client, error) {
	tmClient, err := tm.New(&conf, logger)
	if err != nil {
		return nil, fmt.Errorf("could not instantiate tm client: %w", err)
	}
	return &Client{
		logger:             logger,
		client:             tmClient,
		jupiterAumContract: jupiterAumContract,
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

// SubmitBinanceAumData submits the Binance AUM data to the Binance AUM contract.
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

// SubmitJupiterAumData submits the Jupiter AUM data to the Jupiter AUM contract.
func (c *Client) SubmitJupiterAumData(ctx context.Context, data *JupiterAumData) (*NextRound, error) {
	// print for debug evaluation. TODO: use actual values when the client is implemented
	spew.Dump("submitted Jupiter AUM data:", data)

	msg := MsgExecuteContract{
		// TODO
	}
	c.client.SignAndBroadcast(ctx, msg)

	jupiterRound++
	return &NextRound{
		Round:     int64(jupiterRound),
		Timestamp: time.Now().Add(time.Minute).Unix(),
	}, nil
}
