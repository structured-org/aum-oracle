use anchor_lang::prelude::*;

#[error_code]
pub enum CustomError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Invalid PDA for oracle_data")]
    InvalidPda,
    #[msg("No pending ownership")]
    NoPendingOwnership,
    #[msg("Invalid pubkey provided")]
    InvalidPubkey,
}
