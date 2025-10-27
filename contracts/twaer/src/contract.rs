use crate::state::{CONFIG, ER_HISTORY, MOCKED_MAXBTC_SUPPLY, TWAER, TWA_AGGREGATOR};
use aum_receiver_common::types::{aum_response_from_uwbtc, GetAumResponse};
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, Int256, MessageInfo,
    Order, Response, StdResult, Storage, Uint128,
};
use cw2::set_contract_version;
use cw_ownable::{get_ownership, update_ownership};
use cw_storage_plus::Bound;
use std::ops::Sub;
use twaer_common::error::{ContractError, ContractResult};
use twaer_common::msg::{
    ErWindowInfoResponse, ExecuteMsg, GetTwaerResponse, InstantiateMsg, MigrateMsg, QueryMsg,
    UpdateConfig,
};
use twaer_common::types::{Config, MaxBTCCoreConfig, TwaAggregator};

const CONTRACT_NAME: &str = "crates.io:twaer";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    cw_ownable::initialize_owner(deps.storage, deps.api, Some(msg.owner.as_str()))?;

    let oracles: Vec<Addr> = msg
        .aum_oracles
        .iter()
        .map(|addr| deps.api.addr_validate(addr))
        .collect::<StdResult<_>>()?;

    let config = Config {
        recorder: deps.api.addr_validate(&msg.recorder)?,
        publisher: deps.api.addr_validate(&msg.publisher)?,
        aum_oracles: oracles,
        maxbtc_core_contract: deps.api.addr_validate(&msg.maxbtc_core_contract)?,
        twa_window_seconds: msg.twa_window_seconds,
        twaer_immutability_seconds: msg.twaer_immutability_seconds,
    };
    CONFIG.save(deps.storage, &config)?;
    MOCKED_MAXBTC_SUPPLY.save(deps.storage, &msg.mocked_maxbtc_supply)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig { new_config } => {
            Ok(execute_update_config(deps, info, new_config)?)
        }
        ExecuteMsg::RecordEr {} => Ok(execute_record_er(deps, env, info)?),
        ExecuteMsg::PublishTwaer {} => Ok(execute_publish_twaer(deps, env, info)?),
        ExecuteMsg::SetMockedMaxbtcSupply { value } => {
            Ok(execute_set_maxbtc_supply(deps, env, info, value)?)
        }
        ExecuteMsg::ResetTwaerTo { value } => Ok(execute_reset_twaer_to(deps, env, info, value)?),
        ExecuteMsg::UpdateOwnership(action) => {
            update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::new().add_attribute("action", "update_ownership"))
        }
        ExecuteMsg::RemoveERDatapoint { er_timestamp } => {
            execute_remove_er_datapoint(deps, env, info, er_timestamp)
        }
        ExecuteMsg::Unmock {} => execute_unmock(deps, info),
    }
}

fn execute_unmock(deps: DepsMut, info: MessageInfo) -> ContractResult<Response> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    MOCKED_MAXBTC_SUPPLY.remove(deps.storage);
    Ok(Response::new().add_attribute("action", "unmock"))
}

fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_config: UpdateConfig,
) -> ContractResult<Response> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut config = CONFIG.load(deps.storage)?;

    // Update configuration if fields are provided
    if let Some(ref oracles) = new_config.aum_oracles {
        let validated_oracles: Vec<Addr> = oracles
            .iter()
            .map(|addr| deps.api.addr_validate(addr))
            .collect::<StdResult<_>>()?;
        config.aum_oracles = validated_oracles;
    }
    if let Some(new_recorder) = new_config.recorder {
        let validated_new_recorder = deps.api.addr_validate(&new_recorder)?;
        config.recorder = validated_new_recorder;
    }
    if let Some(new_publisher) = new_config.publisher {
        let validated_new_publisher = deps.api.addr_validate(&new_publisher)?;
        config.publisher = validated_new_publisher;
    }
    if let Some(maxbtc_core) = new_config.maxbtc_core_contract {
        config.maxbtc_core_contract = deps.api.addr_validate(&maxbtc_core)?;
    }
    if let Some(twa_window_seconds) = new_config.twa_window_seconds {
        config.twa_window_seconds = twa_window_seconds;
    }
    if let Some(twaer_immutability_seconds) = new_config.twaer_immutability_seconds {
        config.twaer_immutability_seconds = twaer_immutability_seconds;
    }

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_config"))
}

