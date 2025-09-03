use anchor_lang::prelude::*;

use crate::{constants, Config, Consensus, CustomError, Ownership};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeParams {
    pub round_time: u16,
    pub owner: Pubkey,
}

pub fn initialize_handler(ctx: Context<ExecuteInitialize>, params: InitializeParams) -> Result<()> {
    ctx.accounts.config.round_time = params.round_time;
    msg!("roundTime -- {}", ctx.accounts.config.round_time);

    ctx.accounts.config.oracles = vec![];

    ctx.accounts.config.instance_key = ctx.accounts.instance_key.key();

    if params.owner == Pubkey::default() {
        return err!(CustomError::InvalidPubkey);
    }
    ctx.accounts.owner.owner = ctx.accounts.signer.key();
    msg!("owner -- {:?}", ctx.accounts.owner.owner);

    let consensus = &mut ctx.accounts.consensus;
    consensus.timestamp = 0;
    consensus.data = vec![];

    Ok(())
}

#[derive(Accounts)]
pub struct ExecuteInitialize<'info> {
    #[account(
        init,
        seeds = [constants::CONFIG_PDA_PREFIX, instance_key.key().as_ref()],
        bump,
        payer = signer,
        space = 8 + Config::INIT_SPACE
    )]
    pub config: Account<'info, Config>,

    #[account(
        init, 
        payer = signer,
        space = 8 + Consensus::INIT_SPACE,
        seeds = [constants::CONSENSUS_PDA_PREFIX, instance_key.key().as_ref()], bump
    )]
    pub consensus: Account<'info, Consensus>,

    #[account(
        init,
        seeds = [constants::OWNER_PDA_PREFIX, instance_key.key().as_ref()],
        bump,
        payer = signer,
        space = 8 + Ownership::INIT_SPACE
    )]
    pub owner: Account<'info, Ownership>,

    /// An ephemeral signer that is used as a seed for the jupiter helper PDA.
    /// Must be a signer to prevent front-running attack by someone else but the original creator.
    pub instance_key: Signer<'info>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}
