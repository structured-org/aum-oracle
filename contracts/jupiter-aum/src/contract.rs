use crate::state::{CONFIG, CONSENSUS_STATE};
use consensus::consensus::Config as ConsensusConfig;
use consensus::consensus::PublishResult;
use cosmwasm_std::{
    attr, entry_point, to_json_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use jupiter_aum_common::error::ContractError;
use jupiter_aum_common::msg::{
    ConfigResponse, ExecuteMsg, GetAumResponse, GetDataResponse, InstantiateMsg, MigrateMsg,
    QueryMsg, RoundInfoResponse, UpdateConfig,
};
use jupiter_aum_common::types::{Config, SolanaData};
use neutron_std::types::slinky::oracle::v1::OracleQuerier;
use neutron_std::types::slinky::types::v1::CurrencyPair;
use std::str::FromStr;

const CONTRACT_NAME: &str = "crates.io:jupiter-aum";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// BTC/USD oracle query constants
const BTC_DENOM: &str = "BTC";
const USD_DENOM: &str = "USD";

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        admin: deps.api.addr_validate(&msg.admin)?,
        valid_period: msg.valid_period,
        required_custody_assets: msg.required_custody_assets,
        price_max_blocks_old: msg.price_max_blocks_old,
    };
    config.validate()?;
    CONFIG.save(deps.storage, &config)?;

    let consensus_config = ConsensusConfig {
        oracles: msg
            .oracles
            .into_iter()
            .map(|addr| deps.api.addr_validate(&addr))
            .collect::<Result<Vec<Addr>, _>>()?,
        threshold: msg.threshold,
        data_delta_ppm: msg.data_delta_ppm,
        round_length: msg.round_length,
    };
    CONSENSUS_STATE.initialize(deps.storage, &env, consensus_config)?;

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
        ExecuteMsg::UpdateConfig { new_config } => update_config(deps, info, new_config),
        ExecuteMsg::PublishData { new_data } => execute_publish_data(deps, env, info, new_data),
    }
}

/// Updates configuration parameters for the contract.
/// Only admin can call this method.
#[allow(clippy::too_many_arguments)]
fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_config: UpdateConfig,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // ensure only the contract admin can update the configuration
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(new_admin) = new_config.admin {
        config.admin = deps.api.addr_validate(&new_admin)?;
    }

    if let Some(new_valid_period) = new_config.valid_period {
        config.valid_period = new_valid_period;
    }

    if let Some(new_required_custody_assets) = new_config.required_custody_assets {
        config.required_custody_assets = new_required_custody_assets;
    }

    if let Some(new_price_max_blocks_old) = new_config.price_max_blocks_old {
        config.price_max_blocks_old = new_price_max_blocks_old;
    }

    config.validate()?;
    CONFIG.save(deps.storage, &config)?;

    let mut consensus_config = CONSENSUS_STATE.config.load(deps.storage)?;

    // Update consensus config fields
    if let Some(ref oracles) = new_config.oracles {
        let validated_oracles: Vec<Addr> = oracles
            .iter()
            .map(|addr| deps.api.addr_validate(addr))
            .collect::<StdResult<_>>()?;
        consensus_config.oracles = validated_oracles;
    }
    if let Some(threshold) = new_config.threshold {
        consensus_config.threshold = threshold;
    }
    if let Some(data_delta_ppm) = new_config.data_delta_ppm {
        consensus_config.data_delta_ppm = data_delta_ppm;
    }
    if let Some(round_length) = new_config.round_length {
        consensus_config.round_length = round_length;
    }

    // Save updated consensus config
    CONSENSUS_STATE.update_config(deps.storage, consensus_config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

/// Allows an oracle to publish Solana data.
/// Only registered oracles can call this.
/// Oracle can only publish once per slot.
/// If consensus is reached, the `LAST_PUBLISHED_DATA` is updated
/// and old pending slots are removed.
fn execute_publish_data(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut new_data: SolanaData,
) -> Result<Response, ContractError> {
    let contract_config = CONFIG.load(deps.storage)?;
    let consensus_config = CONSENSUS_STATE.config.load(deps.storage)?;

    // permission check: only registered oracles can publish data
    if !consensus_config.oracles.contains(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    new_data.clean_and_validate(contract_config.required_custody_assets)?;

    let (result, pending_round) =
        CONSENSUS_STATE.publish_data(deps.storage, &env, info.sender, new_data)?;

    let mut res = Response::new().add_attribute("action", "publish_consensus");

    // If we have new published data for the current round, consensus was reached
    if let PublishResult::ConsensusReached(_) = result {
        res = res.add_attribute("consensus_reached", "true");
    } else {
        res = res.add_attribute("consensus_reached", "false");
    }

    let next_round = pending_round.next_round(consensus_config.round_length);
    res = res.add_attributes([
        attr("next_round", next_round.round.to_string()),
        attr("next_round_timestamp", next_round.start.to_string()),
        attr("round", pending_round.round.to_string()),
    ]);

    Ok(res)
}

// ----------------------------------------
//  Queries
// ----------------------------------------
#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps)?)?),
        QueryMsg::GetData {} => Ok(to_json_binary(&query_get_data(deps, env)?)?),
        QueryMsg::GetAum {} => Ok(to_json_binary(&query_get_aum(deps, env)?)?),
        QueryMsg::GetRoundInfo {} => Ok(to_json_binary(&query_round_info(deps, env)?)?),
    }
}

