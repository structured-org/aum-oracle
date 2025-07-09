use anchor_lang::prelude::*;

use crate::{constants, Config, CustomError, Ownership};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateConfigParams {
    pub round_time: Option<u16>,
}

pub fn update_config_handler(
    ctx: Context<ExecuteUpdateConfig>,
    params: UpdateConfigParams,
) -> Result<()> {
    if let Some(round_time) = params.round_time {
        ctx.accounts.config.round_time = round_time;
        msg!("roundTime -- {}", round_time);
    }

    Ok(())
}

#[derive(Accounts)]
pub struct ExecuteUpdateConfig<'info> {
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
