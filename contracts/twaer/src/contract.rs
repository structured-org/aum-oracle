use std::ops::Sub;

use crate::error::{ContractError, ContractResult};
use crate::msg::{
    ExecuteMsg, GetAumResponse, GetTwaerResponse, InstantiateMsg, QueryMsg, UpdateConfig,
};
use crate::state::{Config, TwaAggregator, CONFIG, ER_HISTORY, TWAER, TWA_AGGREGATOR};
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdResult, Uint128,
};
use cw_storage_plus::Bound;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let oracles: Vec<Addr> = msg
        .aum_oracles
        .iter()
        .map(|addr| deps.api.addr_validate(addr))
        .collect::<StdResult<_>>()?;

    let contract_config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        aum_oracles: oracles,
        maxbtc_denom: msg.maxbtc_denom,
        twa_window_seconds: msg.twa_window_seconds,
    };
    CONFIG.save(deps.storage, &contract_config)?;

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
            Ok(execute_update_config(deps, env, info, new_config)?)
        }
        ExecuteMsg::RecordEr {} => Ok(execute_record_er(deps, env, info)?),
        ExecuteMsg::PublishTwaer {} => Ok(execute_publish_twaer(deps, env, info)?),
    }
}

fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_config: UpdateConfig,
) -> ContractResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Update configuration if fields are provided
    if let Some(ref oracles) = new_config.aum_oracles {
        let validated_oracles: Vec<Addr> = oracles
            .iter()
            .map(|addr| deps.api.addr_validate(addr))
            .collect::<StdResult<_>>()?;
        config.aum_oracles = validated_oracles;
    }
    if let Some(new_owner) = new_config.owner {
        let validated_new_owner = deps.api.addr_validate(&new_owner)?;
        config.owner = validated_new_owner;
    }
    if let Some(maxbtc_denom) = new_config.maxbtc_denom {
        config.maxbtc_denom = maxbtc_denom;
    }
    if let Some(twa_window_seconds) = new_config.twa_window_seconds {
        config.twa_window_seconds = twa_window_seconds;
    }

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_config"))
}

fn execute_record_er(deps: DepsMut, env: Env, info: MessageInfo) -> ContractResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    let exchange_rate = calc_exchange_rate(deps.as_ref())?;
    let timestamp = env.block.time.seconds();
    ER_HISTORY.save(deps.storage, timestamp, &exchange_rate)?;

    let window_start = timestamp.sub(config.twa_window_seconds);
    let expired_rates = determine_expired_rates(deps.storage, window_start)?;
    update_twa_aggregator(deps.storage, exchange_rate, timestamp, &expired_rates)?;

    Ok(Response::new().add_attributes([
        ("action", "record_er"),
        ("exchange_rate", &exchange_rate.to_string()),
    ]))
}

fn execute_publish_twaer(deps: DepsMut, env: Env, info: MessageInfo) -> ContractResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    let twaer = calculate_twaer(deps.as_ref(), env.clone())?;
    TWAER.save(deps.storage, &(twaer, env.block.time.seconds()))?;

    Ok(
        Response::new()
            .add_attributes([("action", "publish_twaer"), ("twaer", &twaer.to_string())]),
    )
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

