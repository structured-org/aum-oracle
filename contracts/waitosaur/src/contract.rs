use crate::state::{CONFIG, STATE};

use binance_aum_common::msg::GetDataResponse;
use binance_aum_common::msg::QueryMsg as BinanceAumQueryMsg;

use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    SignedDecimal256, StdResult,
};
use cw2::set_contract_version;
use cw_ownable::{get_ownership, update_ownership};
use waitosaur_common::{
    error::{ContractError, ContractResult},
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, UpdateConfig},
    types::State,
};

const CONTRACT_NAME: &str = "crates.io:waitosaur";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(msg.owner.as_str()))?;
    deps.api.addr_validate(msg.config.locker.as_ref())?;
    deps.api.addr_validate(msg.config.unlocker.as_ref())?;
    deps.api.addr_validate(msg.config.contract.as_ref())?;
    CONFIG.save(deps.storage, &msg.config)?;
    STATE.save(deps.storage, &State::Unlocked {})?;
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
        ExecuteMsg::UpdateConfig { new_config } => execute_update_config(deps, info, new_config),
        ExecuteMsg::Lock { amount } => execute_lock(deps, env, info, amount),
        ExecuteMsg::Unlock {} => execute_unlock(deps, env, info),
        ExecuteMsg::UpdateOwnership(action) => {
            update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::new().add_attribute("action", "update_ownership"))
        }
    }
}

fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_config: UpdateConfig,
) -> ContractResult<Response> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut config = CONFIG.load(deps.storage)?;

    // Update configuration if fields are provided
    if let Some(locker) = new_config.locker {
        config.locker = deps.api.addr_validate(&locker)?;
    }
    if let Some(unlocker) = new_config.unlocker {
        config.unlocker = deps.api.addr_validate(&unlocker)?;
    }
    if let Some(contract) = new_config.contract {
        config.contract = deps.api.addr_validate(&contract)?;
    }
    if let Some(asset) = new_config.asset {
        config.asset = asset;
    }

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_config"))
}

fn execute_lock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: SignedDecimal256,
) -> ContractResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.locker && cw_ownable::assert_owner(deps.storage, &info.sender).is_err()
    {
        return Err(ContractError::Unauthorized {});
    }
    let current_state = STATE.load(deps.storage)?;
    if let State::Locked { .. } = current_state {
        return Err(ContractError::AlreadyLocked {});
    }
    let state = State::Locked {
        amount,
        at_timestamp: env.block.time.nanos(),
    };
    STATE.save(deps.storage, &state)?;
    Ok(Response::new().add_attribute("action", "lock"))
}

fn execute_unlock(deps: DepsMut, _env: Env, info: MessageInfo) -> ContractResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.unlocker
        && cw_ownable::assert_owner(deps.storage, &info.sender).is_err()
    {
        return Err(ContractError::Unauthorized {});
    }
    let current_state = STATE.load(deps.storage)?;
    match current_state {
        State::Locked {
            amount,
            at_timestamp: _,
        } => {
            let response: GetDataResponse = deps
                .querier
                .query_wasm_smart(config.contract.as_str(), &BinanceAumQueryMsg::GetData {})?;
            let contract_data = response
                .last_published_data
                .ok_or(ContractError::NoDataInContract {})?;
            let position = contract_data
                .data
                .spot_balances
                .iter()
                .find(|pos| pos.asset == config.asset)
                .ok_or(ContractError::NoAssetFound {})?;
            if position.amount < amount {
                return Err(ContractError::InsufficientAssetAmount {});
            }
            let state = State::Unlocked {};
            STATE.save(deps.storage, &state)?;
            Ok(Response::new().add_attribute("action", "unlock"))
        }
        State::Unlocked {} => Err(ContractError::AlreadyUnlocked {}),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => Ok(to_json_binary(&CONFIG.load(deps.storage)?)?),
        QueryMsg::GetState {} => Ok(to_json_binary(&STATE.load(deps.storage)?)?),
        QueryMsg::Ownership {} => Ok(to_json_binary(&get_ownership(deps.storage)?)?),
    }
}

/// Migrates the contract
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}
