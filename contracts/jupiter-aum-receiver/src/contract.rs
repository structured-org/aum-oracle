use crate::state::{AUM_IN_WBTC, CONFIG, CONSENSUS_STATE};
use aum_receiver_common::constants::WBTC_DECIMALS;
use aum_receiver_common::types::{aum_response_from_uwbtc, GetAumResponse, RoundInfoResponse};
use consensus::consensus::Config as ConsensusConfig;
use consensus::consensus::PublishResult;
use cosmwasm_std::{
    attr, entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, Int256, MessageInfo,
    Response, SignedDecimal256, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw_ownable::{get_ownership, update_ownership};
use cw_storage_plus::Map;
use jupiter_aum_common::error::ContractError;
use jupiter_aum_common::msg::{
    ConfigResponse, ExecuteMsg, GetDataResponse, InstantiateMsg, MigrateMsg, QueryMsg, UpdateConfig,
};
use jupiter_aum_common::types::{AumInWBTC, Config, PriceTicker, SolanaData};
use neutron_std::types::slinky::oracle::v1::OracleQuerier;
use neutron_std::types::slinky::types::v1::CurrencyPair;
use std::collections::HashMap;
use std::str::FromStr;

const CONTRACT_NAME: &str = "crates.io:jupiter-aum-receiver";
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

    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;

    let config = Config {
        consensus_data_valid_period: msg.consensus_data_valid_period,
        price_data_valid_period: msg.price_data_valid_period,
        required_custody_assets: msg.required_custody_assets,
        required_solana_balances: msg.required_solana_balances,
        required_solana_token_total_supply: msg.required_solana_token_total_supply,
        solana_slinky_map: msg.solana_slinky_map,
    };
    config.validate()?;

    CONFIG.save(deps.storage, &config)?;

    let consensus_config = ConsensusConfig {
        messengers: msg
            .messengers
            .iter()
            .map(|addr| deps.api.addr_validate(addr))
            .collect::<StdResult<_>>()?,
        threshold: msg.threshold,
        data_delta_ppm: msg.data_delta_ppm,
        round_length: msg.round_length,
    };
    consensus_config.validate()?;

    CONSENSUS_STATE.initialize(deps.storage, &env, consensus_config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", msg.owner.to_string()))
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
        ExecuteMsg::UpdateOwnership(action) => {
            update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::new().add_attribute("action", "update_ownership"))
        }
    }
}

