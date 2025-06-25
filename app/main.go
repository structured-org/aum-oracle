package main

import (
	"context"
	"log"
	"time"

	"github.com/gagliardetto/solana-go"
	binanceoracle "github.com/structured-org/aum-oracle/binance"
	binanceclient "github.com/structured-org/aum-oracle/client/binance"
	neutronclient "github.com/structured-org/aum-oracle/client/neutron"
	solanaclient "github.com/structured-org/aum-oracle/client/solana"
	solanaoracle "github.com/structured-org/aum-oracle/solana"
	"go.uber.org/zap"
)

func main() {
	conf := ReadConfig()
	log.Printf("app config: %+v", conf)

	jupiterCustodies := make(map[string]solana.PublicKey)
	for token, programId := range conf.JupiterCustodies {
		jupiterCustodies[token] = solana.MustPublicKeyFromBase58(programId)
	}
	jupiterConfig := solanaoracle.JupiterConfig{
		Custodies: jupiterCustodies,
		Token:     solana.MustPublicKeyFromBase58(conf.JupiterJlpToken),
		Pool:      solana.MustPublicKeyFromBase58(conf.JupiterPool),
		Strategy:  solana.MustPublicKeyFromBase58(conf.JupiterStrategyAddress),
	}

	solanaClient := solanaclient.NewClient(conf.SolanaRpcEndpoint)
	neutronClient, err := neutronclient.NewClient()
	if err != nil {
		log.Fatalf("failed to create neutron client: %v", err)
	}
	binanceClient := binanceclient.NewClient(conf.BinanceApiKey, conf.BinanceApiSecret)

	binanceOracle := binanceoracle.NewOracle(binanceClient, neutronClient, zap.NewExample())
	solanaOracle := solanaoracle.NewOracle(solanaClient, neutronClient, jupiterConfig, zap.NewExample())

	ctx := context.Background()
	go binanceOracle.Run(ctx)
	// shift oracles in time to avoid simultaneous prints to stdout at debug submission. TODO: remove
	time.Sleep(10 * time.Second)
	go solanaOracle.Run(ctx)

	// run for 1 hour. TODO: run until interrupted
	time.Sleep(time.Minute * 60)
}
