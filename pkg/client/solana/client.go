package solana

import (
	"context"
	"fmt"
	"time"

	solana "github.com/gagliardetto/solana-go"
	solanatoken "github.com/gagliardetto/solana-go/programs/token"
	solanarpc "github.com/gagliardetto/solana-go/rpc"
)

const (
	// SolanaNativeTokenDecimals is the number of decimals of SOL. 1 SOL is represented as
	// 1,000,000,000 lamports which are its smallest units.
	SolanaNativeTokenDecimals = 9
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

// GetBinanceAumReceiverNextRound gets the next consensus round for the Binance AUM receiver contract.
func (c *Client) GetBinanceAumReceiverNextRound(ctx context.Context) (*NextRound, error) {
	// TODO: use actual values when the client is implemented
	return &NextRound{
		Round:     uint64(binanceRound),
		Timestamp: uint64(time.Now().Add(time.Second).Unix()),
	}, nil
}

// SubmitBinanceAumData submits the Binance AUM data to the Binance AUM receiver contract.
func (c *Client) SubmitBinanceAumData(ctx context.Context, data *BinanceAumData) (*NextRound, error) {
	binanceRound++
	return &NextRound{
		Round:     uint64(binanceRound),
		Timestamp: uint64(time.Now().Add(time.Minute).Unix()),
	}, nil
}

// GetTokenMint gets the token mint account info.
func (c *Client) GetTokenMint(ctx context.Context, tokenPubKey solana.PublicKey) (*solanatoken.Mint, error) {
	mint := &solanatoken.Mint{}
	if err := c.client.GetAccountDataBorshInto(ctx, tokenPubKey, mint); err != nil {
		return nil, err
	}
	return mint, nil
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

// GetNativeBalance gets the SOL balance of an account.
func (c *Client) GetNativeBalance(ctx context.Context, account solana.PublicKey) (*solanarpc.UiTokenAmount, error) {
	balance, err := c.client.GetBalance(ctx, account, solanarpc.CommitmentFinalized)
	if err != nil {
		return nil, err
	}

	return &solanarpc.UiTokenAmount{
		Amount:   fmt.Sprintf("%d", balance.Value),
		Decimals: SolanaNativeTokenDecimals,
	}, nil
}
