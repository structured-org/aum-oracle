package neutron

import (
	"context"
	"encoding/json"
	"fmt"
	"github.com/CosmWasm/wasmd/x/wasm/types"
	sdk "github.com/cosmos/cosmos-sdk/types"
	"github.com/davecgh/go-spew/spew"
	"github.com/structured-org/aum-oracle/client/tm"
	"go.uber.org/zap"
	"time"
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
	spew.Dump("submitted Jupiter AUM data:", data)

	// === BUILD EXECUTE MSG ===
	msgPayload := map[string]interface{}{
		"publish_data": map[string]interface{}{
			"data": data,
		},
	}
	msgBz, _ := json.Marshal(msgPayload)
	fmt.Printf("Marshalled json: %s\n", string(msgBz))
	executeMsg := &types.MsgExecuteContract{
		Sender:   c.client.GetAddress(),
		Contract: c.jupiterAumContract,
		Msg:      msgBz,
		Funds:    sdk.NewCoins(),
	}

	code, err := c.client.SignAndBroadcast(ctx, executeMsg)
	fmt.Printf("code: %v, err: %v\n", code, err)
	if err != nil {
		// TODO
	}
	if code != 0 {
		// TODO
	}

	jupiterRound++
	return &NextRound{
		Round:     int64(jupiterRound),
		Timestamp: time.Now().Add(time.Minute).Unix(),
	}, nil
}
