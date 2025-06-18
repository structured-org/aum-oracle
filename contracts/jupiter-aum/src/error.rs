use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid threshold: {threshold} is greater than the number of oracles {oracles}")]
    InvalidThreshold { threshold: u32, oracles: usize },

    #[error("Period should be greater than 0")]
    InvalidPeriod {},

    #[error("No data published yet")]
    NoDataPublished {},

    #[error("Data is not valid anymore")]
    DataNotValid {},

    #[error("Consensus not reached: {agreements} out of {threshold} required")]
    ConsensusNotReached { agreements: u32, threshold: u32 },

    #[error("Oracle already published data for this slot")]
    AlreadyPublished {},

    #[error("Invalid Solana slot: Data must be from an N-th slot ({extract_period})")]
    InvalidSolanaSlot { extract_period: u64 },

    #[error("Invalid Solana slot: New data slot ({new_slot}) must be greater than last published slot ({last_slot})")]
    SlotTooOld { new_slot: u64, last_slot: u64 },

    #[error("Slinky oracle price query failed")]
    SlinkyOracleQueryFailed {},

    #[error("Overflow: {error}")]
    DecimalError { error: String },

    #[error("Slinky BTC/USD price is missing")]
    SlinkyBTCPriceMissing {},

    #[error("Slinky BTC/USD price ({price}) is invalid: {error}")]
    SlinkyBTCPriceIncorrect { price: String, error: String },
}