fn execute_record_er(deps: DepsMut, env: Env, info: MessageInfo) -> ContractResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    if !cw_ownable::is_owner(deps.storage, &info.sender)? && info.sender != config.recorder {
        return Err(ContractError::Unauthorized {});
    }

    let exchange_rate = calc_exchange_rate(deps.as_ref())?;
    let timestamp = env.block.time.seconds();
    record_er_at(deps.storage, exchange_rate, timestamp)?;

    Ok(Response::new().add_attributes([
        ("action", "record_er"),
        ("exchange_rate", &exchange_rate.to_string()),
    ]))
}

fn execute_publish_twaer(deps: DepsMut, env: Env, info: MessageInfo) -> ContractResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    if !cw_ownable::is_owner(deps.storage, &info.sender)? && info.sender != config.publisher {
        return Err(ContractError::Unauthorized {});
    }

    let prev_pub_time = TWAER.load(deps.storage).unwrap_or_default().1;
    let next_pub_time = prev_pub_time + config.twaer_immutability_seconds;
    if env.block.time.seconds() < next_pub_time {
        return Err(ContractError::PublicationToSoon { next_pub_time });
    }

    let twaer = calculate_twaer(deps.as_ref(), env.clone())?;
    TWAER.save(deps.storage, &(twaer, env.block.time.seconds()))?;

    Ok(
        Response::new()
            .add_attributes([("action", "publish_twaer"), ("twaer", &twaer.to_string())]),
    )
}

fn execute_reset_twaer_to(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    value: Decimal,
) -> ContractResult<Response> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    TWAER.save(deps.storage, &(value, env.block.time.seconds()))?;
    TWA_AGGREGATOR.save(
        deps.storage,
        &TwaAggregator::from_single_point(env.block.time.seconds(), value),
    )?;
    ER_HISTORY.clear(deps.storage);
    ER_HISTORY.save(deps.storage, env.block.time.seconds(), &value)?;

    Ok(Response::new()
        .add_attributes([("action", "reset_twaer_to"), ("value", &value.to_string())]))
}

fn execute_set_maxbtc_supply(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    value: Uint128,
) -> ContractResult<Response> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    MOCKED_MAXBTC_SUPPLY.save(deps.storage, &value)?;

    Ok(Response::new()
        .add_attribute("action", "set_mocked_maxbtc_supply")
        .add_attribute("value", value.to_string()))
}

fn execute_remove_er_datapoint(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    er_timestamp: u64,
) -> ContractResult<Response> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if let Some(new_twa_aggr) = remove_er_contribution(deps.storage, er_timestamp)? {
        TWA_AGGREGATOR.save(deps.storage, &new_twa_aggr)?;
    } else {
        TWA_AGGREGATOR.remove(deps.storage);
    }

    Ok(Response::new()
        .add_attribute("action", "execute_remove_er_datapoint")
        .add_attribute("er_timestamp", er_timestamp.to_string()))
}

/// Determine which exchange rates are expired and return them in ascending timestamp order.
fn determine_expired_rates(
    storage: &mut dyn cosmwasm_std::Storage,
    window_start: u64,
) -> ContractResult<Vec<(u64, Decimal)>> {
    let expired_data: Vec<(u64, Decimal)> = ER_HISTORY
        .range(
            storage,
            None,                                 // Start from oldest
            Some(Bound::exclusive(window_start)), // Stop at window_start (oldest in-window record)
            Order::Ascending,
        )
        .collect::<StdResult<Vec<_>>>()?;

    Ok(expired_data)
}

