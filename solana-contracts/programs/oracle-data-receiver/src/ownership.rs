use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Ownership {
    pub owner: Pubkey, // Owner who can call withdraw_asset and update_config methods
    pub pending_owner: Option<Pubkey>, // Pending owner. This field isn't supposed to be overwritten by update_config or initialize
}
