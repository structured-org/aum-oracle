use consensus::error::ConsensusError;
use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    ConsensusError(#[from] ConsensusError),

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

    #[error("Oracle already published data for this slot")]
    AlreadyPublished {},

    #[error("Overflow: {error}")]
    DecimalError { error: String },

    #[error("Slinky BTC/USD price is missing")]
    SlinkyBTCPriceMissing {},

    #[error("Slinky BTC/USD price ({price}) is invalid: {error}")]
    SlinkyBTCPriceIncorrect { price: String, error: String },

    #[error("Slinky BTC/USD price is too old: {price_height}")]
    SlinkyBTCPriceTooOld { price_height: u64 },

    #[error("Decimal range exceeded")]
    DecimalRangeError(cosmwasm_std::SignedDecimal256RangeExceeded),

    #[error("Division error")]
    CheckedDiv(cosmwasm_std::CheckedFromRatioError),

    #[error("Overflow error")]
    OverflowError(cosmwasm_std::OverflowError),
}

impl From<cosmwasm_std::SignedDecimal256RangeExceeded> for ContractError {
    fn from(err: cosmwasm_std::SignedDecimal256RangeExceeded) -> Self {
        ContractError::DecimalRangeError(err)
    }
}

impl From<cosmwasm_std::CheckedFromRatioError> for ContractError {
    fn from(err: cosmwasm_std::CheckedFromRatioError) -> Self {
        ContractError::CheckedDiv(err)
    }
}

impl From<cosmwasm_std::OverflowError> for ContractError {
    fn from(err: cosmwasm_std::OverflowError) -> Self {
        ContractError::OverflowError(err)
    }
}
