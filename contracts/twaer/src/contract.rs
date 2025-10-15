use crate::state::{CONFIG, ER_HISTORY, MOCKED_MAXBTC_SUPPLY, TWAER, TWA_AGGREGATOR};
use aum_receiver_common::types::{aum_response_from_uwbtc, GetAumResponse};
use cosmwasm_std::{entry_point, to_json_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, Int256, MessageInfo, Order, Response, StdResult, Storage, Uint128};
use cw2::set_contract_version;
use cw_ownable::{get_ownership, update_ownership};
use cw_storage_plus::Bound;
use std::ops::Sub;
use twaer_common::error::{ContractError, ContractResult};
use twaer_common::msg::{
    ErWindowInfoResponse, ExecuteMsg, GetTwaerResponse, InstantiateMsg, MigrateMsg, QueryMsg,
    UpdateConfig,
};
use twaer_common::types::{Config, TwaAggregator};

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
        publisher: deps.api.addr_validate(&msg.publisher)?,
        aum_oracles: oracles,
        maxbtc_denom: None,
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
        ExecuteMsg::RemoveERDatapoint {er_timestamp} => execute_remove_er_datapoint(deps, env, info, er_timestamp)
    }
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
    if let Some(new_publisher) = new_config.publisher {
        let validated_new_publisher = deps.api.addr_validate(&new_publisher)?;
        config.publisher = validated_new_publisher;
    }
    if let Some(maxbtc_denom) = new_config.maxbtc_denom {
        config.maxbtc_denom = maxbtc_denom;
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

fn execute_record_er(deps: DepsMut, env: Env, _info: MessageInfo) -> ContractResult<Response> {
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

    let mut twa_aggr = TWA_AGGREGATOR.load(deps.storage)?;

    twa_aggr = remove_er_contribution(deps.storage, twa_aggr, er_timestamp)?;

    TWA_AGGREGATOR.save(deps.storage, &twa_aggr)?;

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

    // Remove contributions from rates that are no longer in the window
    for (expired_timestamp, _) in expired_rates {
        twa_aggr = remove_er_contribution(storage, twa_aggr, expired_timestamp)?
    }

    twa_aggr.window_end = new_timestamp;
    let oldest_timestamp = ER_HISTORY
        .range(storage, None, None, Order::Ascending)
        .next()
        .transpose()?
        .map_or_else(|| Err(ContractError::NoEarliestRate {}), |(ts, _)| Ok(ts))?;
    twa_aggr.window_start = oldest_timestamp;

    let total_duration = twa_aggr.window_end - twa_aggr.window_start;
    twa_aggr.current_twa = if total_duration > 0 {
        twa_aggr
            .weighted_sum
            .checked_div(Decimal::from_ratio(total_duration, 1u64))?
    } else {
        new_rate
    };

    TWA_AGGREGATOR.save(storage, &twa_aggr)?;
    Ok(())
}

/// Removes a specific ER data point from ER history and twa aggregator
/// The function returns updated TwaAggregator that must be saved to the storage manually
fn remove_er_contribution(storage: &mut dyn Storage, mut twa_aggr: TwaAggregator, er_timestamp: u64) -> ContractResult<TwaAggregator> {
    let rate = ER_HISTORY.load(storage, er_timestamp)?;

    let next_timestamp = find_next_timestamp_after(storage, er_timestamp)?.ok_or(
        ContractError::NoNextTimestamp {
            timestamp: er_timestamp,
        },
    )?;
    // how long the rate was active
    let expired_duration = next_timestamp - er_timestamp;
    // contribution of the rate
    let expired_contribution =
        rate.checked_mul(Decimal::from_ratio(expired_duration, 1u64))?;

    twa_aggr.weighted_sum = twa_aggr.weighted_sum.checked_sub(expired_contribution)?;

    ER_HISTORY.remove(storage, er_timestamp);

    Ok(twa_aggr)
}

fn calc_exchange_rate(deps: Deps) -> ContractResult<Decimal> {
    let config = CONFIG.load(deps.storage)?;

    let maxbtc_supply = match config.maxbtc_denom {
        Some(maxbtc_denom) => deps.querier.query_supply(maxbtc_denom)?.amount,
        None => MOCKED_MAXBTC_SUPPLY.load(deps.storage)?,
    };

    if maxbtc_supply.is_zero() {
        return Ok(Decimal::one());
    }

    let aum = get_aum_from_oracles(deps)?;
    let aum_uint = Uint128::try_from(aum).map_err(|e| ContractError::ConversionError {
        msg: format!("failed to convert total AUM {} to Uint128: {}", aum, e),
    })?;

    Ok(Decimal::from_ratio(aum_uint, maxbtc_supply))
}

/// Find the next timestamp after the given timestamp
fn find_next_timestamp_after(
    storage: &dyn cosmwasm_std::Storage,
    after_timestamp: u64,
) -> ContractResult<Option<u64>> {
    let next = ER_HISTORY
        .range(
            storage,
            Some(Bound::exclusive(after_timestamp)),
            None,
            Order::Ascending,
        )
        .next()
        .transpose()?
        .map(|(timestamp, _)| timestamp);

    Ok(next)
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
    let aum = get_aum_from_oracles(deps)?;
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
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
