package utils

import (
	"context"
	"encoding/json"
	"fmt"
	"sync"
	"time"

	wasmtypes "github.com/CosmWasm/wasmd/x/wasm/types"
	retry "github.com/avast/retry-go/v4"
	abcitypes "github.com/cometbft/cometbft/abci/types"
	"github.com/cometbft/cometbft/rpc/client"
	rpcclienthttp "github.com/cometbft/cometbft/rpc/client/http"
	cometcoretypes "github.com/cometbft/cometbft/rpc/core/types"
	jsonrpcclient "github.com/cometbft/cometbft/rpc/jsonrpc/client"
	"github.com/cosmos/cosmos-sdk/client/tx"
	sdkcodec "github.com/cosmos/cosmos-sdk/codec"
	codectypes "github.com/cosmos/cosmos-sdk/codec/types"
	cryptocodec "github.com/cosmos/cosmos-sdk/crypto/codec"
	"github.com/cosmos/cosmos-sdk/crypto/hd"
	"github.com/cosmos/cosmos-sdk/crypto/keyring"
	"github.com/cosmos/cosmos-sdk/types"
	"github.com/cosmos/cosmos-sdk/types/tx/signing"
	authtxtypes "github.com/cosmos/cosmos-sdk/x/auth/tx"
	authtypes "github.com/cosmos/cosmos-sdk/x/auth/types"
	"go.uber.org/zap"
)

const (
	// keyName is the name of the client's key pair in the keybase.
	keyName = "client"
	// bip39Passphrase is an additional phrase used in key pair creation along with mnemonic.
	bip39Passphrase = ""
	// accountQueryPath is the path for account queries.
	accountQueryPath = "/cosmos.auth.v1beta1.Query/Account"
)

var (
	// hdPath to be used in new account creation.
	hdPath = hd.CreateHDPath(types.GetConfig().GetCoinType(), 0, 0).String()
)

// CosmosClientConfig represents configuration for CosmosClient.
type CosmosClientConfig struct {
	Mnemonic           string        `yaml:"mnemonic"`
	GasPrices          string        `yaml:"gas_prices"`
	Gas                uint64        `yaml:"gas"`
	ChainID            string        `yaml:"chain_id"`
	Node               string        `yaml:"node"`
	NodeConnRetries    uint          `yaml:"node_conn_retries"`
	NodeConnRetryDelay time.Duration `yaml:"node_conn_retry_delay"`
}

// New creates a new instance of CosmosClient.
func New(cfg *CosmosClientConfig, logger *zap.Logger) (*CosmosClient, error) {
	rpcClient, err := createRpcClient(cfg.Node, cfg.NodeConnRetries, cfg.NodeConnRetryDelay, logger)
	if err != nil {
		return nil, err
	}
	cdc := sdkcodec.NewProtoCodec(codectypes.NewInterfaceRegistry())
	txConfig := authtxtypes.NewTxConfig(cdc, authtxtypes.DefaultSignModes)

	c := &CosmosClient{
		mu:           &sync.Mutex{},
		txEncoder:    txConfig.TxEncoder(),
		rpcClient:    rpcClient,
		subClientsMu: &sync.Mutex{},
		subClients:   make(map[string]client.Client),
		logger:       logger,
		cfg:          cfg,
	}
	keybase := keyring.NewInMemory(getCryptoCodec())
	c.baseTxFactory = tx.Factory{}.
		WithKeybase(keybase).
		WithSignMode(signing.SignMode_SIGN_MODE_DIRECT).
		WithTxConfig(txConfig).
		WithChainID(cfg.ChainID).
		WithGasPrices(cfg.GasPrices).
		WithGas(cfg.Gas)

	k, err := keybase.NewAccount(keyName, cfg.Mnemonic, bip39Passphrase, hdPath, hd.Secp256k1)
	if err != nil {
		return nil, fmt.Errorf("failed to restore account from given mnemonic: %w", err)
	}
	c.address, err = k.GetAddress()
	if err != nil {
		return nil, fmt.Errorf("failed to get address from restored mnemonic: %w", err)
	}
	c.logger.Info("key pair has been restored using the provided mnemonic", zap.String("pubkey", c.address.String()))
	return c, nil
}