/// Updates both general and consensus-related contract configuration.
/// Only callable by the current contract owner.
fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_config: UpdateConfig,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut contract_config = CONFIG.load(deps.storage)?;

    if let Some(new_consensus_data_valid_period) = new_config.consensus_data_valid_period {
        contract_config.consensus_data_valid_period = new_consensus_data_valid_period;
    }

    if let Some(new_price_data_valid_period) = new_config.price_data_valid_period {
        contract_config.price_data_valid_period = new_price_data_valid_period;
    }

    if let Some(new_required_custody_assets) = new_config.required_custody_assets {
        contract_config.required_custody_assets = new_required_custody_assets;
    }

    if let Some(new_required_solana_balances) = new_config.required_solana_balances {
        contract_config.required_solana_balances = new_required_solana_balances;
    }

    if let Some(new_required_solana_token_total_supply) =
        new_config.required_solana_token_total_supply
    {
        contract_config.required_solana_token_total_supply = new_required_solana_token_total_supply;
    }

    if let Some(new_solana_slinky_map) = new_config.solana_slinky_map {
        contract_config.solana_slinky_map = new_solana_slinky_map;
    }

    contract_config.validate()?;

    CONFIG.save(deps.storage, &contract_config)?;

    let mut consensus_config = CONSENSUS_STATE.config.load(deps.storage)?;

    if let Some(ref messengers) = new_config.messengers {
        consensus_config.messengers = messengers
            .iter()
            .map(|addr| deps.api.addr_validate(addr))
            .collect::<StdResult<_>>()?;
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
    consensus_config.validate()?;

    CONSENSUS_STATE.save_config(deps.storage, consensus_config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

/// Allows a registered messenger to publish Solana data for the current round.
/// If consensus is reached, finalizes the data and returns next round info in events.
fn execute_publish_data(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_data: SolanaData,
) -> Result<Response, ContractError> {
    let contract_config = CONFIG.load(deps.storage)?;

    let consensus_config = CONSENSUS_STATE.config.load(deps.storage)?;

    if !consensus_config.messengers.contains(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let (result, pending_round) =
        CONSENSUS_STATE.publish_data(deps.storage, &env, info.sender, new_data, contract_config)?;

    let mut res = Response::new();

    if let PublishResult::ConsensusReached(outcome) = result {
        let aum_amount = calculate_aum(deps.as_ref(), env, outcome.data)?;
        AUM_IN_WBTC.save(
            deps.storage,
            &AumInWBTC {
                amount: aum_amount,
                timestamp: outcome.timestamp,
            },
        )?;
        res = res
            .add_attribute("consensus_reached", outcome.round.to_string())
            .add_attribute("aum_in_wbtc", aum_amount);
    }

    let consensus_config = CONSENSUS_STATE.config.load(deps.storage)?;

    let next_round = pending_round.next_round(consensus_config.round_length);
    res = res.add_attributes([
        attr("action", "publish_data"),
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
        QueryMsg::GetData {} => Ok(to_json_binary(&query_get_data(deps, env)?)?),
        QueryMsg::GetAum {} => Ok(to_json_binary(&query_get_aum(deps, env)?)?),
        QueryMsg::GetRoundInfo {} => Ok(to_json_binary(&query_round_info(deps, env)?)?),
        QueryMsg::GetConfig {} => Ok(to_json_binary(&query_config(deps)?)?),
        QueryMsg::Ownership {} => Ok(to_json_binary(&get_ownership(deps.storage)?)?),
    }
}

/// Returns combined general and consensus configuration of the contract.
fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let consensus_config = CONSENSUS_STATE.config.load(deps.storage)?;
    Ok(ConfigResponse {
        consensus_data_valid_period: config.consensus_data_valid_period,
        required_custody_assets: config.required_custody_assets,
        price_data_valid_period: config.price_data_valid_period,
        messengers: consensus_config.messengers,
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
fn query_get_aum(deps: Deps, env: Env) -> Result<GetAumResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let aum_data: AumInWBTC = AUM_IN_WBTC
        .may_load(deps.storage)?
        .ok_or(ContractError::NoDataPublished {})?;
    if env.block.time.seconds() > aum_data.timestamp + config.consensus_data_valid_period {
        return Err(ContractError::PublishedDataTooOld {});
    }

    Ok(aum_response_from_uwbtc(aum_data.amount))
}

/// Returns pending and next round info.
fn query_round_info(deps: Deps, _env: Env) -> Result<RoundInfoResponse, ContractError> {
    let pending_round = CONSENSUS_STATE.get_pending_round(deps.storage)?;

    Ok(RoundInfoResponse {
        pending_round: pending_round.clone(),
        next_round: pending_round
            .next_round(CONSENSUS_STATE.config.load(deps.storage)?.round_length),
    })
}

/// Migrates the contract
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::StdError;
    use serde_json;

    // read and deserialize previous version of config
    #[cw_serde]
    struct OldConfig {
        consensus_data_valid_period: u64,
        required_custody_assets: Vec<String>,
        price_data_valid_period: u64,
    }
    let Some(old_config_bytes) = deps.storage.get(b"config") else {
        return Err(ContractError::Std(StdError::generic_err(
            "data not found at key config",
        )));
    };
    let old_config: OldConfig = serde_json::from_slice(&old_config_bytes).map_err(|e| {
        ContractError::Std(StdError::generic_err(format!(
            "failed to parse previous version of config: {}",
            e
        )))
    })?;

    // create new config out of the old one and MigrateMsg
    let config = Config {
        consensus_data_valid_period: old_config.consensus_data_valid_period,
        required_custody_assets: old_config.required_custody_assets,
        price_data_valid_period: old_config.price_data_valid_period,
        required_solana_balances: msg.required_solana_balances,
        required_solana_token_total_supply: msg.required_solana_token_total_supply,
        solana_slinky_map: msg.solana_slinky_map,
    };
    config.validate()?;
    CONFIG.save(deps.storage, &config)?;

    // If the state already contains pending data, consensus cannot be reached \
    // because that data does not meet the new SolanaData requirements and therefore cannot be processed.
    let pending_data: Map<Addr, SolanaData> = Map::new("consensus__pending_data");
    pending_data.clear(deps.storage);

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

/// Calculates and returns the current AUM value in wBTC.
pub fn calculate_aum(deps: Deps, env: Env, data: SolanaData) -> Result<Int256, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let btc_price_in_usd = get_asset_price_in_usd(deps, &env, &config, BTC_DENOM.to_string())?;
    let prices = config
        .solana_slinky_map
        .iter()
        .try_fold::<_, _, Result<_, ContractError>>(
            HashMap::new(),
            |mut acc, (solana_asset, price_ticker)| {
                let price = match price_ticker {
                    PriceTicker::Slinky { asset } => {
                        get_asset_price_in_usd(deps, &env, &config, asset.clone())?
                    }
                    PriceTicker::Jlp => get_jlp_price_in_usd(&data, &solana_asset.clone())?,
                };

                acc.insert(solana_asset.clone(), price.checked_div(btc_price_in_usd)?);
                Ok(acc)
            },
        )?;

    let aum_in_btc = calculate_aum_in_wbtc(prices, data)?;
    Ok(aum_in_btc)
}

/// Fetches the Asset/USD price from the slinky oracle and ensures freshness.
pub fn get_asset_price_in_usd(
    deps: Deps,
    env: &Env,
    config: &Config,
    base: String,
) -> Result<SignedDecimal256, ContractError> {
    let querier = OracleQuerier::new(&deps.querier);
    let response = querier.get_price(Some(CurrencyPair {
        base: base.clone(),
        quote: USD_DENOM.to_string(),
    }))?;

    let quote = response
        .price
        .ok_or(ContractError::SlinkyAssetPriceMissing {
            asset: base.clone(),
        })?;

    if quote.block_height + config.price_data_valid_period < env.block.height {
        return Err(ContractError::SlinkyAssetPriceTooOld {
            asset: base.clone(),
            price_height: quote.block_height,
        });
    }

    let asset_price_in_usd =
        Uint128::from_str(&quote.price).map_err(|e| ContractError::SlinkyAssetPriceIncorrect {
            asset: base,
            price: quote.price,
            error: e.to_string(),
        })?;

    let asset_price_in_usd =
        SignedDecimal256::from_atomics(asset_price_in_usd, response.decimals as u32).map_err(
            |e| ContractError::DecimalError {
                error: e.to_string(),
            },
        )?;

    Ok(asset_price_in_usd)
}

/// Computes the AUM in wBTC units using Solana data and BTC/USD price.
pub fn calculate_aum_in_wbtc(
    prices_in_btc: HashMap<String, SignedDecimal256>,
    data: SolanaData,
) -> Result<Int256, ContractError> {
    let decimals = data
        .solana_token_decimals
        .iter()
        .fold(HashMap::new(), |mut acc, dec| {
            acc.insert(&dec.asset, dec.decimals);
            acc
        });
    let res = data
        .solana_balances
        .iter()
        .try_fold::<_, _, Result<Int256, ContractError>>(Int256::zero(), |acc, sb| {
            let token_price = *prices_in_btc.get(&sb.asset).ok_or(
                ContractError::CrucialConsensusDataMissing {
                    details: format!("{} price", sb.asset),
                },
            )?;
            let dec =
                *decimals
                    .get(&sb.asset)
                    .ok_or(ContractError::CrucialConsensusDataMissing {
                        details: format!("{} decimals", sb.asset).to_string(),
                    })? as u32;
            Ok(acc
                + token_price
                    .checked_mul(SignedDecimal256::from_atomics(sb.amount, dec)?)?
                    .atomics()
                    / Int256::from_i128(
                        10i128.pow(SignedDecimal256::DECIMAL_PLACES - WBTC_DECIMALS),
                    ))
        })?;
    Ok(res)
}

pub fn get_jlp_price_in_usd(
    data: &SolanaData,
    jlp_token: &String,
) -> Result<SignedDecimal256, ContractError> {
    let consensus_jlp_total_supply = data
        .solana_token_total_supply
        .iter()
        .find(|t| t.asset == jlp_token.clone())
        .ok_or(ContractError::CrucialConsensusDataMissing {
            details: "JLP total supply".to_string(),
        })?
        .total_supply;
    let consensus_jlp_decimals = data
        .solana_token_decimals
        .iter()
        .find(|d| d.asset == jlp_token.clone())
        .ok_or(ContractError::CrucialConsensusDataMissing {
            details: "JLP decimals".to_string(),
        })?
        .decimals;

    let aum_usd = SignedDecimal256::from_atomics(data.aum_usd, 0)?;
    let total_jlp_supply =
        SignedDecimal256::from_atomics(consensus_jlp_total_supply, consensus_jlp_decimals as u32)?;

    let jlp_virtual_price = aum_usd.checked_div(total_jlp_supply)?;
    Ok(jlp_virtual_price)
}
