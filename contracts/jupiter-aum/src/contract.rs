use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, GetAUMResponse, GetDataResponse, InstantiateMsg, MigrateMsg,
    QueryMsg,
};
use crate::state::{
    Config, PendingData, PublishedData, SolanaData, CONFIG, LAST_PUBLISHED_DATA, PENDING_DATA,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdError, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw_storage_plus::PrefixBound;
use neutron_std::types::slinky::oracle::v1::OracleQuerier;
use neutron_std::types::slinky::types::v1::CurrencyPair;
use std::str::FromStr;

const CONTRACT_NAME: &str = "crates.io:jupiter-aum";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// USD/BTC query constants
const USD_DENOM: &'static str = "USD";
const BTC_DENOM: &'static str = "BTC";

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = deps.api.addr_validate(&msg.admin)?;
    let oracles: Result<Vec<Addr>, _> = msg
        .oracles
        .into_iter()
        .map(|addr| deps.api.addr_validate(&addr))
        .collect();
    let oracles = oracles?;

    let config = Config {
        admin,
        oracles,
        threshold: msg.threshold,
        extract_period: msg.extract_period,
        valid_period: msg.valid_period,
    };
    config.validate()?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", config.admin.to_string()))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            admin,
            oracles,
            threshold,
            extract_period,
            valid_period,
        } => update_config(
            deps,
            info,
            admin,
            oracles,
            threshold,
            extract_period,
            valid_period,
        ),
        ExecuteMsg::PublishData {
            timestamp,
            slot,
            custody_assets,
            aum_usd,
            jlp_total_supply,
            strategy_jlp_balance,
        } => publish_data(
            deps,
            env,
            info,
            SolanaData {
                timestamp,
                slot,
                custody_assets,
                aum_usd,
                jlp_total_supply,
                strategy_jlp_balance,
            },
        ),
    }
}

/// Updates configuration parameters for the contract.
/// Only the current admin can call this method.
#[allow(clippy::too_many_arguments)]
fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    admin: Option<String>,
    oracles: Option<Vec<String>>,
    threshold: Option<u32>,
    extract_period: Option<u64>,
    valid_period: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // Ensure only the contract admin can update the configuration
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(admin) = admin {
        config.admin = deps.api.addr_validate(&admin)?;
    }

    if let Some(oracles_addrs) = oracles {
        let new_oracles: Result<Vec<Addr>, _> = oracles_addrs
            .into_iter()
            .map(|addr| deps.api.addr_validate(&addr))
            .collect();
        config.oracles = new_oracles?;
    }

    if let Some(new_threshold) = threshold {
        config.threshold = new_threshold;
    }

    if let Some(new_extract_period) = extract_period {
        config.extract_period = new_extract_period;
    }

    if let Some(new_valid_period) = valid_period {
        config.valid_period = new_valid_period;
    }

    config.validate()?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

