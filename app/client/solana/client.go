package solana

import (
	"context"
	"fmt"

	solana "github.com/gagliardetto/solana-go"
	solanarpc "github.com/gagliardetto/solana-go/rpc"
)

// Client is a client for Solana and Jupiter.
type Client struct {
	client *solanarpc.Client
}

// NewClient creates a new Solana client.
func NewClient(rpcEndpoint string) *Client {
	return &Client{
		client: solanarpc.New(rpcEndpoint),
	}
}

// GetJupiterCustodyInfo gets the Jupiter custody info.
func (c *Client) GetJupiterCustodyInfo(ctx context.Context, custody solana.PublicKey) (*JupiterPerpsCustodyAccount, error) {
	var resp JupiterPerpsCustodyAccount
	if err := c.client.GetAccountDataBorshInto(ctx, custody, &resp); err != nil {
		return nil, err
	}

	return &resp, nil
}

// GetJupiterPoolInfo gets the Jupiter pool info.
func (c *Client) GetJupiterPoolInfo(ctx context.Context, pool solana.PublicKey) (*JupiterPoolAccount, error) {
	var resp JupiterPoolAccount
	if err := c.client.GetAccountDataBorshInto(ctx, pool, &resp); err != nil {
		return nil, err
	}

	return &resp, nil
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