/// Updates the TWA aggregator with a new exchange rate data point and manages the time window
/// by removing expired entries. This function performs incremental TWA calculation by:
/// 1. Adding the weighted contribution of the most recent rate for the time period since
///    the last update;
/// 2. Removing weighted contributions from exchange rates that have expired outside the
///    configured TWA window;
/// 3. Recalculating the current TWA based on the updated weighted sum and total duration;
/// 4. Updating aggregator metadata (window boundaries, current TWA).
fn update_twa_aggregator(
    storage: &mut dyn cosmwasm_std::Storage,
    new_rate: Decimal,
    new_timestamp: u64,
    expired_rates: &[(u64, Decimal)],
) -> ContractResult<()> {
    let mut twa_aggr = match TWA_AGGREGATOR.may_load(storage)? {
        None => {
            TWA_AGGREGATOR.save(
                storage,
                &TwaAggregator::from_single_point(new_timestamp, new_rate),
            )?;
            return Ok(());
        }
        Some(buffer) => buffer,
    };

    // the rate that was active from window_end until now
    let latest_rate_duration = new_timestamp - twa_aggr.window_end;
    if latest_rate_duration <= 0 {
        return Err(ContractError::DuplicateDataPoint {
            timestamp: new_timestamp,
        });
    }
    let latest_rate = ER_HISTORY
        .range(
            storage,
            None,
            Some(Bound::inclusive(twa_aggr.window_end)),
            Order::Descending,
        )
        .next()
        .transpose()?
        .map(|(_, rate)| rate)
        .unwrap_or(twa_aggr.current_twa);

    let weighted_contribution =
        latest_rate.checked_mul(Decimal::from_ratio(latest_rate_duration, 1u64))?;
    twa_aggr.weighted_sum = twa_aggr.weighted_sum.checked_add(weighted_contribution)?;
    twa_aggr.total_duration += latest_rate_duration;

    // Remove contributions from rates that are no longer in the window
    for (expired_timestamp, expired_rate) in expired_rates {
        let next_timestamp = find_next_timestamp_after(storage, *expired_timestamp)?.ok_or(
            ContractError::NoNextTimestamp {
                timestamp: *expired_timestamp,
            },
        )?;
        // how long the rate was active
        let expired_duration = next_timestamp - expired_timestamp;
        // contribution of the rate
        let expired_contribution =
            expired_rate.checked_mul(Decimal::from_ratio(expired_duration, 1u64))?;

        twa_aggr.weighted_sum = twa_aggr.weighted_sum.checked_sub(expired_contribution)?;
        twa_aggr.total_duration -= expired_duration;

        ER_HISTORY.remove(storage, *expired_timestamp);
    }

    twa_aggr.window_end = new_timestamp;
    if twa_aggr.total_duration > 0 {
        twa_aggr.current_twa = twa_aggr
            .weighted_sum
            .checked_div(Decimal::from_ratio(twa_aggr.total_duration, 1u64))?;
    } else {
        twa_aggr.current_twa = new_rate;
    }

    let oldest_timestamp = ER_HISTORY
        .range(storage, None, None, Order::Ascending)
        .next()
        .transpose()?
        .map(|(timestamp, _)| timestamp)
        .unwrap_or(new_timestamp);
    twa_aggr.window_start = oldest_timestamp;

    TWA_AGGREGATOR.save(storage, &twa_aggr)?;
    Ok(())
}

fn calc_exchange_rate(deps: Deps) -> ContractResult<Decimal> {
    let config = CONFIG.load(deps.storage)?;
    let maxbtc_supply = deps.querier.query_supply(config.maxbtc_denom)?.amount;
    let aum = get_aum(deps)?;

    Ok(Decimal::from_ratio(aum, maxbtc_supply))
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
    }
}

fn query_get_aum(deps: Deps) -> ContractResult<GetAumResponse> {
    let aum = get_aum(deps)?;
    Ok(GetAumResponse { aum_in_btc: aum })
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

fn get_aum(deps: Deps) -> ContractResult<Uint128> {
    let config = CONFIG.load(deps.storage)?;
    let mut total_aum = Uint128::zero();
    for oracle in config.aum_oracles.iter() {
        let aum: GetAumResponse = deps.querier.query_wasm_smart(
            oracle,
            &serde_json::json!({
              "get_aum": {}
            }),
        )?;
        total_aum += aum.aum_in_btc;
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
        .map(|(_, rate)| rate)
        .unwrap_or(twa_buffer.current_twa);

    // Calculate the additional contribution from the last rate
    let additional_duration = env.block.time.seconds() - twa_buffer.window_end;
    let additional_contribution =
        last_rate.checked_mul(Decimal::from_ratio(additional_duration, 1u64))?;

    // Calculate total weighted sum and duration
    let total_weighted_sum = twa_buffer
        .weighted_sum
        .checked_add(additional_contribution)?;
    let total_duration = twa_buffer.total_duration + additional_duration;

    if total_duration == 0 {
        return Ok(last_rate);
    }

    // Calculate final TWA
    total_weighted_sum
        .checked_div(Decimal::from_ratio(total_duration, 1u64))
        .map_err(|e| ContractError::CheckedDiv(e))
}
