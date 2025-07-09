use crate::{config::Config, constants, error::CustomError, ownership::Ownership};
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum UpdateOwnershipAction {
    ProposeNewOwner { new_owner: Pubkey },
    RevokeProposal {},
    AcceptOwnership {},
    RejectOwnership {},
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateOwnershipParams {
    pub action: UpdateOwnershipAction,
}

pub fn update_ownership_handler<'info>(
    ctx: Context<ExecuteUpdateOwnership<'info>>,
    params: UpdateOwnershipParams,
) -> Result<()> {
    match params.action {
        UpdateOwnershipAction::ProposeNewOwner { new_owner } => {
            if ctx.accounts.signer.key() != ctx.accounts.owner.owner {
                return err!(CustomError::Unauthorized);
            }
            ctx.accounts.owner.pending_owner = Some(new_owner);
            msg!("pendingOwner -- {}", new_owner);
        }
        UpdateOwnershipAction::RevokeProposal {} => {
            if ctx.accounts.signer.key() != ctx.accounts.owner.owner {
                return err!(CustomError::Unauthorized);
            }
            ctx.accounts.owner.pending_owner = None;
            msg!("pendingOwner -- None");
        }
        UpdateOwnershipAction::AcceptOwnership {} => {
            if let Some(pending_owner) = ctx.accounts.owner.pending_owner {
                if ctx.accounts.signer.key() != pending_owner {
                    return err!(CustomError::Unauthorized);
                }
                ctx.accounts.owner.owner = pending_owner;
                ctx.accounts.owner.pending_owner = None;
                msg!("owner -- {}", ctx.accounts.owner.owner);
                msg!("pendingOwner -- None");
            } else {
                return err!(CustomError::NoPendingOwnership);
            }
        }
        UpdateOwnershipAction::RejectOwnership {} => {
            if let Some(pending_owner) = ctx.accounts.owner.pending_owner {
                if ctx.accounts.signer.key() != pending_owner {
                    return err!(CustomError::Unauthorized);
                }
                ctx.accounts.owner.pending_owner = None;
                msg!("pendingOwner -- None");
            } else {
                return err!(CustomError::NoPendingOwnership);
            }
        }
    }
    Ok(())
}

#[derive(Accounts)]
pub struct ExecuteUpdateOwnership<'info> {
    #[account(
        seeds = [constants::CONFIG_PDA_PREFIX, instance_key.key().as_ref()],
        bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [constants::OWNER_PDA_PREFIX, instance_key.key().as_ref()],
        bump
    )]
    pub owner: Account<'info, Ownership>,

    pub instance_key: AccountInfo<'info>,

    #[account(mut)]
    pub signer: Signer<'info>,
}
