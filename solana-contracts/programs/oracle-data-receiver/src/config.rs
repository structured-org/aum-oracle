use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub round_time: u16,
    #[max_len(16)]
    pub oracles: Vec<Pubkey>,
    pub instance_key: Pubkey,
}
