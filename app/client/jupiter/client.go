package jupiter

import (
	"context"
	"encoding/json"
	"os"

	solana "github.com/gagliardetto/solana-go"
	solanarpc "github.com/gagliardetto/solana-go/rpc"
)

// Client is a client for Jupiter.
type Client struct {
	client *solanarpc.Client
}

func writeToFile(path string, data interface{}) {
	bz, err := json.MarshalIndent(data, "", "  ")
	if err != nil {
		panic(err.Error())
	}

	if err := os.WriteFile(path, bz, 0600); err != nil {
		panic(err.Error())
	}
}

// NewClient creates a new Jupiter client.
func NewClient(solanaRpc string) *Client {
	return &Client{
		client: solanarpc.New(solanaRpc),
	}
}

// GetJupiterCustodyInfo gets the Jupiter custody info.
func (c *Client) GetJupiterCustodyInfo(ctx context.Context, custody solana.PublicKey) (*JupiterPerpsCustodyAccount, error) {
	var resp JupiterPerpsCustodyAccount
	if err := c.client.GetAccountDataBorshInto(ctx, custody, &resp); err != nil {
		return nil, err
	}

	writeToFile("custody_"+custody.String()+".json", &resp)

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