// CosmosClient is an interface to tendermint API.
type CosmosClient struct {
	// address is the address of the client used in messages broadcasting.
	address types.AccAddress
	// mu controls sequence number for client txs
	mu            *sync.Mutex
	txEncoder     types.TxEncoder
	baseTxFactory tx.Factory
	rpcClient     client.Client
	subClientsMu  *sync.Mutex
	// subClients stores dedicated clients for each subscription. Dedicated clients are needed
	// because tendermint RPC client doesn't allow multiple subscriptions from a single instance.
	subClients map[string]client.Client
	logger     *zap.Logger
	cfg        *CosmosClientConfig
}

// GetAddress returns the client's network address.
func (c *CosmosClient) GetAddress() string {
	return c.address.String()
}

// SignAndBroadcast signs and broadcasts the msg. It locks the client mutex to keep the tx
// sequence value right.
func (c *CosmosClient) SignAndBroadcast(ctx context.Context, msg types.Msg) (*cometcoretypes.ResultBroadcastTxCommit, error) {
	c.mu.Lock()
	defer c.mu.Unlock()

	client, err := c.queryAccount(ctx, c.address.String())
	if err != nil {
		return nil, fmt.Errorf("failed to query client acc info: %w. probably the account has empty balances", err)
	}
	txFactory := c.baseTxFactory.
		WithAccountNumber(client.AccountNumber).
		WithSequence(client.Sequence)

	txBuilder, err := txFactory.BuildUnsignedTx(msg)
	if err != nil {
		return nil, fmt.Errorf("failed to build unsigned msg: %w", err)
	}

	if err = tx.Sign(ctx, txFactory, keyName, txBuilder, false); err != nil {
		return nil, fmt.Errorf("failed to sign tx: %w", err)
	}
	encodedMsg, err := c.txEncoder(txBuilder.GetTx())
	if err != nil {
		return nil, fmt.Errorf("failed to encode signed msg: %w", err)
	}

	res, err := c.rpcClient.BroadcastTxCommit(ctx, encodedMsg)
	if err != nil {
		return nil, fmt.Errorf("send msg broadcast error: %w", err)
	}
	if res.CheckTx.Code != abcitypes.CodeTypeOK {
		return nil, fmt.Errorf("check tx failed: code %d\nLog:%s", res.CheckTx.Code, res.CheckTx.Log)
	}

	if res.TxResult.Code != abcitypes.CodeTypeOK {
		return nil, fmt.Errorf("submit tx failed: code %d\nLog: %s", res.TxResult.Code, res.TxResult.Log)
	}

	return res, nil
}

// QuerySmartContract queries a CosmWasm smart contract with provided state query JSON or raw bytes.
func (c *CosmosClient) QuerySmartContract(ctx context.Context, contractAddr string, query interface{}) ([]byte, error) {
	queryBz, err := json.Marshal(query)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal query into json: %w", err)
	}

	req := wasmtypes.QuerySmartContractStateRequest{
		Address:   contractAddr,
		QueryData: queryBz,
	}

	bz, err := req.Marshal()
	if err != nil {
		return nil, fmt.Errorf("failed to marshal wasm query request: %w", err)
	}

	res, err := c.rpcClient.ABCIQueryWithOptions(ctx, "/cosmwasm.wasm.v1.Query/SmartContractState", bz, client.DefaultABCIQueryOptions)
	if err != nil {
		return nil, fmt.Errorf("abci query failed: %w", err)
	}
	if res.Response.Code != 0 {
		return nil, fmt.Errorf("query failed: code=%d log=%s", res.Response.Code, res.Response.Log)
	}

	var response wasmtypes.QuerySmartContractStateResponse
	if err := response.Unmarshal(res.Response.Value); err != nil {
		return nil, fmt.Errorf("error unmarshalling QuerySmartContractStateResponse for contractAddr=%s: %w", contractAddr, err)
	}

	return response.Data, nil
}

// Subscribe subscribes to events using the given query and returns a stream of events.
func (c *CosmosClient) Subscribe(ctx context.Context, subscriberName, query string) (<-chan cometcoretypes.ResultEvent, error) {
	c.subClientsMu.Lock()
	defer c.subClientsMu.Unlock()

	subClient, err := createRpcClient(c.cfg.Node, c.cfg.NodeConnRetries, c.cfg.NodeConnRetryDelay, c.logger)
	if err != nil {
		return nil, fmt.Errorf("failed to create a cosmos http client: %w", err)
	}
	c.logger.Debug("subscribing to cosmos events",
		zap.String("query", query),
		zap.String("subscriber", subscriberName),
	)
	events, err := subClient.Subscribe(ctx, subscriberName, query)
	if err != nil {
		return nil, err
	}
	c.subClients[subscriberName] = subClient
	return events, nil
}