/// Stores an exchange rate in the history and updates the TWA aggregator with the exchange
/// rate data point and manages the time window by removing expired entries. This function
/// performs incremental TWA calculation by:
/// 1. Adding the weighted contribution of the most recent rate for the time period since
///    the last update;
/// 2. Removing weighted contributions from exchange rates that have expired outside the
///    configured TWA window;
/// 3. Recalculating the current TWA based on the updated weighted sum and total duration;
/// 4. Updating aggregator metadata (window boundaries, current TWA).
fn record_er_at(
    storage: &mut dyn cosmwasm_std::Storage,
    new_rate: Decimal,
    new_timestamp: u64,
) -> ContractResult<()> {
    let mut twa_aggr = match TWA_AGGREGATOR.may_load(storage)? {
        None => {
            TWA_AGGREGATOR.save(
                storage,
                &TwaAggregator::from_single_point(new_timestamp, new_rate),
            )?;
            ER_HISTORY.save(storage, new_timestamp, &new_rate)?;
            return Ok(());
        }
        Some(aggr) => aggr,
    };

    // the rate that was active from window_end until now
    let latest_rate_duration = new_timestamp - twa_aggr.window_end;
    if latest_rate_duration == 0 {
        return Err(ContractError::DuplicateDataPoint {
            timestamp: new_timestamp,
        });
    }
    let latest_rate = ER_HISTORY
        .range(storage, None, None, Order::Descending)
        .next()
        .transpose()?
        .map_or_else(|| Err(ContractError::NoLatestRate {}), |(_, rate)| Ok(rate))?;

    let weighted_contribution =
        latest_rate.checked_mul(Decimal::from_ratio(latest_rate_duration, 1u64))?;
    twa_aggr.weighted_sum = twa_aggr.weighted_sum.checked_add(weighted_contribution)?;

    ER_HISTORY.save(storage, new_timestamp, &new_rate)?;

    let config = CONFIG.load(storage)?;
    let window_start = new_timestamp.sub(config.twa_window_seconds);
    let expired_rates = determine_expired_rates(storage, window_start)?;

    // Remove expired rates from history (just removal, no recalculation yet)
    for (expired_timestamp, _) in expired_rates {
        ER_HISTORY.remove(storage, expired_timestamp);
    }

    // Recalculate aggregator based on remaining history after all removals
    if let Some(new_twa_aggr) = recalculate_twa_aggregator(storage)? {
        TWA_AGGREGATOR.save(storage, &new_twa_aggr)?;
    } else {
        TWA_AGGREGATOR.remove(storage);
    }
    Ok(())
}

/// Recalculates the TWA aggregator from scratch based on current ER history
/// Returns None if history is empty
fn recalculate_twa_aggregator(storage: &dyn Storage) -> ContractResult<Option<TwaAggregator>> {
    let history: Vec<(u64, Decimal)> = ER_HISTORY
        .range(storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    if history.is_empty() {
        return Ok(None);
    }

    if history.len() == 1 {
        // Only one point exists
        let (timestamp, rate) = history[0];
        return Ok(Some(TwaAggregator::from_single_point(timestamp, rate)));
    }

    // Calculate weighted sum from consecutive pairs
    let mut weighted_sum = Decimal::zero();
    for i in 0..history.len() - 1 {
        let (start_ts, start_rate) = history[i];
        let (end_ts, _) = history[i + 1];
        let duration = end_ts - start_ts;
        let contribution = start_rate.checked_mul(Decimal::from_ratio(duration, 1u64))?;
        weighted_sum = weighted_sum.checked_add(contribution)?;
    }

    let window_start = history[0].0;
    let window_end = history.last().unwrap().0;
    let total_duration = window_end - window_start;
    let current_twa = weighted_sum.checked_div(Decimal::from_ratio(total_duration, 1u64))?;

    Ok(Some(TwaAggregator {
        weighted_sum,
        current_twa,
        window_start,
        window_end,
    }))
}

/// Removes a specific ER data point from ER history and updates twa
/// The function either returns an updated aggregator that must be saved to the storage
/// or None, meaning the aggregator is empty and the storage must be cleared
fn remove_er_contribution(
    storage: &mut dyn Storage,
    er_timestamp: u64,
) -> ContractResult<Option<TwaAggregator>> {
    ER_HISTORY.remove(storage, er_timestamp);
    recalculate_twa_aggregator(storage)
}

fn calc_exchange_rate(deps: Deps) -> ContractResult<Decimal> {
    let config = CONFIG.load(deps.storage)?;

    let maxbtc_supply = match MOCKED_MAXBTC_SUPPLY.may_load(deps.storage)? {
        Some(mocked_supply) => mocked_supply,
        None => {
            let maxbtc_config = query_maxbtc_config(deps, config.maxbtc_core_contract.clone())?;
            let maxbtc_denom = "factory/".to_owned()
                + config.maxbtc_core_contract.as_str()
                + "/"
                + maxbtc_config.maxbtc_denom.as_str();
            deps.querier.query_supply(maxbtc_denom)?.amount
        }
    };

    if maxbtc_supply.is_zero() {
        return Ok(Decimal::one());
    }

    let aum = get_aum(deps)?;
    let aum_uint = Uint128::try_from(aum).map_err(|e| ContractError::ConversionError {
        msg: format!("failed to convert total AUM {} to Uint128: {}", aum, e),
    })?;

    Ok(Decimal::from_ratio(aum_uint, maxbtc_supply))
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::GetAum {} => Ok(to_json_binary(&query_get_aum(deps)?)?),
        QueryMsg::GetConfig {} => Ok(to_json_binary(&CONFIG.load(deps.storage)?)?),
        QueryMsg::GetTwaer {} => Ok(to_json_binary(&query_get_twaer(deps)?)?),
        QueryMsg::PredictTwaer {} => Ok(to_json_binary(&calculate_twaer(deps, env)?)?),
        QueryMsg::Ownership {} => Ok(to_json_binary(&get_ownership(deps.storage)?)?),
        QueryMsg::ErWindowInfo {} => Ok(to_json_binary(&query_er_window_info(deps)?)?),
    }
}

