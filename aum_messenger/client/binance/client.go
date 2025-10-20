package binance

import (
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strconv"
	"time"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"

	binance "github.com/adshao/go-binance/v2"
	binanceportfolio "github.com/adshao/go-binance/v2/portfolio"
)

var BinanceRateLimitRequestWeight = promauto.NewGaugeVec(prometheus.GaugeOpts{
	Namespace:   "",
	Subsystem:   "",
	Name:        "binance_rate_limit_request_weight",
	Help:        "Binance rate limit request weight metric",
	ConstLabels: nil,
}, []string{"interval", "interval_num"})

var BinanceRateLimitUsedWeightInMinute = promauto.NewGaugeVec(prometheus.GaugeOpts{
	Namespace:   "",
	Subsystem:   "",
	Name:        "binance_rate_limit_used_weight_in_minute",
	Help:        "Binance rate limit used weight in 1 minute window metric",
	ConstLabels: nil,
}, []string{"domain"})

// NewClient creates a new Binance client.
func NewClient(spotBaseURL, pmBaseURL, apiKey, apiSecret string) *Client {
	c := &Client{
		spotBaseURL: spotBaseURL,
		pmBaseURL:   pmBaseURL,
		apiKey:      apiKey,
		apiSecret:   apiSecret,
		httpClient:  http.DefaultClient,
	}

	c.setRateLimits()

	return c
}

// Client is the Binance client.
type Client struct {
	spotBaseURL string
	pmBaseURL   string
	apiKey      string
	apiSecret   string
	httpClient  *http.Client
}

// GetUmPositions gets user's USD-margined portfolio perpetual futures positions.
func (c *Client) GetUmPositions(ctx context.Context) ([]*binanceportfolio.UMPosition, error) {
	resp, rate, err := c.doRequest(ctx, c.pmBaseURL, "/papi/v1/um/positionRisk", url.Values{}, true)
	if err != nil {
		return nil, fmt.Errorf("Binance API query failed: %w", err)
	}

	BinanceRateLimitUsedWeightInMinute.WithLabelValues("papi").Set(float64(rate.UsedWeight1Min))

	var res []*binanceportfolio.UMPosition
	if err := json.Unmarshal(resp, &res); err != nil {
		return nil, fmt.Errorf("failed to unmarshal pm um positions: %w", err)
	}

	return res, nil
}

// GetPMAccountInfo gets user's Portfolio Margin account information.
func (c *Client) GetPMAccountInfo(ctx context.Context) (*binanceportfolio.Account, error) {
	resp, rate, err := c.doRequest(ctx, c.pmBaseURL, "/papi/v1/account", url.Values{}, true)
	if err != nil {
		return nil, fmt.Errorf("Binance API query failed: %w", err)
	}

	BinanceRateLimitUsedWeightInMinute.WithLabelValues("papi").Set(float64(rate.UsedWeight1Min))

	var res binanceportfolio.Account
	if err := json.Unmarshal(resp, &res); err != nil {
		return nil, fmt.Errorf("failed to unmarshal pm account info: %w", err)
	}

	return &res, nil
}

// GetPMAccountBalance gets user's Portfolio Margin account balance.
func (c *Client) GetPMAccountBalance(ctx context.Context) ([]*binanceportfolio.Balance, error) {
	resp, rate, err := c.doRequest(ctx, c.pmBaseURL, "/papi/v1/balance", url.Values{}, true)
	if err != nil {
		return nil, fmt.Errorf("Binance API query failed: %w", err)
	}

	BinanceRateLimitUsedWeightInMinute.WithLabelValues("papi").Set(float64(rate.UsedWeight1Min))

	var res []*binanceportfolio.Balance
	if err := json.Unmarshal(resp, &res); err != nil {
		return nil, fmt.Errorf("failed to unmarshal pm account balances: %w", err)
	}

	return res, nil
}

// GetSpotAccountInfo gets user's spot account information.
func (c *Client) GetSpotAccountInfo(ctx context.Context) (*binance.Account, error) {
	resp, rate, err := c.doRequest(ctx, c.spotBaseURL, "/api/v3/account", url.Values{}, true)
	if err != nil {
		return nil, fmt.Errorf("Binance API query failed: %w", err)
	}

	BinanceRateLimitUsedWeightInMinute.WithLabelValues("api").Set(float64(rate.UsedWeight1Min))

	var res binance.Account
	if err := json.Unmarshal(resp, &res); err != nil {
		return nil, fmt.Errorf("failed to unmarshal spot account info: %w", err)
	}

	return &res, nil
}

func (c *Client) setRateLimits() error {
	resp, rate, err := c.doRequest(context.Background(), c.spotBaseURL, "/api/v3/exchangeInfo", url.Values{}, true)
	if err != nil {
		return fmt.Errorf("Binance API query failed: %w", err)
	}

	BinanceRateLimitUsedWeightInMinute.WithLabelValues("api").Set(float64(rate.UsedWeight1Min))

	var res binance.ExchangeInfo
	if err := json.Unmarshal(resp, &res); err != nil {
		return fmt.Errorf("failed to unmarshal exchange info: %w", err)
	}

	for _, rateLimit := range res.RateLimits {
		switch rateLimit.RateLimitType {
		case string(binance.RateLimitTypeRequestWeight):
			BinanceRateLimitRequestWeight.WithLabelValues(rateLimit.Interval, strconv.FormatInt(rateLimit.IntervalNum, 10)).
				Set(float64(rateLimit.Limit))
		}
	}

	return nil
}

func (c *Client) doRequest(
	ctx context.Context,
	baseURL, endpoint string,
	params url.Values,
	signed bool,
) ([]byte, *RateLimitInfo, error) {
	reqURL := fmt.Sprintf("%s%s", baseURL, endpoint)

	if signed {
		timestamp := strconv.FormatInt(time.Now().UnixMilli(), 10)
		params.Set("timestamp", timestamp)
		queryString := params.Encode()
		signature := c.sign(queryString)
		reqURL = fmt.Sprintf("%s?%s&signature=%s", reqURL, queryString, signature)
	} else if len(params) > 0 {
		reqURL = fmt.Sprintf("%s?%s", reqURL, params.Encode())
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, reqURL, nil)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to create request: %w", err)
	}
	req.Header.Set("X-MBX-APIKEY", c.apiKey)

	resp, err := c.httpClient.Do(req)
	if resp != nil {
		defer resp.Body.Close()
	}
	if err != nil {
		return nil, nil, fmt.Errorf("request failed: %w", err)
	}

	rate := parseRateLimitHeaders(resp.Header)

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to read body: %w", err)
	}
	if resp.StatusCode != http.StatusOK {
		return nil, nil, fmt.Errorf("unexpected status %s: %s", resp.Status, string(body))
	}

	return body, rate, nil
}

func (c *Client) sign(query string) string {
	mac := hmac.New(sha256.New, []byte(c.apiSecret))
	mac.Write([]byte(query))
	return hex.EncodeToString(mac.Sum(nil))
}

func parseRateLimitHeaders(h http.Header) *RateLimitInfo {
	atoi := func(s string) int {
		if s == "" {
			return 0
		}
		n, _ := strconv.Atoi(s)
		return n
	}

	return &RateLimitInfo{
		UsedWeight1Min: atoi(h.Get("X-MBX-USED-WEIGHT-1M")),
	}
}
