package neutron

import (
	"context"
	"encoding/json"
	"fmt"
	"github.com/CosmWasm/wasmd/x/wasm/types"
	comettypes "github.com/cometbft/cometbft/abci/types"
	cometcoretypes "github.com/cometbft/cometbft/rpc/core/types"
	sdk "github.com/cosmos/cosmos-sdk/types"
	"github.com/structured-org/aum-oracle/utils"
	"go.uber.org/zap"
	"strconv"
)

const (
	eventTypeWasm          = "wasm"
	attrNextRound          = "next_round"
	attrNextRoundTimestamp = "next_round_timestamp"
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
	resp, err := c.queryRoundInfo(ctx, c.binanceAumContract, "binance")
	if err != nil {
		return nil, err
	}
	return &NextRound{
		Round:     resp.NextRound.Round,
		Timestamp: resp.NextRound.Start,
	}, nil
}

// GetJupiterAumContractNextRound gets the next consensus round for the Jupiter AUM contract.
func (c *Client) GetJupiterAumContractNextRound(ctx context.Context) (*NextRound, error) {
	resp, err := c.queryRoundInfo(ctx, c.jupiterAumContract, "jupiter")
	if err != nil {
		return nil, err
	}
	return &NextRound{
		Round:     resp.NextRound.Round,
		Timestamp: resp.NextRound.Start,
	}, nil
}

// SubmitBinanceAumData submits the Binance AUM data to the Binance AUM contract.
func (c *Client) SubmitBinanceAumData(ctx context.Context, data *BinanceAumData) (*NextRound, error) {
	c.logger.Info("submitting binance aum data")
	msg := map[string]interface{}{
		"publish_data": map[string]interface{}{
			"new_data": data,
		},
	}
	resp, err := c.sendExecuteMsg(ctx, c.binanceAumContract, msg, "binance")
	if err != nil {
		return nil, err
	}
	c.logger.Info("submitted binance aum data",
		zap.Uint32("code", resp.TxResult.Code),
		zap.String("tx_hash", resp.Hash.String()),
		zap.Int64("height", resp.Height),
		zap.String("contract", "binance"),
	)

	nextRound, err := GetNextRoundFromEvents(resp.TxResult.GetEvents())
	if err != nil {
		return nil, fmt.Errorf("binance: failed to extract next round from events: %w", err)
	}
	c.logger.Info("got next round for binance oracle from events")
	return nextRound, nil
}

// SubmitJupiterAumData submits the Jupiter AUM data to the Jupiter AUM contract.
func (c *Client) SubmitJupiterAumData(ctx context.Context, data *JupiterAumData) (*NextRound, error) {
	c.logger.Info("submitting jupiter aum data")
	msg := map[string]interface{}{
		"publish_data": map[string]interface{}{
			"new_data": data,
		},
	}
	resp, err := c.sendExecuteMsg(ctx, c.jupiterAumContract, msg, "jupiter")
	if err != nil {
		return nil, err
	}
	c.logger.Info("submitted jupiter aum data",
		zap.Uint32("code", resp.TxResult.Code),
		zap.String("tx_hash", resp.Hash.String()),
		zap.Int64("height", resp.Height),
		zap.String("contract", "jupiter"),
	)

	nextRound, err := GetNextRoundFromEvents(resp.TxResult.GetEvents())
	if err != nil {
		return nil, fmt.Errorf("jupiter: failed to extract next round from events: %w", err)
	}
	c.logger.Info("got next round for jupiter oracle from events")
	return nextRound, nil
}

// internal: query round info from smart contract
func (c *Client) queryRoundInfo(ctx context.Context, contract string, label string) (*GetRoundResponse, error) {
	msg := map[string]interface{}{"get_round_info": struct{}{}}
	msgBz, err := json.Marshal(msg)
	if err != nil {
		return nil, fmt.Errorf("%s: failed to marshal get_round_info: %w", label, err)
	}
	resBz, err := c.client.QuerySmartContract(ctx, contract, msgBz)
	if err != nil {
		return nil, fmt.Errorf("%s: failed to query smart contract: %w", label, err)
	}
	var response GetRoundResponse
	if err := json.Unmarshal(resBz, &response); err != nil {
		return nil, fmt.Errorf("%s: failed to unmarshal get_round_info response: %w", label, err)
	}
	return &response, nil
}

// internal: execute smart contract message
func (c *Client) sendExecuteMsg(ctx context.Context, contract string, msg any, label string) (*cometcoretypes.ResultBroadcastTxCommit, error) {
	msgBz, err := json.Marshal(msg)
	if err != nil {
		return nil, fmt.Errorf("%s: failed to marshal execute message: %w", label, err)
	}
	execute := &types.MsgExecuteContract{
		Sender:   c.client.GetAddress(),
		Contract: contract,
		Msg:      msgBz,
		Funds:    sdk.NewCoins(),
	}
	resp, err := c.client.SignAndBroadcast(ctx, execute)
	if err != nil {
		return nil, fmt.Errorf("%s: failed to sign and broadcast transaction: %w", label, err)
	}
	return resp, nil
}

// GetNextRoundFromEvents parses wasm events to extract next round info.
func GetNextRoundFromEvents(events []comettypes.Event) (*NextRound, error) {
	var nextRoundStr, nextRoundTimestampStr string

	for _, evt := range events {
		if evt.Type != eventTypeWasm {
			continue
		}
		for _, attr := range evt.Attributes {
			switch attr.Key {
			case attrNextRound:
				nextRoundStr = attr.Value
			case attrNextRoundTimestamp:
				nextRoundTimestampStr = attr.Value
			}
		}
	}

	nextRound, err := strconv.ParseUint(nextRoundStr, 10, 64)
	if err != nil {
		return nil, fmt.Errorf("failed to parse %q from events: %w", attrNextRound, err)
	}
	nextRoundTimestamp, err := strconv.ParseUint(nextRoundTimestampStr, 10, 64)
	if err != nil {
		return nil, fmt.Errorf("failed to parse %q from events: %w", attrNextRoundTimestamp, err)
	}

	return &NextRound{
		Round:     nextRound,
		Timestamp: nextRoundTimestamp,
	}, nil
}
