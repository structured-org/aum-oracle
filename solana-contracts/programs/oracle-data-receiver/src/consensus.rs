use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Consensus {
    pub timestamp: u64,
    #[max_len(10_000)]
    pub data: Vec<u8>,
}