// Unsubscribe unsubscribes from events stream on the given query.
func (c *CosmosClient) Unsubscribe(ctx context.Context, subscriberName, query string) {
	c.subClientsMu.Lock()
	defer c.subClientsMu.Unlock()

	subClient, ex := c.subClients[subscriberName]
	if !ex {
		c.logger.Error("failed to Unsubscribe from cosmos events",
			zap.Error(fmt.Errorf("no subscription client defined for the subscriber")),
			zap.String("query", query),
			zap.String("subscriber", subscriberName),
		)
		return
	}
	if err := subClient.Unsubscribe(ctx, subscriberName, query); err != nil {
		c.logger.Error("failed to Unsubscribe from cosmos events",
			zap.Error(err),
			zap.String("query", query),
			zap.String("subscriber", subscriberName),
		)
	} else {
		c.logger.Debug("unsubscribed from cosmos events",
			zap.String("query", query),
			zap.String("subscriber", subscriberName),
		)
		delete(c.subClients, subscriberName)
	}
}

// queryAccount retrieves the given account info using tendermint RPC.
func (c *CosmosClient) queryAccount(ctx context.Context, address string) (*authtypes.BaseAccount, error) {
	c.logger.Debug("querying cosmos client account info", zap.String("address", address))
	request := authtypes.QueryAccountRequest{Address: address}
	req, err := request.Marshal()
	if err != nil {
		return nil, fmt.Errorf("error marshalling query account request for account=%s: %w", address, err)
	}

	res, err := c.rpcClient.ABCIQueryWithOptions(ctx, accountQueryPath, req, client.DefaultABCIQueryOptions)
	if err != nil {
		return nil, fmt.Errorf("error making abci query for account=%s: %w", address, err)
	}
	if res.Response.Code != 0 {
		return nil, fmt.Errorf("error fetching account with address=%s log=%s", address, res.Response.Log)
	}

	var response authtypes.QueryAccountResponse
	if err := response.Unmarshal(res.Response.Value); err != nil {
		return nil, fmt.Errorf("error unmarshalling QueryAccountResponse for account=%s: %w", address, err)
	}
	var account authtypes.BaseAccount
	if err := account.Unmarshal(response.Account.Value); err != nil {
		return nil, fmt.Errorf("error unmarshalling BaseAccount for account=%s: %w", address, err)
	}
	c.logger.Debug("got cosmos client account info",
		zap.String("address", address),
		zap.Uint64("account_number", account.AccountNumber),
		zap.Uint64("sequence_number", account.Sequence),
	)
	return &account, nil
}

// createRpcClient connects to a node by the given addr and returns the client.
func createRpcClient(addr string, retries uint, delay time.Duration, logger *zap.Logger) (client.Client, error) {
	httpClient, err := jsonrpcclient.DefaultHTTPClient(addr)
	if err != nil {
		return nil, fmt.Errorf("could not create http client with address=%s: %w", addr, err)
	}
	httpClient.Timeout = 10 * time.Second

	rpcClient, err := rpcclienthttp.NewWithClient(addr, "/websocket", httpClient)
	if err != nil {
		return nil, fmt.Errorf("could not initialize rpc client from http client with address=%s: %w", addr, err)
	}
	attempts := retries + 1
	var attempt uint
	logger.Info("establishing connection to neutron node...", zap.String("node_address", addr))
	if err := retry.Do(func() error {
		attempt++
		if err = rpcClient.Start(); err != nil {
			logger.Debug("connection to node error",
				zap.Error(err),
				zap.String("attempt", fmt.Sprintf("%d/%d", attempt, attempts)),
			)
			return err
		}
		return nil
	}, retry.Attempts(attempts), retry.Delay(delay), retry.LastErrorOnly(true)); err != nil {
		return nil, fmt.Errorf("failed to establish connection to node %s in %d attempts with delay of %.0f seconds: %w", addr, attempts, delay.Seconds(), err)
	}
	logger.Debug("connection to node successful", zap.String("node_address", addr))
	return rpcClient, nil
}

func getCryptoCodec() *sdkcodec.ProtoCodec {
	registry := codectypes.NewInterfaceRegistry()
	cryptocodec.RegisterInterfaces(registry)
	return sdkcodec.NewProtoCodec(registry)
}
