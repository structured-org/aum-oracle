use cosmwasm_std::StdError;
use thiserror::Error;

pub type ContractResult<T> = Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("Msg sender must be the contract owner")]
    Unauthorized,

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

impl From<cosmwasm_std::CheckedFromRatioError> for ContractError {
    fn from(err: cosmwasm_std::CheckedFromRatioError) -> Self {
        ContractError::CheckedDiv(err)
    }
}
