package solana

import (
	"bytes"
	"context"
	"crypto/sha256"

	"fmt"
	"time"

	bin "github.com/gagliardetto/binary"
	solana "github.com/gagliardetto/solana-go"
	"github.com/gagliardetto/solana-go/rpc"
	solanarpc "github.com/gagliardetto/solana-go/rpc"
	msgrclient "github.com/structured-org/aum-messenger/client"
	"go.uber.org/zap"
)

// Client is a client for Solana.
type Client struct {
	client  *solanarpc.Client
	keypair solana.PrivateKey
	logger  *zap.Logger
}

// NewClient creates a new Solana client.
func NewClient(solanaRpc string, keypairPath string, logger *zap.Logger) *Client {

	return &Client{
		client:  solanarpc.New(solanaRpc),
		keypair: mustLoadPrivateKeyFromFile(keypairPath),
		logger:  logger.With(zap.String("client", "solana")),
	}
}

func mustLoadPrivateKeyFromFile(filepath string) solana.PrivateKey {

	privateKey, err := solana.PrivateKeyFromSolanaKeygenFile(filepath)
	if err != nil {
		panic(fmt.Errorf("unable to load private key: %w", err))
	}

	return privateKey
}

// TODO: use actual values when the client is implemented
var binanceRound = 0

// GetBinanceAumReceiverNextRound gets the next consensus round for the Binance AUM receiver contract.
func (c *Client) GetBinanceAumReceiverNextRound(ctx context.Context) (*msgrclient.NextRound, error) {
	// TODO: use actual values when the client is implemented
	return &msgrclient.NextRound{
		Round:     uint64(binanceRound),
		Timestamp: uint64(time.Now().Add(time.Second).Unix()),
	}, nil
}

