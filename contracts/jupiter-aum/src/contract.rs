use crate::state::{CONFIG, CONSENSUS_STATE};
use consensus::consensus::Config as ConsensusConfig;
use consensus::consensus::PublishResult;
use cosmwasm_std::{
    attr, entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, Int256, MessageInfo,
    Response, SignedDecimal256, StdResult, Uint128,
};
use cw2::set_contract_version;
use jupiter_aum_common::constants::WBTC_DECIMALS;
use jupiter_aum_common::error::ContractError;
use jupiter_aum_common::msg::{
    AumResponse, ConfigResponse, ExecuteMsg, GetDataResponse, InstantiateMsg, MigrateMsg, QueryMsg,
    RoundInfoResponse, UpdateConfig,
};
use jupiter_aum_common::types::{Config, SolanaData};
use neutron_std::types::slinky::oracle::v1::OracleQuerier;
use neutron_std::types::slinky::types::v1::CurrencyPair;
use std::str::FromStr;

const CONTRACT_NAME: &str = "crates.io:jupiter-aum";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const BTC_DENOM: &str = "BTC";
const USD_DENOM: &str = "USD";

/// Instantiates the contract with initial configuration and oracle consensus settings.
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        consensus_data_validity_period: msg.consensus_data_validity_period,
        required_custody_assets: msg.required_custody_assets,
        price_data_validity_period: msg.price_data_validity_period,
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
        .add_attribute("owner", config.owner.to_string()))
}

/// Entry point for executing contract messages.
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

/// Updates both general and consensus-related contract configuration.
/// Only callable by the current contract owner.
fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_config: UpdateConfig,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(new_owner) = new_config.owner {
        config.owner = deps.api.addr_validate(&new_owner)?;
    }

    if let Some(new_consensus_data_validity_period) = new_config.consensus_data_validity_period {
        config.consensus_data_validity_period = new_consensus_data_validity_period;
    }

    if let Some(new_required_custody_assets) = new_config.required_custody_assets {
        config.required_custody_assets = new_required_custody_assets;
    }

    if let Some(new_price_data_validity_period) = new_config.price_data_validity_period {
        config.price_data_validity_period = new_price_data_validity_period;
    }

    config.validate()?;
    CONFIG.save(deps.storage, &config)?;

    let mut consensus_config = CONSENSUS_STATE.config.load(deps.storage)?;

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

    CONSENSUS_STATE.update_config(deps.storage, consensus_config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

/// Allows a registered oracle to publish Solana data for the current round.
/// If consensus is reached, finalizes the data and returns next round info in events.
fn execute_publish_data(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut new_data: SolanaData,
) -> Result<Response, ContractError> {
    let contract_config = CONFIG.load(deps.storage)?;
    let consensus_config = CONSENSUS_STATE.config.load(deps.storage)?;

    if !consensus_config.oracles.contains(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    new_data.clean_and_validate(contract_config.required_custody_assets)?;

    let (result, pending_round) =
        CONSENSUS_STATE.publish_data(deps.storage, &env, info.sender, new_data)?;

    let mut res = Response::new().add_attribute("action", "publish_consensus");

    match result {
        PublishResult::ConsensusReached(_) => {
            res = res.add_attribute("consensus_reached", "true");
        }
        PublishResult::ConsensusNotReached => {
            res = res.add_attribute("consensus_reached", "false");
        }
    }

    let next_round = pending_round.next_round(consensus_config.round_length);
    res = res.add_attributes([
        attr("next_round", next_round.round.to_string()),
        attr("next_round_timestamp", next_round.start.to_string()),
        attr("round", pending_round.round.to_string()),
    ]);

    Ok(res)
}

/// Dispatches query messages to appropriate query handlers.
#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps)?)?),
        QueryMsg::GetData {} => Ok(to_json_binary(&query_get_data(deps, env)?)?),
        QueryMsg::GetAum {} => Ok(to_json_binary(&query_get_aum(deps, env)?)?),
        QueryMsg::GetRoundInfo {} => Ok(to_json_binary(&query_round_info(deps, env)?)?),
    }
}

