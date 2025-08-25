use cosmwasm_std::StdError;
use thiserror::Error;

pub type ConsensusResult<T> = Result<T, ConsensusError>;

#[derive(Error, Debug, PartialEq)]
pub enum ConsensusError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("Prepublish error: {msg}")]
    PrepublishError { msg: String },

    #[error("Consensus threshold should be greater than 0")]
    ZeroThreshold {},

    #[error("Consensus threshold should be less than or equal to the number of messengers")]
    UnreachableThreshold {},

    #[error("Messengers should have at least 1 address")]
    InvalidMessengers {},

    #[error("You already submitted data for this round")]
    DoubleSubmission {},

    #[error("Invalid round: {msg}")]
    InvalidRound { msg: String },

    #[error("Overflow error")]
    Overflow(cosmwasm_std::OverflowError),

    #[error("Division error")]
    CheckedDiv(cosmwasm_std::CheckedFromRatioError),
}

impl From<cosmwasm_std::OverflowError> for ConsensusError {
    fn from(err: cosmwasm_std::OverflowError) -> Self {
        ConsensusError::Overflow(err)
    }
}

impl From<cosmwasm_std::CheckedFromRatioError> for ConsensusError {
    fn from(err: cosmwasm_std::CheckedFromRatioError) -> Self {
        ConsensusError::CheckedDiv(err)
    }
}
