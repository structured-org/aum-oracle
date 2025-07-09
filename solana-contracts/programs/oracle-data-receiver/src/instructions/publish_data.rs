use anchor_lang::prelude::*;
use sha2::{Digest, Sha256};

use crate::{constants, Config, Consensus, CustomError, OracleData};

pub fn publish_data_handler<'info>(ctx: Context<'_, '_, 'info, 'info, ExecutePublishData<'info>>, data: Vec<u8>) -> Result<()> {
    let config = &ctx.accounts.config;
    let clock = Clock::get()?;
    let timestamp = clock.unix_timestamp as u64;

    require!(
        config.oracles.contains(&ctx.accounts.signer.key()),
        CustomError::Unauthorized
    );

    let mut hasher = Sha256::new();
    hasher.update(&data);
    let hash = hasher.finalize().to_vec();

    let record = &mut ctx.accounts.oracle_data;
    record.oracle = ctx.accounts.signer.key();
    record.data = data.clone();
    record.hash = hash;
    record.timestamp = clock.unix_timestamp as u64;

    // Collect valid submissions
    let ra = ctx.remaining_accounts;
    let mut valid: Vec<Account<OracleData>> = ra
        .into_iter()
        .filter_map(|acc_info| {
            if let Ok(loaded) = Account::<OracleData>::try_from(acc_info) {
                Some(loaded)
            } else {

                None
            }
        })
        .collect();

    valid.push(record.clone());

    // Count hashes
    let mut hash_map = std::collections::HashMap::new();
    for rec in valid.iter() {
        *hash_map.entry(&rec.hash).or_insert(0u32) += 1;
    }

    msg!("hash_map -- {:?}", hash_map);

    // Threshold check
    let threshold = ((config.oracles.len() * 2 + 2) / 3) as u32; // ceil(2/3)

    for (_, count) in hash_map {
        if count >= threshold {
            let consensus = &mut ctx.accounts.consensus;
            consensus.timestamp = timestamp;
            consensus.data = data.clone();

            break;
        }
    }

    Ok(())
}

#[derive(Accounts)]
pub struct ExecutePublishData<'info> {
    #[account(
        seeds = [constants::CONFIG_PDA_PREFIX, config.instance_key.key().as_ref()],
        bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        init_if_needed,
        payer = signer,
        space = 8 + OracleData::INIT_SPACE,
        seeds = [constants::ORACLE_DATA_PDA_PREFIX, config.instance_key.key().as_ref(), signer.key().as_ref()],
        bump
    )]
    pub oracle_data: Account<'info, OracleData>,

    #[account(
        mut, 
        seeds = [constants::CONSENSUS_PDA_PREFIX, config.instance_key.key().as_ref()], 
        bump
    )]
    pub consensus: Account<'info, Consensus>,

    #[account(
        mut,
    )]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}
