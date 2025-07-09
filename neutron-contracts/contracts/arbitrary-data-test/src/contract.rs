use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

use crate::{
    msgs::{InstantiateMsg, QueryMsg},
    state::TestData,
};

pub const CONTRACT_NAME: &str = concat!("crates.io:aum_messenger__", env!("CARGO_PKG_NAME"));

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetTestData {} => query_test_data(),
    }
}

fn query_test_data() -> StdResult<Binary> {
    let test_data = TestData {
        frist_value: "first_value".to_string(),
        second_value: "second_value".to_string(),
    };

    to_json_binary(&test_data)
}