/// Returns the current contract configuration.
fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        admin: config.admin.to_string(),
        valid_period: config.valid_period,
    })
}

/// Returns the last successfully published and finalized Solana data.
fn query_get_data(deps: Deps, env: Env) -> Result<GetDataResponse, ContractError> {
    Ok(GetDataResponse {
        last_published_data: CONSENSUS_STATE.get_last_published_data(&env, deps.storage)?,
    })
}

/// Returns Jupiter AUM value represented in BTC.
fn query_get_aum(deps: Deps, env: Env) -> Result<GetAumResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let published_state = CONSENSUS_STATE
        .get_last_published_data(&env, deps.storage)?
        .ok_or(ContractError::NoDataPublished {})?;
    if env.block.time.seconds() > published_state.timestamp + config.valid_period {
        return Err(ContractError::DataNotValid {});
    }

    let btc_price_in_usd = query_btc_price_in_usd(deps, env, config)?;
    let aum_in_btc = calculate_aum_in_btc(published_state.data, btc_price_in_usd)?;

    Ok(GetAumResponse { aum_in_btc })
}

fn query_round_info(deps: Deps, _env: Env) -> Result<RoundInfoResponse, ContractError> {
    let pending_round = CONSENSUS_STATE.get_pending_round(deps.storage)?;

    Ok(RoundInfoResponse {
        pending_round,
        next_round: pending_round
            .next_round(CONSENSUS_STATE.config.load(deps.storage)?.round_length),
    })
}

fn query_btc_price_in_usd(deps: Deps, env: Env, config: Config) -> Result<Decimal, ContractError> {
    let querier = OracleQuerier::new(&deps.querier);
    let response = querier.get_price(Some(CurrencyPair {
        base: BTC_DENOM.to_string(),
        quote: USD_DENOM.to_string(),
    }))?;
    let quote = response
        .price
        .ok_or(ContractError::SlinkyBTCPriceMissing {})?;

    if quote.block_height + config.price_max_blocks_old < env.block.height {
        return Err(ContractError::SlinkyBTCPriceTooOld {
            price_height: quote.block_height,
        });
    }

    let btc_price_in_usd =
        Uint128::from_str(&quote.price).map_err(|e| ContractError::SlinkyBTCPriceIncorrect {
            price: quote.price,
            error: e.to_string(),
        })?;

    let btc_price_in_usd = Decimal::from_atomics(btc_price_in_usd, response.decimals as u32)
        .map_err(|e| ContractError::DecimalError {
            error: e.to_string(),
        })?;
    Ok(btc_price_in_usd)
}

pub fn calculate_aum_in_btc(
    data: SolanaData,
    btc_price_in_usd: Decimal,
) -> Result<Uint128, ContractError> {
    let aum_usd =
        Decimal::from_atomics(data.aum_usd, data.jlp_token_decimals as u32).map_err(|e| {
            ContractError::DecimalError {
                error: e.to_string(),
            }
        })?;
    let total_jlp_supply =
        Decimal::from_atomics(data.total_jlp_supply, data.jlp_token_decimals as u32).map_err(
            |e| ContractError::DecimalError {
                error: e.to_string(),
            },
        )?;
    let strategy_jlp_balance =
        Decimal::from_atomics(data.strategy_jlp_balance, data.jlp_token_decimals as u32).map_err(
            |e| ContractError::DecimalError {
                error: e.to_string(),
            },
        )?;
    let jlp_virtual_price =
        aum_usd
            .checked_div(total_jlp_supply)
            .map_err(|e| ContractError::DecimalError {
                error: e.to_string(),
            })?;
    let jlp_balance_in_usd = jlp_virtual_price
        .checked_mul(strategy_jlp_balance)
        .map_err(|e| ContractError::DecimalError {
            error: e.to_string(),
        })?;
    let aum_in_btc = jlp_balance_in_usd
        .checked_div(btc_price_in_usd)
        .map_err(|e| ContractError::DecimalError {
            error: e.to_string(),
        })?;
    // convert to multiplier to make it integer with decimal places
    let multiplier = Decimal::pow(
        Decimal::from_atomics(Uint128::new(10), 0).map_err(|e| ContractError::DecimalError {
            error: e.to_string(),
        })?,
        data.jlp_token_decimals as u32,
    );
    let result = (aum_in_btc * multiplier).to_uint_floor();
    Ok(result)
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