/// Allows an oracle to publish Solana data.
/// Only registered oracles can call this.
/// Oracle can only publish once per slot.
/// If consensus is reached, the `LAST_PUBLISHED_DATA` is updated.
fn publish_data(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_data: SolanaData,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Permission check: only registered oracles can publish data
    if !config.oracles.contains(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    // if new_data.slot % config.N ≠ 0: reject such publishing
    if new_data.slot % config.extract_period != 0 {
        return Err(ContractError::InvalidSolanaSlot {
            extract_period: config.extract_period,
        });
    }

    // if new_data.slot <= last_published_data.slot: reject such publishing
    if let Ok(last_published) = LAST_PUBLISHED_DATA.load(deps.storage) {
        if new_data.slot <= last_published.data.slot {
            return Err(ContractError::SlotTooOld {
                new_slot: new_data.slot,
                last_slot: last_published.data.slot,
            });
        }
    }

    // write into the key that includes hash so that we can track whether we reached consensus for the slot
    let new_data_hash = new_data.hash()?;
    let pending_slot_key = (new_data.slot, new_data_hash.clone());
    let mut pending_slots = PENDING_DATA
        .may_load(deps.storage, pending_slot_key.clone())?
        .unwrap_or_default();

    // Check if the oracle has already published for this slot
    if pending_slots.iter().any(|s| s.oracle == info.sender) {
        return Err(ContractError::AlreadyPublished {});
    }

    // Save the new_data to the pending_data[(slot, data_hash)] state
    let new_pending_data = PendingData {
        data: new_data.clone(),
        oracle: info.sender.clone(),
    };

    pending_slots.push(new_pending_data);
    PENDING_DATA.save(deps.storage, pending_slot_key, &pending_slots)?;

    let mut response = Response::new()
        .add_attribute("action", "publish_data")
        .add_attribute("slot", new_data.slot.to_string())
        .add_attribute("oracle", info.sender.to_string());

    // Check that consensus is reached or not for the new_data.slot
    let consensus_reached = pending_slots.len() as u32 >= config.threshold;
    if consensus_reached {
        // If consensus is reached, rewrite last_published_data item in the State
        LAST_PUBLISHED_DATA.save(
            deps.storage,
            &PublishedData {
                data: new_data.clone(),
                published_at: env.block.time,
            },
        )?;
        response = response
            .add_attribute("consensus_reached", "true")
            .add_attribute("published_at", env.block.time.to_string());

        // Clear pending data for this slot to save space
        let obsolete_data: Vec<((u64, String), _)> = PENDING_DATA
            .prefix_range(
                deps.as_ref().storage,
                None,
                Some(PrefixBound::inclusive(new_data.slot)),
                Order::Ascending,
            )
            .collect::<StdResult<Vec<(_, _)>>>()?;
        for (key, _) in obsolete_data {
            PENDING_DATA.remove(deps.storage, key);
        }
    }

    Ok(response)
}

// ----------------------------------------
//  Queries
// ----------------------------------------
#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::GetData {} => to_json_binary(&query_get_data(deps)?),
        QueryMsg::GetAUM {} => to_json_binary(&query_get_aum(deps, env)?),
    }
}

/// Returns the current contract configuration.
fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        admin: config.admin.to_string(),
        oracles: config.oracles.iter().map(|a| a.to_string()).collect(),
        threshold: config.threshold,
        extract_period: config.extract_period,
        valid_period: config.valid_period,
    })
}

/// Returns the last successfully published and finalized Solana data.
fn query_get_data(deps: Deps) -> StdResult<GetDataResponse> {
    let last_published_data = LAST_PUBLISHED_DATA.load(deps.storage).ok();
    Ok(GetDataResponse {
        data: last_published_data.map(|d| d.data),
    })
}

/// Returns Jupiter AUM value represented in BTC.
fn query_get_aum(deps: Deps, env: Env) -> StdResult<GetAUMResponse> {
    let config = CONFIG.load(deps.storage)?;
    let data = LAST_PUBLISHED_DATA
        .load(deps.storage)
        .map_err(|_| StdError::generic_err("no data published"))?
        .data;

    // TODO: use data.timestamp or lastPublishedData.timestamp?
    if env.block.time.seconds() > data.timestamp.seconds() + config.valid_period {
        // return Err(ContractError::DataNotValid {});
        return Err(StdError::generic_err("DataNotValid"));
    }

    // TODO: downgrade conversion?
    let btc_price_in_usd = query_btc_price_in_usd(deps)?;
    let aum_in_btc = calculate_aum_in_btc(data, btc_price_in_usd)?;

    Ok(GetAUMResponse { aum_in_btc })
}

fn query_btc_price_in_usd(deps: Deps) -> Result<Uint128, StdError> {
    let querier = OracleQuerier::new(&deps.querier);
    let btc_usd_price_result = querier.get_price(Some(CurrencyPair {
        base: USD_DENOM.to_string(),
        quote: BTC_DENOM.to_string(),
    }))?;
    // TODO: think about rounding
    let btc_price_in_usd = Uint128::from_str(
        &btc_usd_price_result
            .price
            .ok_or(StdError::generic_err("no price for BTC/USD pair"))?
            .price,
    )? / Uint128::new(10).pow(btc_usd_price_result.decimals as u32); // TODO: downgrade conversion?
    Ok(btc_price_in_usd)
}

pub fn calculate_aum_in_btc(data: SolanaData, btc_price_in_usd: Uint128) -> StdResult<Uint128> {
    let jlp_virtual_price = data.aum_usd.checked_div(data.jlp_total_supply)?;
    let jlp_balance_in_usd = jlp_virtual_price * data.strategy_jlp_balance;
    let aum_in_btc = jlp_balance_in_usd.checked_div(btc_price_in_usd)?;
    Ok(aum_in_btc)
}

// ----------------------------------------
//  Migration
// ----------------------------------------
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
