package arbitrary_data

import (
	"context"

	solana "github.com/gagliardetto/solana-go"
	msgrclient "github.com/structured-org/aum-messenger/client"
)

// SolanaClient is the definition of the expected Solana client.
type SolanaClient interface {
	GetArbitraryNeutronContractDataNextRound(ctx context.Context, in string, instanceKey solana.PublicKey) (*msgrclient.NextRound, error)
	SubmitArbitraryNeutronContractData(ctx context.Context, programID solana.PublicKey, instanceKey solana.PublicKey, oraclesList *[]string, data *[]byte) (*msgrclient.NextRound, error)
}

// NeutronAumReceiverClient is the definition of the expected Neutron AUM receiver client.
type NeutronClient interface {
	QueryArbitraryNeutronContract(ctx context.Context, contract string, queryStr string) (*[]byte, error)
	GetOraclesList(ctx context.Context, contractAddress string) (*[]string, error)
}
