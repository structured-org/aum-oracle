use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use thiserror::Error;

pub type ContractResult<T> = Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("{0}")]
    Ownable(#[from] OwnershipError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Next TWAER publication will be available at timestamp {next_pub_time}")]
    PublicationToSoon { next_pub_time: u64 },

    #[error("TWA exchange rate not yet calculated")]
    TwaerNotCalculated,

    #[error("Overflow error")]
    Overflow(cosmwasm_std::OverflowError),

    #[error("Division error")]
    CheckedDiv(cosmwasm_std::CheckedFromRatioError),

    #[error("Duplicate data point at timestamp {timestamp}")]
    DuplicateDataPoint { timestamp: u64 },

    #[error("No next timestamp found for expired rate at timestamp {timestamp}")]
    NoNextTimestamp { timestamp: u64 },

    #[error("No latest exchange rate found")]
    NoLatestRate,

    #[error("No earliest exchange rate found")]
    NoEarliestRate,

    #[error("maxBTC denom is not set")]
    MaxBTCDenomNotSet,

    #[error("Conversion error: {msg}")]
    ConversionError { msg: String },

    #[error("TWAER difference too large: new={new_twaer}, previous={prev_twaer}, diff_ppm={diff_ppm}, max_allowed_ppm={max_allowed_ppm}")]
    TwaerDiffTooLarge {
        new_twaer: String,
        prev_twaer: String,
        diff_ppm: u64,
        max_allowed_ppm: u64,
    },
}

impl From<cosmwasm_std::OverflowError> for ContractError {
    fn from(err: cosmwasm_std::OverflowError) -> Self {
        ContractError::Overflow(err)
    }
}

impl From<cosmwasm_std::CheckedFromRatioError> for ContractError {
    fn from(err: cosmwasm_std::CheckedFromRatioError) -> Self {
        ContractError::CheckedDiv(err)
    }
}
