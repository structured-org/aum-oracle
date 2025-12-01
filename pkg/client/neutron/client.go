package neutron

import (
	"context"
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/CosmWasm/wasmd/x/wasm/types"
	"github.com/avast/retry-go/v4"
	comettypes "github.com/cometbft/cometbft/abci/types"
	cometcoretypes "github.com/cometbft/cometbft/rpc/core/types"
	sdk "github.com/cosmos/cosmos-sdk/types"
	cosmosclient "github.com/structured-org/aum-messenger/pkg/client/cosmos"
	"go.uber.org/zap"
)

const (
	eventTypeWasm          = "wasm"
	attrNextRound          = "next_round"
	attrNextRoundTimestamp = "next_round_timestamp"
)

// Client is the Neutron client.
type Client struct {
	logger             *zap.Logger
	client             *cosmosclient.CosmosClient
	jupiterAumContract string
	binanceAumContract string
	twaerContract      string
}

// NewClient creates a new Neutron client.
func NewClient(conf cosmosclient.Config, twaerContract string, jupiterAumContract string, binanceAumContract string, logger *zap.Logger) (*Client, error) {
	client, err := cosmosclient.NewClient(&conf, logger)
	if err != nil {
		return nil, fmt.Errorf("could not instantiate cosmos client: %w", err)
	}
	return &Client{
		logger:             logger,
		client:             client,
		jupiterAumContract: jupiterAumContract,
		binanceAumContract: binanceAumContract,
		twaerContract:      twaerContract,
	}, nil
}

// GetBinanceAumReceiverNextRound gets the next consensus round for the Binance AUM receiver contract.
func (c *Client) GetBinanceAumReceiverNextRound(ctx context.Context) (*NextRound, error) {
	resp, err := c.queryRoundInfo(ctx, c.binanceAumContract)
	if err != nil {
		return nil, fmt.Errorf("can't query round info for Binance contract: %w", err)
	}
	return &NextRound{
		Round:     resp.NextRound.Round,
		Timestamp: resp.NextRound.Start,
	}, nil
}

// GetJupiterAumReceiverNextRound gets the next consensus round for the Jupiter AUM receiver contract.
func (c *Client) GetJupiterAumReceiverNextRound(ctx context.Context) (*NextRound, error) {
	resp, err := c.queryRoundInfo(ctx, c.jupiterAumContract)
	if err != nil {
		return nil, fmt.Errorf("can't query round info for Jupiter contract: %w", err)
	}
	return &NextRound{
		Round:     resp.NextRound.Round,
		Timestamp: resp.NextRound.Start,
	}, nil
}

// SubmitBinanceAumData submits the Binance AUM data to the Binance AUM receiver contract.
func (c *Client) SubmitBinanceAumData(ctx context.Context, data *BinanceAumData) (*NextRound, error) {
	c.logger.Info("submitting binance aum data")
	msg := map[string]any{
		"publish_data": map[string]any{
			"new_data": data,
		},
	}

	var resp *cometcoretypes.ResultBroadcastTxCommit
	var err error

	err = retry.Do(
		func() error {
			var innerError error
			resp, innerError = c.sendExecuteMsg(ctx, c.binanceAumContract, msg)
			return innerError
		},
		// Configuration options
		retry.Attempts(3),                   // Try 3 times
		retry.Delay(1*time.Second),          // Initial delay
		retry.DelayType(retry.BackOffDelay), // Use exponential backoff
		retry.OnRetry(func(n uint, err error) {
			c.logger.Info("Retry publish binance data attempt one more time")
		}),
	)
	if err != nil {
		return nil, err
	}

	c.logger.Info("submitted binance aum data",
		zap.Uint32("code", resp.TxResult.Code),
		zap.String("tx_hash", resp.Hash.String()),
		zap.Int64("height", resp.Height),
		zap.String("contract", "binance"),
		zap.Int64("tx_gas_used", resp.TxResult.GasUsed),
	)

	nextRound, err := GetNextRoundFromEvents(resp.TxResult.GetEvents())
	if err != nil {
		return nil, fmt.Errorf("binance: failed to extract next round from events: %w", err)
	}
	c.logger.Info("got next round for binance receiver from events")
	return nextRound, nil
}

// SubmitJupiterAumData submits the Jupiter AUM data to the Jupiter AUM receiver contract.
func (c *Client) SubmitJupiterAumData(ctx context.Context, data *JupiterAumData) (*NextRound, error) {
	c.logger.Info("submitting jupiter aum data")
	msg := map[string]any{
		"publish_data": map[string]any{
			"new_data": data,
		},
	}

	var resp *cometcoretypes.ResultBroadcastTxCommit
	var err error

	err = retry.Do(
		func() error {
			var innerError error
			resp, innerError = c.sendExecuteMsg(ctx, c.jupiterAumContract, msg)
			return innerError
		},
		// Configuration options
		retry.Attempts(3),                   // Try 3 times
		retry.Delay(1*time.Second),          // Initial delay
		retry.DelayType(retry.BackOffDelay), // Use exponential backoff
		retry.OnRetry(func(n uint, err error) {
			c.logger.Info("Retry publish jupiter data attempt one more time")
		}),
	)
	if err != nil {
		return nil, err
	}

	c.logger.Info("submitted jupiter aum data",
		zap.Uint32("code", resp.TxResult.Code),
		zap.String("tx_hash", resp.Hash.String()),
		zap.Int64("height", resp.Height),
		zap.String("contract", "jupiter"),
		zap.Int64("tx_gas_used", resp.TxResult.GasUsed),
	)

	nextRound, err := GetNextRoundFromEvents(resp.TxResult.GetEvents())
	if err != nil {
		return nil, fmt.Errorf("jupiter: failed to extract next round from events: %w", err)
	}
	c.logger.Info("got next round for jupiter receiver from events")
	return nextRound, nil
}

