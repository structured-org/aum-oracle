use crate::error::{ContractError, ContractResult};
use crate::msg::{ExecuteMsg, GetAumResponse, InstantiateMsg, QueryMsg, UpdateConfig};
use crate::state::{Config, CONFIG};
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, Int256, MessageInfo, Response,
    StdResult,
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
    if new_config.oracles.is_some() {
        if let Some(ref oracles) = new_config.oracles {
            let validated_oracles: Vec<Addr> = oracles
                .iter()
                .map(|addr| deps.api.addr_validate(addr))
                .collect::<StdResult<_>>()?;
            contract_config.oracles = validated_oracles;
        }
    }

    if let Some(new_owner) = new_config.owner {
        let validated_new_owner = deps.api.addr_validate(&new_owner)?;
        contract_config.owner = validated_new_owner;
    }

    CONFIG.save(deps.storage, &contract_config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::GetAum {} => Ok(to_json_binary(&query_get_aum(deps, env)?)?),
        QueryMsg::GetConfig {} => Ok(to_json_binary(&CONFIG.load(deps.storage)?)?),
    }
}

pub fn query_get_aum(deps: Deps, _env: Env) -> ContractResult<GetAumResponse> {
    let config = CONFIG.load(deps.storage)?;

    let mut total_aum = Int256::zero();

    for oracle in config.oracles.iter() {
        let aum: GetAumResponse = deps.querier.query_wasm_smart(
            oracle,
            &serde_json::json!({
              "get_aum": {}
            }),
        )?;
        total_aum += aum.aum_in_btc;
    }

    Ok(GetAumResponse {
        aum_in_btc: total_aum,
    })
}
