package binance

import (
	"context"
	"fmt"

	binance "github.com/adshao/go-binance/v2"
	binanceportfolio "github.com/adshao/go-binance/v2/portfolio"
)

func NewClient(apiKey, apiSecret string) *Client {
	return &Client{
		binance:          binance.NewClient(apiKey, apiSecret),
		binanceportfolio: binanceportfolio.NewClient(apiKey, apiSecret),
	}
}

type Client struct {
	binance          *binance.Client
	binanceportfolio *binanceportfolio.Client
}

// GetCmPositions gets user's coin-margined portfolio perpetual futures positions.
func (c *Client) GetCmPositions(ctx context.Context) ([]*binanceportfolio.CMPosition, error) {
	resp, err := c.binanceportfolio.NewGetCMPositionRiskService().Do(ctx)
	if err != nil {
		return nil, fmt.Errorf("Binance API query failed: %w", err)
	}
	return resp, nil
}

// GetUmPositions gets user's USD-margined portfolio perpetual futures positions.
func (c *Client) GetUmPositions(ctx context.Context) ([]*binanceportfolio.UMPosition, error) {
	resp, err := c.binanceportfolio.NewGetUMPositionRiskService().Do(ctx)
	if err != nil {
		return nil, fmt.Errorf("Binance API query failed: %w", err)
	}
	return resp, nil
}

// GetPMAccountInfo gets user's Portfolio Margin account information.
func (c *Client) GetPMAccountInfo(ctx context.Context) (*binanceportfolio.Account, error) {
	resp, err := c.binanceportfolio.NewGetAccountService().Do(ctx)
	if err != nil {
		return nil, fmt.Errorf("Binance API query failed: %w", err)
	}
	return resp, nil
}

// GetPMAccountBalance gets user's Portfolio Margin account balance.
func (c *Client) GetPMAccountBalance(ctx context.Context) ([]*binanceportfolio.Balance, error) {
	resp, err := c.binanceportfolio.NewGetBalanceService().Do(ctx)
	if err != nil {
		return nil, fmt.Errorf("Binance API query failed: %w", err)
	}
	return resp, nil
}

// GetSpotAccountInfo gets user's spot account information.
func (c *Client) GetSpotAccountInfo(ctx context.Context) (*binance.Account, error) {
	resp, err := c.binance.NewGetAccountService().Do(ctx)
	if err != nil {
		return nil, fmt.Errorf("Binance API query failed: %w", err)
	}
	return resp, nil
}
