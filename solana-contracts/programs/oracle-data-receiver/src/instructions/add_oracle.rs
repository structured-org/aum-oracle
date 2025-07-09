use anchor_lang::prelude::*;

use crate::{constants, Config, CustomError, Ownership};

pub fn add_oracle_handler(ctx: Context<ExecuteUpdateOracle>, oracle: Pubkey) -> Result<()> {
    let config = &mut ctx.accounts.config;
    if !config.oracles.contains(&oracle) {
        config.oracles.push(oracle);
    }
    Ok(())
}

#[derive(Accounts)]
#[instruction(oracle: Pubkey)]
pub struct ExecuteUpdateOracle<'info> {
    #[account(
        mut,
        seeds = [constants::CONFIG_PDA_PREFIX, config.instance_key.key().as_ref()],
        bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        seeds = [constants::OWNER_PDA_PREFIX, config.instance_key.key().as_ref()],
        bump
    )]
    pub owner: Account<'info, Ownership>,

    #[account(
        mut,
        address = owner.owner @ CustomError::Unauthorized
    )]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}
