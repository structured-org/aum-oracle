use crate::error::{ContractError, ContractResult};
use crate::msg::{
    ExecuteMsg, GetAumResponse, GetDataResponse, InstantiateMsg, QueryMsg, RoundInfoResponse,
};
use crate::state::{BinanceData, Config, CONFIG, CONSENSUS_STATE};
use crate::utils::{get_prices, spot_balance_in_btc};
use consensus::consensus::{Config as ConsensusConfig, ConsensusResult, OracleData};
use cosmwasm_std::{
    attr, entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    SignedDecimal, StdError, StdResult,
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let oracles: Vec<Addr> = msg
        .oracles
        .iter()
        .map(|addr| deps.api.addr_validate(addr))
        .collect::<StdResult<_>>()?;
    let consensus_config = ConsensusConfig {
        oracles,
        threshold: msg.threshold,
        data_delta_ppm: msg.data_delta_ppm,
        round_length: msg.round_length,
    };
    CONSENSUS_STATE.initialize(deps.storage, &env, consensus_config)?;

    let contract_config = Config {
        admin: deps.api.addr_validate(&msg.admin)?,
        consensus_data_valid_period: msg.consensus_data_valid_period,
        required_binance_spot_assets: msg.required_binance_spot_assets,
        required_binance_positions: msg.required_binance_positions,
        price_max_blocks_old: msg.price_data_valid_period,
        price_oracle_contract: deps.api.addr_validate(&msg.price_oracle_contract)?,
    };
    CONFIG.save(deps.storage, &contract_config)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::PublishData { new_data } => execute_publish_data(deps, env, info, new_data),
    }
}

fn execute_publish_data(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut new_data: OracleData<BinanceData>,
) -> StdResult<Response> {
    let contract_config = CONFIG.load(deps.storage)?;

    let consensus_config = CONSENSUS_STATE.config.load(deps.storage)?;
    // Only oracle can submit
    if !consensus_config.oracles.contains(&info.sender) {
        return Err(StdError::generic_err("Unauthorized oracle"));
    }

    // clean and validate published data
    new_data.data.clean_and_validate(
        contract_config.required_binance_positions,
        contract_config.required_binance_spot_assets,
    )?;

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
                .next_round(consensus_config.round_length)
                .round
                .to_string(),
        ),
        attr("round", pending_round.round.to_string()),
    ]);

    Ok(res)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::GetData {} => Ok(to_json_binary(&query_get_data(deps, env)?)?),
        QueryMsg::GetAum {} => Ok(to_json_binary(&query_get_aum(deps, env)?)?),
        QueryMsg::GetRoundInfo {} => Ok(to_json_binary(&query_round_info(deps, env)?)?),
    }
}

fn query_round_info(deps: Deps, _env: Env) -> StdResult<RoundInfoResponse> {
    let pending_round = CONSENSUS_STATE.get_pending_round(deps.storage)?;

    Ok(RoundInfoResponse {
        pending_round,
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
    let d = CONSENSUS_STATE
        .last_published_data
        .may_load(deps.storage)?
        .ok_or_else(|| StdError::generic_err("No published data"))?;
    if d.timestamp + config.consensus_data_valid_period < env.block.time.seconds() {
        return Err(ContractError::PublishedDataTooOld {});
    }

    let spot_total_balance_btc = d
        .data
        .spot_balances
        .iter()
        .map(|b| {
            spot_balance_in_btc(
                deps,
                config.price_oracle_contract.to_string(),
                config.price_max_blocks_old,
                b,
            )
        })
        .collect::<ContractResult<Vec<SignedDecimal>>>()?
        .iter()
        .try_fold(SignedDecimal::zero(), |total, b| total.checked_add(*b))?;

    let btc_price_in_usd = get_prices(
        deps,
        config.price_oracle_contract.to_string(),
        "BTC".to_string(),
        "USD".to_string(),
        config.price_max_blocks_old,
    )?
    .price_0_to_1;

    let aum_in_btc = (d.data.pm_account_actual_equity / btc_price_in_usd) + spot_total_balance_btc;

    Ok(GetAumResponse { aum_in_btc })
}
