package neutron

import (
	"context"
	"encoding/json"
	"fmt"
	"github.com/CosmWasm/wasmd/x/wasm/types"
	comettypes "github.com/cometbft/cometbft/abci/types"
	sdk "github.com/cosmos/cosmos-sdk/types"
	"github.com/structured-org/aum-oracle/utils"
	"go.uber.org/zap"
	"strconv"
)

const (
	nextRoundTimestampAttr = "next_round_timestamp"
	nextRoundAttr          = "next_round"
)

// Client is the Neutron client.
type Client struct {
	logger             *zap.Logger
	client             *utils.CosmosClient
	jupiterAumContract string
	binanceAumContract string
}

// NewClient creates a new Neutron client.
func NewClient(conf utils.CosmosClientConfig, jupiterAumContract string, binanceAumContract string, logger *zap.Logger) (*Client, error) {
	client, err := utils.New(&conf, logger)
	if err != nil {
		return nil, fmt.Errorf("could not instantiate cosmos client: %w", err)
	}
	return &Client{
		logger:             logger,
		client:             client,
		jupiterAumContract: jupiterAumContract,
		binanceAumContract: binanceAumContract,
	}, nil
}

// GetBinanceAumContractNextRound gets the next consensus round for the Binance AUM contract.
func (c *Client) GetBinanceAumContractNextRound(ctx context.Context) (*NextRound, error) {
	msgPayload := map[string]interface{}{
		"get_round_info": map[string]interface{}{},
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
		Timestamp: response.NextRound.Start,
	}, nil
}

// SubmitBinanceAumData submits the Binance AUM data to the Binance AUM contract.
func (c *Client) SubmitBinanceAumData(ctx context.Context, data *BinanceAumData) (*NextRound, error) {
	c.logger.Info("submitting binance aum data")

	msgPayload := map[string]interface{}{
		"publish_data": map[string]interface{}{
			"new_data": data,
		},
	}
	msgBz, err := json.Marshal(msgPayload)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal publish data message: %w", err)
	}

	executeMsg := &types.MsgExecuteContract{
		Sender:   c.client.GetAddress(),
		Contract: c.binanceAumContract,
		Msg:      msgBz,
		Funds:    sdk.NewCoins(),
	}
	res, err := c.client.SignAndBroadcast(ctx, executeMsg)
	if err != nil {
		return nil, fmt.Errorf("failed to sign and broadcast binance publish tx during publish_data: %w", err)
	}
	c.logger.Info("submitted binance aum data",
		zap.Uint32("code", res.TxResult.Code),
		zap.String("hash", res.Hash.String()),
		zap.Int64("height", res.Height))

	events := res.TxResult.GetEvents()
	nextRound, err := GetNextRoundFromEvents(events)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch next data from events: %w", err)
	}
	c.logger.Info("got next round for binance oracle from publish data response events")
	return nextRound, nil
}

// GetJupiterAumContractNextRound gets the next consensus round for the Jupiter AUM contract.
func (c *Client) GetJupiterAumContractNextRound(ctx context.Context) (*NextRound, error) {
	msgPayload := map[string]interface{}{
		"get_round_info": map[string]interface{}{},
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
		Timestamp: response.NextRound.Start,
	}, nil
}

// SubmitJupiterAumData submits the Jupiter AUM data to the Jupiter AUM contract.
func (c *Client) SubmitJupiterAumData(ctx context.Context, data *JupiterAumData) (*NextRound, error) {
	c.logger.Info("submitting jupiter aum data")

	msgPayload := map[string]interface{}{
		"publish_data": map[string]interface{}{
			"new_data": data,
		},
	}
	msgBz, err := json.Marshal(msgPayload)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal jupiter publish data message: %w", err)
	}

	executeMsg := &types.MsgExecuteContract{
		Sender:   c.client.GetAddress(),
		Contract: c.jupiterAumContract,
		Msg:      msgBz,
		Funds:    sdk.NewCoins(),
	}

	res, err := c.client.SignAndBroadcast(ctx, executeMsg)
	if err != nil {
		return nil, fmt.Errorf("failed to sign and broadcast jupiter publish tx during publish_data: %w", err)
	}
	c.logger.Info("submitted jupiter aum data",
		zap.Uint32("code", res.TxResult.Code),
		zap.String("hash", res.Hash.String()),
		zap.Int64("height", res.Height))

	events := res.TxResult.GetEvents()
	nextRound, err := GetNextRoundFromEvents(events)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch next round for jupiter from events: %w", err)
	}
	c.logger.Info("got next round for jupiter oracle from publish data response events")

	return nextRound, nil
}

func GetNextRoundFromEvents(events []comettypes.Event) (*NextRound, error) {
	var nextRoundStr, nextRoundTimestampStr string

	for _, evt := range events {
		if evt.Type != "wasm" {
			continue
		}

		for _, attr := range evt.Attributes {
			switch attr.Key {
			case nextRoundAttr:
				nextRoundStr = attr.Value
			case nextRoundTimestampAttr:
				nextRoundTimestampStr = attr.Value
			}
		}
	}

	nextRound, err := strconv.ParseUint(nextRoundStr, 10, 64)
	if err != nil {
		return nil, fmt.Errorf("failed to parse next_round from events: %w", err)
	}
	nextRoundTimestamp, err := strconv.ParseUint(nextRoundTimestampStr, 10, 64)
	if err != nil {
		return nil, fmt.Errorf("failed to parse next_round_timestamp from events: %w", err)
	}

	return &NextRound{
		Round:     nextRound,
		Timestamp: nextRoundTimestamp,
	}, nil
}
