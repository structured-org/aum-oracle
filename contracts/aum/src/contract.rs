use crate::error::{ContractError, ContractResult};
use crate::msg::{
    ExecuteMsg, GetAumResponse, GetHistoricalDataResponse, InstantiateMsg, QueryMsg, UpdateConfig,
};
use crate::state::{
    Config, ExchangeRateDataPoint, CONFIG, EXCHANGE_RATE_HISTORY, TWA_EXCHANGE_RATE,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdResult, Uint128,
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let oracles: Vec<Addr> = msg
        .oracles
        .iter()
        .map(|addr| deps.api.addr_validate(addr))
        .collect::<StdResult<_>>()?;

    let contract_config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        oracles,
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
        ExecuteMsg::StoreInstantExchangeRate {} => {
            Ok(execute_store_instant_exchange_rate(deps, env, info)?)
        }
        ExecuteMsg::UpdateTwaExchangeRate {} => {
            Ok(execute_update_twa_exchange_rate(deps, env, info)?)
        }
    }
}

fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_config: UpdateConfig,
) -> ContractResult<Response> {
    // Load current contract config
    let mut contract_config = CONFIG.load(deps.storage)?;

    // Only admin can update config
    if info.sender != contract_config.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Update configuration if fields are provided
    if let Some(ref oracles) = new_config.oracles {
        let validated_oracles: Vec<Addr> = oracles
            .iter()
            .map(|addr| deps.api.addr_validate(addr))
            .collect::<StdResult<_>>()?;
        contract_config.oracles = validated_oracles;
    }

    if let Some(new_owner) = new_config.owner {
        let validated_new_owner = deps.api.addr_validate(&new_owner)?;
        contract_config.owner = validated_new_owner;
    }

    if let Some(twa_window_seconds) = new_config.twa_window_seconds {
        contract_config.twa_window_seconds = twa_window_seconds;
    }

    CONFIG.save(deps.storage, &contract_config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

fn execute_store_instant_exchange_rate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> ContractResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    // Only admin can order to store instant exchange rate
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    let exchange_rate = calc_exchange_rate(deps.as_ref())?;
    let timestamp = env.block.time.seconds();

    // Store the instant exchange rate data point
    let data_point = ExchangeRateDataPoint {
        rate: exchange_rate,
        timestamp,
        block_height: env.block.height,
    };

    EXCHANGE_RATE_HISTORY.save(deps.storage, timestamp, &data_point)?;

    // Clean up old data points outside the TWA window
    cleanup_old_data_points(deps.storage, timestamp, config.twa_window_seconds)?;

    Ok(Response::new().add_attribute("action", "store_instant_exchange_rate"))
}

fn execute_update_twa_exchange_rate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> ContractResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    // Only admin can order to update TWA exchange rate
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    let twa_rate = calculate_twa_exchange_rate(deps.as_ref(), &env, &config)?;
    TWA_EXCHANGE_RATE.save(deps.storage, &twa_rate)?;

    Ok(Response::new().add_attribute("action", "update_twa_exchange_rate"))
}

/// Cleans up exchange rate data points that are older than the TWA window
fn cleanup_old_data_points(
    storage: &mut dyn cosmwasm_std::Storage,
    current_timestamp: u64,
    twa_window_seconds: u64,
) -> ContractResult<()> {
    let window_start = current_timestamp.saturating_sub(twa_window_seconds);

    // Collect timestamps of old data points to remove
    let old_timestamps: Vec<u64> = EXCHANGE_RATE_HISTORY
        .range(storage, None, None, Order::Ascending)
        .filter_map(|item| match item {
            Ok((timestamp, _)) if timestamp < window_start => Some(timestamp),
            _ => None,
        })
        .collect();

    // Remove old data points
    for timestamp in old_timestamps {
        EXCHANGE_RATE_HISTORY.remove(storage, timestamp);
    }

    Ok(())
}

