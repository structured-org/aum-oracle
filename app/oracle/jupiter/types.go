package jupiter

import solana "github.com/gagliardetto/solana-go"

// JupiterConfig is the configuration for the Jupiter oracle.
type JupiterConfig struct {
	// Custodies is the map of Jupiter custodies represented as token->programId.
	Custodies map[string]solana.PublicKey
	// Token is the Jupiter JLP token address.
	Token solana.PublicKey
	// Pool is the Jupiter JLP pool address.
	Pool solana.PublicKey
	// Strategy is the Jupiter JLP strategy address.
	Strategy solana.PublicKey
}