// internal: query round info from smart contract
func (c *Client) queryRoundInfo(ctx context.Context, contract string) (*GetRoundResponse, error) {
	msg := map[string]any{"get_round_info": struct{}{}}
	resBz, err := c.client.QuerySmartContract(ctx, contract, msg)
	if err != nil {
		return nil, fmt.Errorf("failed to query smart contract: %w", err)
	}
	var response GetRoundResponse
	if err := json.Unmarshal(resBz, &response); err != nil {
		return nil, fmt.Errorf("failed to unmarshal get_round_info response: %w", err)
	}
	return &response, nil
}

// QueryERWindowInfo queries er_window_info from TWAER contract
func (c *Client) QueryERWindowInfo(ctx context.Context) (*GetERWindowInfo, error) {
	msg := map[string]any{"er_window_info": struct{}{}}
	resBz, err := c.client.QuerySmartContract(ctx, c.twaerContract, msg)
	if err != nil {
		// This means there is no window created yet, so we can safely exit here with zero
		// TODO: fix the TWAER, so it would not return an error
		if strings.Contains(err.Error(), "key: [74, 77, 61, 5F, 61, 67, 67, 72, 65, 67, 61, 74, 6F, 72] not found") {
			return &GetERWindowInfo{
				WindowStart: 0,
				WindowEnd:   0,
			}, nil
		}
		return nil, fmt.Errorf("failed to query smart contract: %w", err)
	}
	var response GetERWindowInfo
	if err := json.Unmarshal(resBz, &response); err != nil {
		return nil, fmt.Errorf("failed to unmarshal get_er_window_info response: %w", err)
	}
	return &response, nil
}

// QueryTwaerInfo queries er_window_info from TWAER contract
func (c *Client) QueryTwaerInfo(ctx context.Context) (*GetTwaerInfo, error) {
	msg := map[string]any{"get_twaer": struct{}{}}
	resBz, err := c.client.QuerySmartContract(ctx, c.twaerContract, msg)
	if err != nil {
		return nil, fmt.Errorf("failed to query smart contract: %w", err)
	}
	var response GetTwaerInfo
	if err := json.Unmarshal(resBz, &response); err != nil {
		return nil, fmt.Errorf("failed to unmarshal get_twaer response: %w", err)
	}
	return &response, nil
}

// internal: execute smart contract message
func (c *Client) sendExecuteMsg(ctx context.Context, contract string, msg any) (*cometcoretypes.ResultBroadcastTxCommit, error) {
	msgBz, err := json.Marshal(msg)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal execute message: %w", err)
	}
	execute := &types.MsgExecuteContract{
		Sender:   c.client.GetAddress(),
		Contract: contract,
		Msg:      msgBz,
		Funds:    sdk.NewCoins(),
	}
	resp, err := c.client.SignAndBroadcast(ctx, execute)
	if err != nil {
		return nil, fmt.Errorf("failed to sign and broadcast transaction: %w", err)
	}
	return resp, nil
}

// RecordER executes the record_er message on a TWAER contract.
func (c *Client) RecordER(ctx context.Context) error {
	c.logger.Info("executing record_er on TWAER contract", zap.String("contract", c.twaerContract))

	msg := map[string]any{
		"record_er": map[string]any{},
	}

	var resp *cometcoretypes.ResultBroadcastTxCommit
	var err error

	err = retry.Do(
		func() error {
			var innerError error
			resp, innerError = c.sendExecuteMsg(ctx, c.twaerContract, msg)
			return innerError
		},
		retry.Attempts(3),
		retry.Delay(1*time.Second),
		retry.DelayType(retry.BackOffDelay),
		retry.OnRetry(func(n uint, err error) {
			c.logger.Info("Retry record_er attempt", zap.Uint("attempt", n+1))
		}),
	)
	if err != nil {
		return fmt.Errorf("failed to execute record_er: %w", err)
	}

	c.logger.Info("executed record_er successfully",
		zap.Uint32("code", resp.TxResult.Code),
		zap.String("tx_hash", resp.Hash.String()),
		zap.Int64("height", resp.Height),
		zap.String("contract", c.twaerContract),
		zap.Int64("tx_gas_used", resp.TxResult.GasUsed),
	)

	return nil
}

// RecordER executes the record_er message on a TWAER contract.
func (c *Client) PublishTwaer(ctx context.Context) error {
	c.logger.Info("executing publish_twaer on TWAER contract", zap.String("contract", c.twaerContract))

	msg := map[string]any{
		"publish_twaer": map[string]any{},
	}

	var resp *cometcoretypes.ResultBroadcastTxCommit
	var err error

	err = retry.Do(
		func() error {
			var innerError error
			resp, innerError = c.sendExecuteMsg(ctx, c.twaerContract, msg)
			return innerError
		},
		retry.Attempts(3),
		retry.Delay(1*time.Second),
		retry.DelayType(retry.BackOffDelay),
		retry.OnRetry(func(n uint, err error) {
			c.logger.Info("Retry publish_twaer attempt", zap.Uint("attempt", n+1))
		}),
	)
	if err != nil {
		return fmt.Errorf("failed to execute publish_twaer: %w", err)
	}

	c.logger.Info("executed publish_twaer successfully",
		zap.Uint32("code", resp.TxResult.Code),
		zap.String("tx_hash", resp.Hash.String()),
		zap.Int64("height", resp.Height),
		zap.String("contract", c.twaerContract),
		zap.Int64("tx_gas_used", resp.TxResult.GasUsed),
	)

	return nil
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
