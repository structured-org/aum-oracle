use consensus::error::ConsensusError;
use cosmwasm_std::{SignedDecimal256RangeExceeded, StdError};
use thiserror::Error;

pub type ContractResult<T> = Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    ConsensusError(#[from] ConsensusError),

    #[error("Failed to convert value to SignedDecimal256: {msg:?}")]
    SignedDecimal256RangeExceeded{msg: String},

    #[error("Invalid Binance data: {msg:?}")]
    InvalidBinanceData { msg: String },

    #[error("Price is invalid")]
    InvalidPrice,

    #[error("Failed to convert value to Decimal256")]
    Decimal256ConversionError,

    #[error("Too many Decimal256s from oracle responce, exceeds u32 allowance")]
    TooManyDecimal256s,

    #[error("Market {symbol}, {quote} did not return an block height")]
    PriceAgeUnavailable { symbol: String, quote: String },

    #[error("Market {symbol}, {quote} did not return a block height")]
    PriceNotAvailable { symbol: String, quote: String },

    #[error("Market {symbol}, {quote} returned a nil price")]
    PriceIsNil { symbol: String, quote: String },

    #[error("Timestamp for {symbol}, {quote} price is nil")]
    TimestampIsNil { symbol: String, quote: String },

    #[error("Market {symbol}, {quote} is older than {max_seconds} seconds")]
    PriceTooOld {
        symbol: String,
        quote: String,
        max_seconds: u64,
    },

    #[error("Published data is too old")]
    PublishedDataTooOld {},

    #[error("Price cannot be negative")]
    PriceIsNegative,

    #[error("Msg sender must be the contract owner")]
    Unauthorized,

    #[error("Price oracle error: {msg}")]
    PriceOracleError { msg: String },

    #[error("Overflow error")]
    Overflow(cosmwasm_std::OverflowError),

    #[error("Division error")]
    CheckedDiv(cosmwasm_std::CheckedFromRatioError)
}

impl From<cosmwasm_std::OverflowError> for ContractError {
    fn from(err: cosmwasm_std::OverflowError) -> Self {
        ContractError::Overflow(err)
    }
}

impl From<SignedDecimal256RangeExceeded> for ContractError {
    fn from(err: SignedDecimal256RangeExceeded) -> Self {
        ContractError::SignedDecimal256RangeExceeded{msg: err.to_string()}
    }
}

impl From<cosmwasm_std::CheckedFromRatioError> for ContractError {
    fn from(err: cosmwasm_std::CheckedFromRatioError) -> Self {
        ContractError::CheckedDiv(err)
    }
}