fn query_get_aum(deps: Deps) -> ContractResult<GetAumResponse> {
    let aum = get_aum(deps)?;
    Ok(aum_response_from_uwbtc(aum))
}

fn query_get_twaer(deps: Deps) -> ContractResult<GetTwaerResponse> {
    let (twaer, published_at) = TWAER
        .load(deps.storage)
        .map_err(|_| ContractError::TwaerNotCalculated {})?;

    Ok(GetTwaerResponse {
        twaer,
        published_at,
    })
}

pub fn query_maxbtc_config(
    deps: Deps,
    maxbtc_core_contract: Addr,
) -> ContractResult<MaxBTCCoreConfig> {
    let config: MaxBTCCoreConfig = deps.querier.query_wasm_smart(
        maxbtc_core_contract,
        &serde_json::json!({
          "config": {}
        }),
    )?;

    Ok(config)
}

fn get_aum(deps: Deps) -> ContractResult<Int256> {
    let config = CONFIG.load(deps.storage)?;

    let maxbtc_config = query_maxbtc_config(deps, config.maxbtc_core_contract.clone())?;
    let maxbtc_balance = deps
        .querier
        .query_balance(config.maxbtc_core_contract, maxbtc_config.deposit_denom)?;

    let oracles_aum = get_aum_from_oracles(deps)?;

    Ok(oracles_aum + Int256::from(maxbtc_balance.amount))
}

fn get_aum_from_oracles(deps: Deps) -> ContractResult<Int256> {
    let config = CONFIG.load(deps.storage)?;
    let mut total_aum = Int256::zero();
    for oracle in config.aum_oracles.iter() {
        let aum: GetAumResponse = deps.querier.query_wasm_smart(
            oracle,
            &serde_json::json!({
              "get_aum": {}
            }),
        )?;
        total_aum += aum.aum_in_wbtc;
    }

    Ok(total_aum)
}

fn calculate_twaer(deps: Deps, env: Env) -> ContractResult<Decimal> {
    let twa_buffer = TWA_AGGREGATOR.load(deps.storage)?;

    // If no time has passed since the last update, return the buffer TWA
    if env.block.time.seconds() <= twa_buffer.window_end {
        return Ok(twa_buffer.current_twa);
    }

    // Get the last rate from the most recent timestamp
    let last_rate = ER_HISTORY
        .range(deps.storage, None, None, Order::Descending)
        .next()
        .transpose()?
        .map_or_else(|| Err(ContractError::NoLatestRate {}), |(_, rate)| Ok(rate))?;

    // Calculate the additional contribution from the last rate
    let additional_duration = env.block.time.seconds() - twa_buffer.window_end;
    let additional_contribution =
        last_rate.checked_mul(Decimal::from_ratio(additional_duration, 1u64))?;

    // Calculate total weighted sum and duration
    let total_weighted_sum = twa_buffer
        .weighted_sum
        .checked_add(additional_contribution)?;
    let total_duration = twa_buffer.window_end - twa_buffer.window_start + additional_duration;

    if total_duration == 0 {
        return Ok(last_rate);
    }

    // Calculate final TWA
    total_weighted_sum
        .checked_div(Decimal::from_ratio(total_duration, 1u64))
        .map_err(ContractError::CheckedDiv)
}

fn query_er_window_info(deps: Deps) -> ContractResult<ErWindowInfoResponse> {
    let twa_buffer = TWA_AGGREGATOR.load(deps.storage)?;
    let data_points = ER_HISTORY
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(u64, Decimal)>>>()?;
    let total_points = data_points.len() as u64;

    Ok(ErWindowInfoResponse {
        window_start: twa_buffer.window_start,
        window_end: twa_buffer.window_end,
        total_points,
        data_points,
    })
}

/// Migrates the contract
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let old_config = CONFIG.load(deps.storage)?;
    let new_config = Config {
        recorder: deps.api.addr_validate(&msg.recorder)?,
        publisher: old_config.publisher.clone(),
        aum_oracles: old_config.aum_oracles.clone(),
        maxbtc_core_contract: old_config.maxbtc_core_contract.clone(),
        twa_window_seconds: old_config.twa_window_seconds,
        twaer_immutability_seconds: old_config.twaer_immutability_seconds,
    };
    CONFIG.save(deps.storage, &new_config)?;

    Ok(Response::default())
}
