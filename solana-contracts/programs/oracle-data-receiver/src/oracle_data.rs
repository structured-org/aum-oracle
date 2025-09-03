use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct OracleData {
    pub oracle: Pubkey,
    #[max_len(10_000)]
    pub data: Vec<u8>,
    #[max_len(32)]
    pub hash: Vec<u8>,
    pub timestamp: u64,
}
