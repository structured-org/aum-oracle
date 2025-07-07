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
	binanceAumContract string
}

// NewClient creates a new Neutron client.
func NewClient(conf tm.ClientConfig, jupiterAumContract string, binanceAumContract string, logger *zap.Logger) (*Client, error) {
	tmClient, err := tm.New(&conf, logger)
	if err != nil {
		return nil, fmt.Errorf("could not instantiate tm client: %w", err)
	}
	return &Client{
		logger:             logger,
		client:             tmClient,
		jupiterAumContract: jupiterAumContract,
		binanceAumContract: binanceAumContract,
	}, nil
}

// TODO: use actual values when the client is implemented
var binanceRound = 0
var jupiterRound = 0

// GetBinanceAumContractNextRound gets the next consensus round for the Binance AUM contract.
func (c *Client) GetBinanceAumContractNextRound(ctx context.Context) (*NextRound, error) {
	// TODO: use actual values when the client is implemented
	return &NextRound{
		Round:     uint64(binanceRound),
		Timestamp: time.Now().Add(time.Second * 10).Unix(),
	}, nil
}

// SubmitBinanceAumData submits the Binance AUM data to the Binance AUM contract.
func (c *Client) SubmitBinanceAumData(ctx context.Context, data *BinanceAumData) (*NextRound, error) {
	spew.Dump("submitted Binance AUM data:", data)
	c.logger.Info("submitting binance aum data")

	msgPayload := map[string]interface{}{
		"publish_data": map[string]interface{}{
			"data": data,
		},
	}
	msgBz, _ := json.Marshal(msgPayload)
	executeMsg := &types.MsgExecuteContract{
		Sender:   c.client.GetAddress(),
		Contract: c.binanceAumContract,
		Msg:      msgBz,
		Funds:    sdk.NewCoins(),
	}
	res, err := c.client.SignAndBroadcast(ctx, executeMsg)
	if err != nil {
		return nil, fmt.Errorf("failed to sign and broadcast tx during submit data: %w", err)
	}
	c.logger.Info("submitted binance aum data",
		zap.Uint32("code", res.TxResult.Code),
		zap.String("hash", res.Hash.String()), // TODO: check that hex output?
		zap.Int64("height", res.Height))

	binanceRound++
	return &NextRound{
		Round:     uint64(binanceRound),
		Timestamp: time.Now().Add(time.Minute).Unix(),
	}, nil
}

// GetJupiterAumContractNextRound gets the next consensus round for the Jupiter AUM contract.
func (c *Client) GetJupiterAumContractNextRound(ctx context.Context) (*NextRound, error) {
	msgPayload := map[string]interface{}{
		"get_round": map[string]interface{}{},
	}
	msgBz, _ := json.Marshal(msgPayload)
	resBz, err := c.client.QuerySmartContract(ctx, c.jupiterAumContract, msgBz)
	if err != nil {
		return nil, fmt.Errorf("failed to query jupiter aum smart contract for next round: %w", err)
	}
	var response GetRoundResponse
	if err := json.Unmarshal(resBz, &response); err != nil {
		return nil, fmt.Errorf("failed to unmarshal next round response fro jupiter aum contract: %w", err)
	}

	return &NextRound{
		Round:     response.NextRound.Round,
		Timestamp: time.Now().Add(time.Second * 10000).Unix(), // Turn off for now
	}, nil
}

// SubmitJupiterAumData submits the Jupiter AUM data to the Jupiter AUM contract.
func (c *Client) SubmitJupiterAumData(ctx context.Context, data *JupiterAumData) (*NextRound, error) {
	//spew.Dump("submitted Jupiter AUM data:", data)
	c.logger.Info("submitting jupiter aum data")

	msgPayload := map[string]interface{}{
		"publish_data": map[string]interface{}{
			"data": data,
		},
	}
	msgBz, _ := json.Marshal(msgPayload)
	executeMsg := &types.MsgExecuteContract{
		Sender:   c.client.GetAddress(),
		Contract: c.jupiterAumContract,
		Msg:      msgBz,
		Funds:    sdk.NewCoins(),
	}

	res, err := c.client.SignAndBroadcast(ctx, executeMsg)
	if err != nil {
		return nil, fmt.Errorf("failed to sign and broadcast tx during submit data: %w", err)
	}
	c.logger.Info("submitted jupiter aum data",
		zap.Uint32("code", res.TxResult.Code),
		zap.String("hash", res.Hash.String()), // TODO: check that hex output?
		zap.Int64("height", res.Height))

	jupiterRound++
	return &NextRound{
		Round:     uint64(jupiterRound),
		Timestamp: time.Now().Add(time.Minute).Unix(),
	}, nil
}

type GetRoundResponse struct {
	/// PendingRound is a currently pending round.
	PendingRound Round `json:"pending_round"`
	/// NextRound is the next round.
	NextRound Round `json:"next_round"`
}

type Round struct {
	/// Round is a number of the round.
	Round uint64 `json:"round"`
	/// Start is when the round started (UNIX timestamp in seconds).
	Start uint64 `json:"start"`
}
