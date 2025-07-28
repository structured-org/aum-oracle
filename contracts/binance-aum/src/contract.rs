use crate::error::{ContractError, ContractResult};
use crate::msg::{
    ExecuteMsg, GetAumResponse, GetConfigResponse, GetDataResponse, InstantiateMsg, QueryMsg,
    RoundInfoResponse, UpdateConfig,
};
use crate::state::{BinanceData, Config, CONFIG, CONSENSUS_STATE};
use crate::utils::{get_prices, spot_balance_asset_in_btc};
use consensus::consensus::{Config as ConsensusConfig, PublishResult};
use cosmwasm_std::{
    attr, entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, Int256, MessageInfo,
    Response, SignedDecimal256, StdError, StdResult,
};

const WBTC_DECIMALS: u32 = 8; // WBTC via Eureka has 8 decimals

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let messengers: Vec<Addr> = msg
        .messengers
        .iter()
        .map(|addr| deps.api.addr_validate(addr))
        .collect::<StdResult<_>>()?;
    let consensus_config = ConsensusConfig {
        messengers,
        threshold: msg.threshold,
        data_delta_ppm: msg.data_delta_ppm,
        round_length: msg.round_length,
    };
    CONSENSUS_STATE.initialize(deps.storage, &env, consensus_config)?;

    let contract_config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        consensus_data_valid_period: msg.consensus_data_valid_period,
        required_binance_spot_assets: msg.required_binance_spot_assets,
        required_binance_positions: msg.required_binance_positions,
        price_data_valid_period: msg.price_data_valid_period,
        price_oracle_contract: deps.api.addr_validate(&msg.price_oracle_contract)?,
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
        ExecuteMsg::PublishData { new_data } => {
            Ok(execute_publish_data(deps, env, info, new_data)?)
        }
        ExecuteMsg::UpdateConfig { new_config } => {
            Ok(execute_update_config(deps, env, info, new_config)?)
        }
    }
}

fn execute_publish_data(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut new_data: BinanceData,
) -> ContractResult<Response> {
    let contract_config = CONFIG.load(deps.storage)?;

    let consensus_config = CONSENSUS_STATE.config.load(deps.storage)?;
    // Only oracle can submit
    if !consensus_config.messengers.contains(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    // clean and validate published data
    new_data.clean_and_validate(
        contract_config.required_binance_positions,
        contract_config.required_binance_spot_assets,
    )?;

    let (result, pending_round) =
        CONSENSUS_STATE.publish_data(deps.storage, &env, info.sender, new_data)?;

    let mut res = Response::new();

    // If we have new published data for the current round, consensus was reached
    if let PublishResult::ConsensusReached(_) = result {
        res = res.add_attribute("action", "publish_consensus");
    }

    let next_round = pending_round.next_round(consensus_config.round_length);
    res = res.add_attributes([
        attr("next_round", next_round.round.to_string()),
        attr("next_round_timestamp", next_round.start.to_string()),
        attr("round", pending_round.round.to_string()),
    ]);

    Ok(res)
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

    // Update contract configuration
    contract_config.update_config(deps.as_ref(), &new_config)?;
    CONFIG.save(deps.storage, &contract_config)?;

    let mut consensus_config = CONSENSUS_STATE.config.load(deps.storage)?;

    // Update consensus config fields
    if let Some(ref messengers) = new_config.messengers {
        let validated_messengers: Vec<Addr> = messengers
            .iter()
            .map(|addr| deps.api.addr_validate(addr))
            .collect::<StdResult<_>>()?;
        consensus_config.messengers = validated_messengers;
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

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::GetData {} => Ok(to_json_binary(&query_get_data(deps, env)?)?),
        QueryMsg::GetAum {} => Ok(to_json_binary(&query_get_aum(deps, env)?)?),
        QueryMsg::GetRoundInfo {} => Ok(to_json_binary(&query_round_info(deps, env)?)?),
        QueryMsg::GetConfig {} => Ok(to_json_binary(&query_config(deps, env)?)?),
    }
}

fn query_config(deps: Deps, _env: Env) -> StdResult<GetConfigResponse> {
    let consensus_config = CONSENSUS_STATE.config.load(deps.storage)?;
    let contract_config = CONFIG.load(deps.storage)?;

    Ok(GetConfigResponse {
        contract_config,
        consensus_config,
    })
}

fn query_round_info(deps: Deps, _env: Env) -> StdResult<RoundInfoResponse> {
    let pending_round = CONSENSUS_STATE.get_pending_round(deps.storage)?;

    Ok(RoundInfoResponse {
        pending_round: pending_round.clone(),
        next_round: pending_round
            .next_round(CONSENSUS_STATE.config.load(deps.storage)?.round_length),
    })
}

fn query_get_data(deps: Deps, env: Env) -> ContractResult<GetDataResponse> {
    Ok(GetDataResponse {
        last_published_data: CONSENSUS_STATE.get_last_published_data(&env, deps.storage)?,
    })
}

pub fn query_get_aum(deps: Deps, env: Env) -> ContractResult<GetAumResponse> {
    let config = CONFIG.load(deps.storage)?;
    let last_published_data = CONSENSUS_STATE
        .last_published_data
        .may_load(deps.storage)?
        .ok_or_else(|| StdError::generic_err("No published data"))?;
    if last_published_data.timestamp + config.consensus_data_valid_period < env.block.time.seconds()
    {
        return Err(ContractError::PublishedDataTooOld {});
    }

    let spot_total_balance_btc = last_published_data
        .data
        .spot_balances
        .iter()
        .map(|sb| {
            spot_balance_asset_in_btc(
                deps,
                config.price_oracle_contract.to_string(),
                config.price_data_valid_period,
                sb,
            )
        })
        .collect::<ContractResult<Vec<SignedDecimal256>>>()?
        .iter()
        .try_fold(SignedDecimal256::zero(), |total, b| total.checked_add(*b))?;

    let btc_price_in_usd = get_prices(
        deps,
        config.price_oracle_contract.to_string(),
        "BTC".to_string(),
        "USD".to_string(),
        config.price_data_valid_period,
    )?
    .price_0_to_1;

    let aum_in_btc = (last_published_data.data.pm_account_actual_equity / btc_price_in_usd)
        + spot_total_balance_btc;

    // here we convert the AUM in BTC to WBTC (uwBTC specifically)
    // since WBTC has 8 decimals, we need to adjust the decimal places accordingly
    // We get atomics value, which has 18 decimal places and divided it by 10^(18 - WBTC_DECIMALS),
    // which 10^10
    // In result we get AUM in uwBTC
    let aum_in_wbtc = aum_in_btc.atomics()
        / Int256::from_i128(10i128.pow(aum_in_btc.decimal_places() - WBTC_DECIMALS));

    Ok(GetAumResponse {
        aum_in_btc: aum_in_wbtc,
        decimals: WBTC_DECIMALS,
    })
}
