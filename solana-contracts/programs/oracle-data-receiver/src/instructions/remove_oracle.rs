use anchor_lang::prelude::*;

use crate::{constants, Config, CustomError, Ownership};

pub fn remove_oracle_handler(ctx: Context<ExecuteRemoveOracle>, oracle: Pubkey) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.oracles.retain(|pk| pk != &oracle);
    Ok(())
}

#[derive(Accounts)]
pub struct ExecuteRemoveOracle<'info> {
    #[account(
        mut,
        seeds = [constants::CONFIG_PDA_PREFIX, instance_key.key().as_ref()],
        bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        seeds = [constants::OWNER_PDA_PREFIX, instance_key.key().as_ref()],
        bump
    )]
    pub owner: Account<'info, Ownership>,

    pub instance_key: AccountInfo<'info>,

    #[account(
        mut,
        address = owner.owner @ CustomError::Unauthorized
    )]
    pub signer: Signer<'info>,
}
