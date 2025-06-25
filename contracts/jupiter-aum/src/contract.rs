use crate::state::{CONFIG, CONSENSUS_STATE};
use consensus::consensus::OracleData;
use consensus::consensus::{Config as ConsensusConfig, ConsensusResult};
use cosmwasm_std::{
    attr, entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, Int128, MessageInfo,
    Response, SignedDecimal,
};
use cw2::set_contract_version;
use jupiter_aum_common::error::ContractError;
use jupiter_aum_common::msg::{
    ConfigResponse, ExecuteMsg, GetAUMResponse, GetDataResponse, InstantiateMsg, MigrateMsg,
    QueryMsg,
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

const DECIMAL_MULTIPLIER: i128 = 1_000_000; // 6 points

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
        ExecuteMsg::UpdateConfig {
            admin,
            valid_period,
        } => update_config(deps, info, admin, valid_period),
        // TODO: ExecuteMsg::UpdateConsensusConfig {}
        ExecuteMsg::PublishData { data } => publish_data(deps, env, info, data),
    }
}

/// Updates configuration parameters for the contract.
/// Only admin can call this method.
#[allow(clippy::too_many_arguments)]
fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    admin: Option<String>,
    valid_period: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // ensure only the contract admin can update the configuration
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(new_admin) = admin {
        config.admin = deps.api.addr_validate(&new_admin)?;
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
/// If consensus is reached, the `LAST_PUBLISHED_DATA` is updated
/// and old pending slots are removed.
fn publish_data(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_data: OracleData<SolanaData>,
) -> Result<Response, ContractError> {
    let config = CONSENSUS_STATE.config.load(deps.storage)?;

    // permission check: only registered oracles can publish data
    if !config.oracles.contains(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }
    let (result, pending_round) =
        CONSENSUS_STATE.publish_data(deps.storage, &env, info.sender, new_data)?;

    let mut res = Response::new();

    // If we have new published data for the current round, consensus was reached
    if let ConsensusResult::ConsensusReached(_) = result {
        res = res.add_attribute("action", "publish_consensus");
    }

    res = res.add_attributes([
        attr(
            "next_round",
            pending_round
                .next_round(config.round_length)
                .round
                .to_string(),
        ),
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
        QueryMsg::GetAUM {} => Ok(to_json_binary(&query_get_aum(deps, env)?)?),
        // TODO: QueryMsg::Round {} => { pending_round(), if voted -> next_round() }
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
fn query_get_aum(deps: Deps, env: Env) -> Result<GetAUMResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let data = CONSENSUS_STATE
        .get_last_published_data(&env, deps.storage)?
        .ok_or(ContractError::NoDataPublished {})?
        .data;

    if env.block.time.seconds() > data.timestamp.seconds() + config.valid_period {
        return Err(ContractError::DataNotValid {});
    }

    let btc_price_in_usd = query_btc_price_in_usd(deps)?;
    let aum_in_btc = calculate_aum_in_btc(data, btc_price_in_usd)?;

    Ok(GetAUMResponse { aum_in_btc })
}

fn query_btc_price_in_usd(deps: Deps) -> Result<SignedDecimal, ContractError> {
    let querier = OracleQuerier::new(&deps.querier);
    let btc_usd_price_result = querier.get_price(Some(CurrencyPair {
        base: BTC_DENOM.to_string(),
        quote: USD_DENOM.to_string(),
    }))?;
    let btc_usd_price_string = btc_usd_price_result
        .price
        .ok_or(ContractError::SlinkyBTCPriceMissing {})?
        .price;
    let btc_price_in_usd = Int128::from_str(&btc_usd_price_string).map_err(|e| {
        ContractError::SlinkyBTCPriceIncorrect {
            price: btc_usd_price_string,
            error: e.to_string(),
        }
    })?;

    let btc_price_in_usd =
        SignedDecimal::from_atomics(btc_price_in_usd, btc_usd_price_result.decimals as u32)
            .map_err(|e| ContractError::DecimalError {
                error: e.to_string(),
            })?;
    Ok(btc_price_in_usd)
}

pub fn calculate_aum_in_btc(
    data: SolanaData,
    btc_price_in_usd: SignedDecimal,
) -> Result<Int128, ContractError> {
    let jlp_virtual_price = data
        .aum_usd
        .checked_div(data.total_jlp_supply)
        .map_err(|e| ContractError::DecimalError {
            error: e.to_string(),
        })?;
    let jlp_balance_in_usd = jlp_virtual_price
        .checked_mul(data.strategy_jlp_balance)
        .map_err(|e| ContractError::DecimalError {
            error: e.to_string(),
        })?;
    let aum_in_btc = jlp_balance_in_usd
        .checked_div(btc_price_in_usd)
        .map_err(|e| ContractError::DecimalError {
            error: e.to_string(),
        })?;
    // convert to multiplier to make it integer with decimal places
    let multiplier = SignedDecimal::from_atomics(DECIMAL_MULTIPLIER, 0).map_err(|e| {
        ContractError::DecimalError {
            error: e.to_string(),
        }
    })?;
    let result = (aum_in_btc * multiplier).to_int_floor();
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
