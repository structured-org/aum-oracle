package solana

import (
	"context"
	"fmt"
	"time"

	solana "github.com/gagliardetto/solana-go"
	solanarpc "github.com/gagliardetto/solana-go/rpc"
)

// Client is a client for Solana.
type Client struct {
	client *solanarpc.Client
}

// NewClient creates a new Solana client.
func NewClient(solanaRpc string) *Client {
	return &Client{
		client: solanarpc.New(solanaRpc),
	}
}

// TODO: use actual values when the client is implemented
var binanceRound = 0

// GetBinanceAumContractNextRound gets the next consensus round for the Binance AUM contract.
func (c *Client) GetBinanceAumContractNextRound(ctx context.Context) (*NextRound, error) {
	// TODO: use actual values when the client is implemented
	return &NextRound{
		Round:     uint64(binanceRound),
		Timestamp: uint64(time.Now().Add(time.Second).Unix()),
	}, nil
}

// SubmitBinanceAumData submits the Binance AUM data to the Binance AUM contract.
func (c *Client) SubmitBinanceAumData(ctx context.Context, data *BinanceAumData) (*NextRound, error) {
	// print for debug evaluation. TODO: use actual values when the client is implemented
	//spew.Dump("submitted Binance AUM data:", data)

	binanceRound++
	return &NextRound{
		Round:     uint64(binanceRound),
		Timestamp: uint64(time.Now().Add(time.Minute).Unix()),
	}, nil
}

// GetTokenSupply gets the token total supply.
func (c *Client) GetTokenSupply(ctx context.Context, token solana.PublicKey) (*solanarpc.UiTokenAmount, error) {
	supply, err := c.client.GetTokenSupply(ctx, token, solanarpc.CommitmentFinalized)
	if err != nil {
		return nil, err
	}

	return supply.Value, nil
}

// GetTokenAccountBalance gets the token account balance.
func (c *Client) GetTokenAccountBalance(ctx context.Context, token solana.PublicKey, account solana.PublicKey) (*solanarpc.UiTokenAmount, error) {
	tokenAccount, _, err := solana.FindAssociatedTokenAddress(account, token)
	if err != nil {
		return nil, fmt.Errorf("failed to find associated token address: %w", err)
	}

	balance, err := c.client.GetTokenAccountBalance(ctx, tokenAccount, solanarpc.CommitmentFinalized)
	if err != nil {
		return nil, err
	}

	return balance.Value, nil
}
