use consensus::error::ConsensusError;
use cosmwasm_std::{SignedDecimal256RangeExceeded, StdError};
use cw_ownable::OwnershipError;
use thiserror::Error;

pub type ContractResult<T> = Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("{0}")]
    Ownable(#[from] OwnershipError),

    #[error(transparent)]
    ConsensusError(#[from] ConsensusError),

    #[error("Consensus period should be greater than 0")]
    InvalidConsensusPeriod {},

    #[error("Price data period should be greater than 0")]
    InvalidPriceDataPeriod {},

    #[error("Failed to convert value to SignedDecimal256: {msg:?}")]
    SignedDecimal256RangeExceeded { msg: String },

    #[error("Invalid Binance data: {msg:?}")]
    InvalidBinanceData { msg: String },

    #[error("Failed to convert value to Decimal256")]
    Decimal256ConversionError,

    #[error("Published data is too old")]
    PublishedDataTooOld {},

    #[error("No data published")]
    NoDataPublished {},

    #[error("Msg sender must be the contract owner")]
    Unauthorized,

    #[error("Price oracle error: {msg}")]
    PriceOracleError { msg: String },

    #[error("Overflow error")]
    Overflow(cosmwasm_std::OverflowError),

    #[error("Division error")]
    CheckedDiv(cosmwasm_std::CheckedFromRatioError),
}

impl From<cosmwasm_std::OverflowError> for ContractError {
    fn from(err: cosmwasm_std::OverflowError) -> Self {
        ContractError::Overflow(err)
    }
}

impl From<SignedDecimal256RangeExceeded> for ContractError {
    fn from(err: SignedDecimal256RangeExceeded) -> Self {
        ContractError::SignedDecimal256RangeExceeded {
            msg: err.to_string(),
        }
    }
}

impl From<cosmwasm_std::CheckedFromRatioError> for ContractError {
    fn from(err: cosmwasm_std::CheckedFromRatioError) -> Self {
        ContractError::CheckedDiv(err)
    }
}
