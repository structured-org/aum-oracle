package solana

import (
	"context"

	solana "github.com/gagliardetto/solana-go"
	solanarpc "github.com/gagliardetto/solana-go/rpc"
	jupiterclient "github.com/structured-org/aum-oracle/client/jupiter"
	neutronclient "github.com/structured-org/aum-oracle/client/neutron"
)

// JupiterClient is the definition of the expected Jupiter client interface.
type JupiterClient interface {
	GetJupiterCustodyInfo(ctx context.Context, custody solana.PublicKey) (*jupiterclient.JupiterPerpsCustodyAccount, error)
	GetJupiterPoolInfo(ctx context.Context, pool solana.PublicKey) (*jupiterclient.JupiterPoolAccount, error)
}

// SolanaClient is the definition of the expected Solana client interface.
type SolanaClient interface {
	GetTokenSupply(ctx context.Context, token solana.PublicKey) (*solanarpc.UiTokenAmount, error)
	GetTokenAccountBalance(ctx context.Context, token solana.PublicKey, account solana.PublicKey) (*solanarpc.UiTokenAmount, error)
}

// NeutronClient is the definition of the expected Neutron client interface.
type NeutronClient interface {
	GetSolanaAumContractNextRound(ctx context.Context) (*neutronclient.NextRound, error)
	SubmitSolanaAumData(ctx context.Context, data *neutronclient.SolanaData) (*neutronclient.NextRound, error)
}
