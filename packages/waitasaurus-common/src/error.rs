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
    Unauthorized {},

    #[error("Already Locked")]
    AlreadyLocked {},

    #[error("Already Unlocked")]
    AlreadyUnlocked {},

    #[error("No data available in the target contract")]
    NoDataInContract {},

    #[error("No asset found in the published data")]
    NoAssetFound {},

    #[error("Insufficient asset amount to unlock")]
    InsufficientAssetAmount {},
}
