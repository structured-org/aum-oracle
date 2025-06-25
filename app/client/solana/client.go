package solana

import (
	"context"

	solana "github.com/gagliardetto/solana-go"
	solanarpc "github.com/gagliardetto/solana-go/rpc"
)

type Client struct {
	client *solanarpc.Client
}

func NewClient(rpcEndpoint string) *Client {
	return &Client{
		client: solanarpc.New(rpcEndpoint),
	}
}

func (c *Client) GetJupiterCustodyInfo(ctx context.Context, custody solana.PublicKey) (*JupiterPerpsCustodyAccount, error) {
	var resp JupiterPerpsCustodyAccount
	if err := c.client.GetAccountDataBorshInto(ctx, custody, &resp); err != nil {
		return nil, err
	}

	return &resp, nil
}

func (c *Client) GetJupiterPoolInfo(ctx context.Context, pool solana.PublicKey) (*JupiterPoolAccount, error) {
	var resp JupiterPoolAccount
	if err := c.client.GetAccountDataBorshInto(ctx, pool, &resp); err != nil {
		return nil, err
	}

	return &resp, nil
}

func (c *Client) GetTokenSupply(ctx context.Context, token solana.PublicKey) (*solanarpc.UiTokenAmount, error) {
	supply, err := c.client.GetTokenSupply(ctx, token, "")
	if err != nil {
		return nil, err
	}

	return supply.Value, nil
}

func (c *Client) GetTokenAccountBalance(ctx context.Context, token solana.PublicKey, account solana.PublicKey) (*solanarpc.UiTokenAmount, error) {
	tokenAccount, _, err := solana.FindAssociatedTokenAddress(account, token)
	if err != nil {
		return nil, err
	}

	balance, err := c.client.GetTokenAccountBalance(ctx, tokenAccount, "")
	if err != nil {
		return nil, err
	}

	return balance.Value, nil
}
