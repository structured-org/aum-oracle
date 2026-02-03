use consensus::error::ConsensusError;
use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    ConsensusError(#[from] ConsensusError),

    #[error("{0}")]
    Ownable(#[from] OwnershipError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Consensus period should be greater than 0")]
    InvalidConsensusPeriod {},

    #[error("Price data period should be greater than 0")]
    InvalidPriceDataPeriod {},

    #[error("Duplicate custody asset found: {asset}")]
    DuplicateCustodyAsset { asset: String },

    #[error("Duplicate solana balance asset found for address {address}: {asset}")]
    DuplicateSolanaBalanceAsset { address: String, asset: String },

    #[error("Duplicate solana token total supply found: {asset}")]
    DuplicateSolanaTokenTotalSupply { asset: String },

    #[error("Duplicate PriceTicker found: {ticker}")]
    DuplicatePriceTicker { ticker: String },

    #[error("JLP token supply tracking is required")]
    JlpTotalSupplyNotTracked {},

    #[error("Asset {asset} from required_solana_balances is not present in solana_slinky_map")]
    AssetNotInSlinkyMap { asset: String },

    #[error("JLP token should not be present in solana_slinky_map")]
    JlpTokenInSlinkyMap {},

    #[error("Crucial consensus data is missing: {details}")]
    CrucialConsensusDataMissing { details: String },

    #[error("No data published yet")]
    NoDataPublished {},

    #[error("Data is too old to be used")]
    PublishedDataTooOld {},

    #[error("Oracle already published data for this slot")]
    AlreadyPublished {},

    #[error("Overflow: {error}")]
    DecimalError { error: String },

    #[error("Slinky {asset}/USD price is missing")]
    SlinkyAssetPriceMissing { asset: String },

    #[error("Slinky {asset}/USD price ({price}) is invalid: {error}")]
    SlinkyAssetPriceIncorrect {
        asset: String,
        price: String,
        error: String,
    },

    #[error("Slinky {asset}/USD price is too old: {price_height}")]
    SlinkyAssetPriceTooOld { asset: String, price_height: u64 },

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
