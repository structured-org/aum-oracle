package jupiter

import (
	"context"

	solana "github.com/gagliardetto/solana-go"
	solanatoken "github.com/gagliardetto/solana-go/programs/token"
	solanarpc "github.com/gagliardetto/solana-go/rpc"
	jupiterclient "github.com/structured-org/aum-messenger/pkg/client/jupiter"
	neutronclient "github.com/structured-org/aum-messenger/pkg/client/neutron"
)

// JupiterClient is the definition of the expected Jupiter client.
type JupiterClient interface {
	GetJupiterCustodyInfo(ctx context.Context, custody solana.PublicKey) (*jupiterclient.JupiterPerpsCustodyAccount, error)
	GetJupiterPoolInfo(ctx context.Context, pool solana.PublicKey) (*jupiterclient.JupiterPoolAccount, error)
}

// SolanaClient is the definition of the expected Solana client.
type SolanaClient interface {
	GetTokenMint(ctx context.Context, tokenPubKey solana.PublicKey) (*solanatoken.Mint, error)
	GetTokenAccountBalance(ctx context.Context, token solana.PublicKey, account solana.PublicKey) (*solanarpc.UiTokenAmount, error)
	GetNativeBalance(ctx context.Context, account solana.PublicKey) (*solanarpc.UiTokenAmount, error)
}

// NeutronAumReceiverClient is the definition of the expected Neutron AUM receiver client.
type NeutronAumReceiverClient interface {
	GetJupiterAumReceiverNextRound(ctx context.Context) (*neutronclient.NextRound, error)
	SubmitJupiterAumData(ctx context.Context, data *neutronclient.JupiterAumData) (*neutronclient.NextRound, error)
}