/// Calculates the Time-Weighted Average exchange rate based on historical data points
fn calculate_twa_exchange_rate(deps: Deps, env: &Env, config: &Config) -> ContractResult<Decimal> {
    let current_timestamp = env.block.time.seconds();
    let window_start = current_timestamp.saturating_sub(config.twa_window_seconds);

    // Get all data points within the TWA window
    let mut data_points: Vec<ExchangeRateDataPoint> = EXCHANGE_RATE_HISTORY
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|item| match item {
            Ok((timestamp, data_point)) if timestamp >= window_start => Some(data_point),
            _ => None,
        })
        .collect();

    // If no historical data, return current exchange rate
    if data_points.is_empty() {
        return calc_exchange_rate(deps);
    }

    // Sort by timestamp (should already be sorted from the range query, but ensure it)
    data_points.sort_by_key(|p| p.timestamp);

    let mut weighted_sum = Decimal::zero();
    let mut total_weight = 0u64;

    for (i, data_point) in data_points.iter().enumerate() {
        // Calculate the time weight for this data point
        let next_timestamp = if i + 1 < data_points.len() {
            data_points[i + 1].timestamp
        } else {
            current_timestamp
        };

        let weight = next_timestamp - data_point.timestamp;

        // Add to weighted sum
        weighted_sum = weighted_sum.checked_add(
            data_point
                .rate
                .checked_mul(Decimal::from_ratio(weight, 1u64))?,
        )?;
        total_weight += weight;
    }

    // Calculate the time-weighted average
    if total_weight == 0 {
        // If total weight is zero, return the most recent rate
        return Ok(data_points.last().unwrap().rate);
    }

    let twa_rate = weighted_sum.checked_div(Decimal::from_ratio(total_weight, 1u64))?;
    Ok(twa_rate)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::GetAum {} => Ok(to_json_binary(&query_get_aum(deps)?)?),
        QueryMsg::GetConfig {} => Ok(to_json_binary(&CONFIG.load(deps.storage)?)?),
        QueryMsg::GetTwaExchangeRate {} => Ok(to_json_binary(&query_get_twa_exchange_rate(deps)?)?),
        QueryMsg::PredictTwaExchangeRate {} => {
            Ok(to_json_binary(&predict_twa_exchange_rate(deps, env)?)?)
        }
        QueryMsg::GetHistoricalData { limit } => Ok(to_json_binary(&query_get_historical_data(
            deps, env, limit,
        )?)?),
        QueryMsg::GetDataPointCount {} => {
            Ok(to_json_binary(&query_get_data_point_count(deps, env)?)?)
        }
    }
}

pub fn query_get_aum(deps: Deps) -> ContractResult<GetAumResponse> {
    let aum = get_aum(deps)?;
    Ok(GetAumResponse { aum_in_btc: aum })
}

fn query_get_twa_exchange_rate(deps: Deps) -> ContractResult<Decimal> {
    TWA_EXCHANGE_RATE.load(deps.storage).map_err(|_| {
        ContractError::Std(cosmwasm_std::StdError::generic_err(
            "TWA exchange rate not yet calculated. Call UpdateTwaExchangeRate first.",
        ))
    })
}

fn predict_twa_exchange_rate(deps: Deps, env: Env) -> ContractResult<Decimal> {
    let config = CONFIG.load(deps.storage)?;
    calculate_twa_exchange_rate(deps, &env, &config)
}

fn calc_exchange_rate(deps: Deps) -> ContractResult<Decimal> {
    let config = CONFIG.load(deps.storage)?;
    let supply = deps.querier.query_supply(config.maxbtc_denom)?;
    let aum = get_aum(deps)?;

    Ok(Decimal::from_ratio(aum, supply.amount))
}

fn get_aum(deps: Deps) -> ContractResult<Uint128> {
    let config = CONFIG.load(deps.storage)?;

    let mut total_aum = Uint128::zero();

    for oracle in config.oracles.iter() {
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

fn query_get_historical_data(
    deps: Deps,
    env: Env,
    limit: Option<u32>,
) -> ContractResult<GetHistoricalDataResponse> {
    let config = CONFIG.load(deps.storage)?;
    let current_timestamp = env.block.time.seconds();
    let window_start = current_timestamp.saturating_sub(config.twa_window_seconds);
    let limit = limit.unwrap_or(100);

    // Get all data points within the TWA window
    let mut data_points: Vec<ExchangeRateDataPoint> = EXCHANGE_RATE_HISTORY
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|item| match item {
            Ok((timestamp, data_point)) if timestamp >= window_start => Some(data_point),
            _ => None,
        })
        .collect();

    let total_count = data_points.len() as u32;

    // Apply limit if specified
    if data_points.len() > limit as usize {
        data_points = data_points
            .into_iter()
            .rev()
            .take(limit as usize)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
    }

    Ok(GetHistoricalDataResponse {
        data_points,
        total_count,
    })
}

fn query_get_data_point_count(deps: Deps, env: Env) -> ContractResult<u32> {
    let config = CONFIG.load(deps.storage)?;
    let current_timestamp = env.block.time.seconds();
    let window_start = current_timestamp.saturating_sub(config.twa_window_seconds);

    // Count data points within the TWA window
    let count = EXCHANGE_RATE_HISTORY
        .range(deps.storage, None, None, Order::Ascending)
        .filter(|item| match item {
            Ok((timestamp, _)) if *timestamp >= window_start => true,
            _ => false,
        })
        .count() as u32;

    Ok(count)
}
