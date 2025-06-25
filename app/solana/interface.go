package solana

import (
	"context"

	solana "github.com/gagliardetto/solana-go"
	solanarpc "github.com/gagliardetto/solana-go/rpc"
	neutronclient "github.com/structured-org/aum-oracle/client/neutron"
	solanaclient "github.com/structured-org/aum-oracle/client/solana"
)

// SolanaClient is the definition of the expected Solana client interface.
type SolanaClient interface {
	GetJupiterCustodyInfo(ctx context.Context, custody solana.PublicKey) (*solanaclient.JupiterPerpsCustodyAccount, error)
	GetJupiterPoolInfo(ctx context.Context, pool solana.PublicKey) (*solanaclient.JupiterPoolAccount, error)
	GetTokenSupply(ctx context.Context, token solana.PublicKey) (*solanarpc.UiTokenAmount, error)
	GetTokenAccountBalance(ctx context.Context, token solana.PublicKey, account solana.PublicKey) (*solanarpc.UiTokenAmount, error)
}

// NeutronClient is the definition of the expected Neutron client interface.
type NeutronClient interface {
	GetSolanaAumContractNextRound(ctx context.Context) (*neutronclient.NextRound, error)
	SubmitSolanaAumData(ctx context.Context, data *neutronclient.SolanaData) (*neutronclient.NextRound, error)
}
