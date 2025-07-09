use anchor_lang::prelude::*;

pub mod config;
pub mod consensus;
pub mod constants;
pub mod error;
pub mod instructions;
pub mod oracle_data;
pub mod ownership;

pub use config::*;
pub use consensus::*;
pub use error::*;
pub use instructions::*;
pub use oracle_data::*;
pub use ownership::*;

declare_id!("6GwFiXmSWfh4eY7eZsAFVfm1p2uZgrrkXNUU8epWK29B");

#[program]
pub mod oracle_data_receiver {
    use super::*;

    // This method is supposed to be called only once
    pub fn initialize(ctx: Context<ExecuteInitialize>, params: InitializeParams) -> Result<()> {
        initialize_handler(ctx, params)
    }

    pub fn update_config(
        ctx: Context<ExecuteUpdateConfig>,
        config: UpdateConfigParams,
    ) -> Result<()> {
        update_config_handler(ctx, config)
    }

    // ProposeNewOwner and RevokeProposal are permissioned
    // AcceptOwnership and RejectOwnership are permisson-less
    pub fn update_ownership(
        ctx: Context<ExecuteUpdateOwnership>,
        params: UpdateOwnershipParams,
    ) -> Result<()> {
        update_ownership_handler(ctx, params)
    }

    pub fn add_oracle(ctx: Context<ExecuteUpdateOracle>, oracle: Pubkey) -> Result<()> {
        add_oracle_handler(ctx, oracle)
    }

    pub fn remove_oracle(ctx: Context<ExecuteRemoveOracle>, oracle: Pubkey) -> Result<()> {
        remove_oracle_handler(ctx, oracle)
    }

    pub fn publish_data<'info>(
        ctx: Context<'_, '_, 'info, 'info, ExecutePublishData<'info>>,
        data: Vec<u8>,
    ) -> Result<()> {
        publish_data_handler(ctx, data)
    }
}
