use crate::msg::{
    ExecuteMsg, GetAumResponse, GetDataResponse, InstantiateMsg, QueryMsg, RoundInfoResponse,
};
use crate::state::{BinanceData, Config, CONFIG, CONSENSUS_STATE};
use consensus::consensus::{Config as ConsensusConfig, OracleData};
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
        valid_period: msg.valid_period,
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
    new_data: OracleData<BinanceData>,
) -> StdResult<Response> {
    let config = CONSENSUS_STATE.config.load(deps.storage)?;
    // Only oracle can submit
    if !config.oracles.contains(&info.sender) {
        return Err(StdError::generic_err("Unauthorized oracle"));
    }

    // Check if there's published data for this round before the update
    let last_published_before = CONSENSUS_STATE.last_published_data.may_load(deps.storage)?;
    let current_round = CONSENSUS_STATE.pending_round.load(deps.storage)?.round;

    CONSENSUS_STATE.publish_data(deps.storage, &env, info.sender, new_data)?;

    let mut res = Response::new();

    // Check if there's published data for this round after the update
    let last_published_after = CONSENSUS_STATE.last_published_data.may_load(deps.storage)?;

    // If we have new published data for the current round, consensus was reached
    if let Some(published_data) = last_published_after {
        if published_data.round == current_round
            && (last_published_before.is_none()
                || last_published_before.unwrap().round != current_round)
        {
            res = res.add_attribute("action", "publish_consensus");
        }
    }

    res = res.add_attributes([
        attr(
            "next_round",
            CONSENSUS_STATE
                .pending_round
                .load(deps.storage)?
                .next_round(config.round_length)
                .round
                .to_string(),
        ),
        attr(
            "round",
            CONSENSUS_STATE
                .pending_round
                .load(deps.storage)?
                .round
                .to_string(),
        ),
    ]);

    Ok(res)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetData {} => to_json_binary(&query_get_data(deps, env)?),
        QueryMsg::GetAum {} => to_json_binary(&query_get_aum(deps, env)?),
        QueryMsg::GetRoundInfo {} => to_json_binary(&query_round_info(deps, env)?),
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

fn query_get_data(deps: Deps, env: Env) -> StdResult<GetDataResponse> {
    Ok(GetDataResponse {
        last_published_data: CONSENSUS_STATE.get_last_published_data(&env, deps.storage)?,
    })
}

fn query_get_aum(deps: Deps, env: Env) -> StdResult<GetAumResponse> {
    let config = CONFIG.load(deps.storage)?;
    let d = CONSENSUS_STATE
        .last_published_data
        .may_load(deps.storage)?
        .ok_or_else(|| StdError::generic_err("No published data"))?;
    if d.timestamp + config.valid_period < env.block.time.seconds() {
        return Err(StdError::generic_err("Published data is outdated"));
    }
    // TODO: Replace btc_price_in_usd and btc_price_in(asset) with real oracle lookups.
    let btc_price_in_usd = SignedDecimal::from_ratio(65000, 1); // placeholder, in production query a real price
    let spot_total_balance_btc: SignedDecimal = d
        .data
        .spot_balances
        .iter()
        .map(|b| {
            if b.asset == "BTC" {
                b.amount
            } else if b.asset == "USDT" {
                b.amount / btc_price_in_usd
            } else {
                SignedDecimal::zero() // extend for more assets
            }
        })
        .sum();
    let aum_in_btc = (d.data.pm_account_actual_equity / btc_price_in_usd) + spot_total_balance_btc;
    Ok(GetAumResponse { aum_in_btc })
}