/// Returns combined general and consensus configuration of the contract.
fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let consensus_config = CONSENSUS_STATE.config.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: config.owner.to_string(),
        consensus_data_validity_period: config.consensus_data_validity_period,
        required_custody_assets: config.required_custody_assets,
        price_data_validity_period: config.price_data_validity_period,
        oracles: consensus_config.oracles,
        threshold: consensus_config.threshold,
        data_delta_ppm: consensus_config.data_delta_ppm,
        round_length: consensus_config.round_length,
    })
}

/// Returns the last finalized Solana data.
fn query_get_data(deps: Deps, env: Env) -> Result<GetDataResponse, ContractError> {
    Ok(GetDataResponse {
        last_published_data: CONSENSUS_STATE.get_last_published_data(&env, deps.storage)?,
    })
}

/// Calculates and returns the current AUM value in wBTC.
fn query_get_aum(deps: Deps, env: Env) -> Result<AumResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let published_state = CONSENSUS_STATE
        .get_last_published_data(&env, deps.storage)?
        .ok_or(ContractError::NoDataPublished {})?;

    if env.block.time.seconds() > published_state.timestamp + config.consensus_data_validity_period
    {
        return Err(ContractError::PublishedDataTooOld {});
    }

    let btc_price_in_usd = query_btc_price_in_usd(deps, env, &config)?;
    let aum_in_btc = calculate_aum_in_btc(published_state.data, btc_price_in_usd)?;

    Ok(AumResponse { aum_in_btc })
}

/// Returns pending and next round info.
fn query_round_info(deps: Deps, _env: Env) -> Result<RoundInfoResponse, ContractError> {
    let pending_round = CONSENSUS_STATE.get_pending_round(deps.storage)?;

    Ok(RoundInfoResponse {
        pending_round,
        next_round: pending_round
            .next_round(CONSENSUS_STATE.config.load(deps.storage)?.round_length),
    })
}

/// Fetches the BTC/USD price from the oracle and ensures freshness.
fn query_btc_price_in_usd(
    deps: Deps,
    env: Env,
    config: &Config,
) -> Result<SignedDecimal256, ContractError> {
    let querier = OracleQuerier::new(&deps.querier);
    let response = querier.get_price(Some(CurrencyPair {
        base: BTC_DENOM.to_string(),
        quote: USD_DENOM.to_string(),
    }))?;

    let quote = response
        .price
        .ok_or(ContractError::SlinkyBTCPriceMissing {})?;

    if quote.block_height + config.price_data_validity_period < env.block.height {
        return Err(ContractError::SlinkyBTCPriceTooOld {
            price_height: quote.block_height,
        });
    }

    let btc_price_in_usd =
        Uint128::from_str(&quote.price).map_err(|e| ContractError::SlinkyBTCPriceIncorrect {
            price: quote.price,
            error: e.to_string(),
        })?;

    let btc_price_in_usd =
        SignedDecimal256::from_atomics(btc_price_in_usd, response.decimals as u32).map_err(
            |e| ContractError::DecimalError {
                error: e.to_string(),
            },
        )?;

    Ok(btc_price_in_usd)
}

/// Computes the AUM in wBTC units using Solana data and BTC/USD price.
pub fn calculate_aum_in_btc(
    data: SolanaData,
    btc_price_in_usd: SignedDecimal256,
) -> Result<Int256, ContractError> {
    let aum_usd = SignedDecimal256::from_atomics(data.aum_usd, 0)?;
    let total_jlp_supply = SignedDecimal256::from_atomics(
        data.total_jlp_supply,
        data.total_jlp_supply_decimals as u32,
    )?;
    let strategy_jlp_balance = SignedDecimal256::from_atomics(
        data.strategy_jlp_balance,
        data.strategy_jlp_balance_decimals as u32,
    )?;
    let jlp_virtual_price = aum_usd.checked_div(total_jlp_supply)?;
    let jlp_balance_in_usd = jlp_virtual_price.checked_mul(strategy_jlp_balance)?;
    let aum_in_btc = jlp_balance_in_usd.checked_div(btc_price_in_usd)?;
    let aum_in_wbtc = aum_in_btc.atomics()
        / Int256::from_i128(10i128.pow(aum_in_btc.decimal_places() - WBTC_DECIMALS));

    Ok(aum_in_wbtc)
}

/// Migrates the contract
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
