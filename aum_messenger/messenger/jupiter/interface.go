package jupiter

import (
	"context"

	solana "github.com/gagliardetto/solana-go"
	solanarpc "github.com/gagliardetto/solana-go/rpc"
	jupiterclient "github.com/structured-org/aum-messenger/client/jupiter"
	neutronclient "github.com/structured-org/aum-messenger/client/neutron"
)

// JupiterClient is the definition of the expected Jupiter client.
type JupiterClient interface {
	GetJupiterCustodyInfo(ctx context.Context, custody solana.PublicKey) (*jupiterclient.JupiterPerpsCustodyAccount, error)
	GetJupiterPoolInfo(ctx context.Context, pool solana.PublicKey) (*jupiterclient.JupiterPoolAccount, error)
}

// SolanaClient is the definition of the expected Solana client.
type SolanaClient interface {
	GetTokenSupply(ctx context.Context, token solana.PublicKey) (*solanarpc.UiTokenAmount, error)
	GetTokenAccountBalance(ctx context.Context, token solana.PublicKey, account solana.PublicKey) (*solanarpc.UiTokenAmount, error)
}

// NeutronAumReceiverClient is the definition of the expected Neutron AUM receiver client.
type NeutronAumReceiverClient interface {
	GetJupiterAumReceiverNextRound(ctx context.Context) (*neutronclient.NextRound, error)
	SubmitJupiterAumData(ctx context.Context, data *neutronclient.JupiterAumData) (*neutronclient.NextRound, error)
}
