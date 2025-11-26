use crate::types::{Config, State};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::SignedDecimal256;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_serde]
pub struct InstantiateMsg {
    pub config: Config,
    pub owner: String,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig { new_config: UpdateConfig },
    Lock { amount: SignedDecimal256 },
    Unlock {},
}

#[cw_serde]
pub struct UpdateConfig {
    pub locker: Option<String>,
    pub unlocker: Option<String>,
    pub contract: Option<String>,
    pub asset: Option<String>,
    pub aum_stale_period: Option<u64>,
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    /// Returns contract's configuration.
    GetConfig {},

    #[returns(State)]
    GetState {},
}

/// MigrateMsg is used for contract migration.
#[cw_serde]
pub struct MigrateMsg {}