// SubmitBinanceAumData submits the Binance AUM data to the Binance AUM receiver contract.
func (c *Client) SubmitBinanceAumData(ctx context.Context, data *BinanceAumData) (*msgrclient.NextRound, error) {
	binanceRound++
	return &msgrclient.NextRound{
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

// GetArbitraryNeutronContractDataNextRound gets the next consensus round for the Solana data receiver contract.
func (c *Client) GetArbitraryNeutronContractDataNextRound(ctx context.Context, programId string, instanceKey solana.PublicKey) (*msgrclient.NextRound, error) {
	roundTime, err := c.getArbitraryNeutronContractDataRoundTime(ctx, programId, instanceKey)
	if err != nil {
		return nil, err
	}

	if *roundTime == uint16(0) {
		return nil, fmt.Errorf("round time is zero")
	}

	currentTime := time.Now().Unix()

	nextRoundTime := currentTime - (currentTime % int64(*roundTime)) + int64(*roundTime)

	return &msgrclient.NextRound{
		Round:     uint64(0),
		Timestamp: uint64(nextRoundTime),
	}, nil
}

type SimpleHandlerArgs struct {
	Data []byte `bin:"data"`
}

func anchorDiscriminator(name string) []byte {
	hash := sha256.Sum256([]byte("global:" + name))
	return hash[:8]
}

// SubmitArbitraryNeutronContractData submits the arbitrary contract data to the Solana data receiver contract.
func (c *Client) SubmitArbitraryNeutronContractData(ctx context.Context, programID solana.PublicKey, instanceKey solana.PublicKey, oraclesList *[]string, data *[]byte) (*msgrclient.NextRound, error) {

	configPdaSeeds := [][]byte{
		[]byte("config"),
		instanceKey.Bytes(),
	}

	configPDA, _, err := solana.FindProgramAddress(configPdaSeeds, programID)
	if err != nil {
		return nil, fmt.Errorf("failed to find config PDA address: %w", err)
	}

	oracleDataPdaSeeds := [][]byte{
		[]byte("oracle_data"),
		instanceKey.Bytes(),
		c.keypair.PublicKey().Bytes(),
	}

	oracleDataPDA, _, err := solana.FindProgramAddress(oracleDataPdaSeeds, programID)
	if err != nil {
		return nil, fmt.Errorf("failed to find oracle data PDA address: %w", err)
	}

	consensusPdaSeeds := [][]byte{
		[]byte("consensus"),
		instanceKey.Bytes(),
	}

	consensusPDA, _, err := solana.FindProgramAddress(consensusPdaSeeds, programID)
	if err != nil {
		return nil, fmt.Errorf("failed to find consensus PDA address: %w", err)
	}

	accounts := []*solana.AccountMeta{
		{PublicKey: configPDA, IsSigner: false, IsWritable: false},
		{PublicKey: oracleDataPDA, IsSigner: false, IsWritable: true},
		{PublicKey: consensusPDA, IsSigner: false, IsWritable: true},
		{PublicKey: c.keypair.PublicKey(), IsSigner: true, IsWritable: true},
		{PublicKey: solana.SystemProgramID, IsSigner: false, IsWritable: false},
	}

	oraclesDataAccounts := []*solana.AccountMeta{}

	for i := range *oraclesList {
		oracleAccountStr := (*oraclesList)[i]

		if oracleAccountStr == c.keypair.PublicKey().String() {
			continue
		}

		oracleAccount := solana.MustPublicKeyFromBase58(oracleAccountStr)
		dataPdaSeeds := [][]byte{
			[]byte("oracle_data"),
			instanceKey.Bytes(),
			oracleAccount.Bytes(),
		}

		dataPDA, _, err := solana.FindProgramAddress(dataPdaSeeds, programID)
		if err != nil {
			return nil, fmt.Errorf("failed to find oracle data PDA address: %w", err)
		}
		oraclesDataAccounts = append(
			oraclesDataAccounts,
			&solana.AccountMeta{PublicKey: dataPDA, IsSigner: false, IsWritable: false},
		)
	}

	commonAccounts := append(accounts, oraclesDataAccounts...)

	dataBuffer := new(bytes.Buffer)

	discriminator := anchorDiscriminator("publish_data")
	dataBuffer.Write(discriminator)

	args := &SimpleHandlerArgs{
		Data: *data,
	}

	borshEncoder := bin.NewBorshEncoder(dataBuffer)

	err = borshEncoder.Encode(args)
	if err != nil {
		return nil, fmt.Errorf("failed to encode args: %w", err)
	}

	latest, err := c.client.GetLatestBlockhash(ctx, rpc.CommitmentFinalized)
	if err != nil {
		return nil, fmt.Errorf("Unable to get latest blockhash: %w", err)
	}

	tx, err := solana.NewTransaction(
		[]solana.Instruction{
			solana.NewInstruction(
				programID,
				commonAccounts,
				dataBuffer.Bytes(),
			),
		},
		latest.Value.Blockhash,
		solana.TransactionPayer(c.keypair.PublicKey()),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to create transaction: %w", err)
	}

	_, err = tx.Sign(
		func(key solana.PublicKey) *solana.PrivateKey {
			if c.keypair.PublicKey().Equals(key) {
				return &c.keypair
			}
			return nil
		},
	)
	if err != nil {
		return nil, fmt.Errorf("unable to sign transaction: %w", err)
	}

	_, err = c.client.SimulateTransaction(
		ctx,
		tx,
	)

	if err != nil {
		c.logger.Info("Simulate err", zap.Any("err", err))
	}

	_, err = c.client.SendTransactionWithOpts(
		ctx,
		tx,
		rpc.TransactionOpts{
			SkipPreflight:       true,
			PreflightCommitment: rpc.CommitmentFinalized,
		},
	)
	if err != nil {
		return nil, fmt.Errorf("Unable to send transaction: %w", err)
	}

	nextRound, err := c.GetArbitraryNeutronContractDataNextRound(ctx, programID.String(), instanceKey)
	if err != nil {
		return nil, err
	}

	return nextRound, nil
}

// getArbitraryNeutronContractDataRoundTime returns the round time for the Arbitrary Neutron contract.
func (c *Client) getArbitraryNeutronContractDataRoundTime(ctx context.Context, in string, instanceKey solana.PublicKey) (*uint16, error) {
	programID := solana.MustPublicKeyFromBase58(in)

	seeds := [][]byte{
		[]byte("config"),
		instanceKey.Bytes(),
	}

	configPDA, _, err := solana.FindProgramAddress(seeds, programID)
	if err != nil {
		return nil, fmt.Errorf("failed to find program address: %w", err)
	}

	c.logger.Info("getArbitraryNeutronContractDataRoundTime: 1")

	var cfg Config
	if err := c.client.GetAccountDataBorshInto(ctx, configPDA, &cfg); err != nil {
		return nil, err
	}

	c.logger.Info("getArbitraryNeutronContractDataRoundTime", zap.Any("Config", cfg))

	return &cfg.RoundTime, nil

}
