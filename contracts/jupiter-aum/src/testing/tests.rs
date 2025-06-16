use crate::contract::{execute, instantiate, query};
use crate::error::ContractError::Unauthorized;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::CONFIG;
use cosmwasm_std::testing::{message_info, mock_env};
use cosmwasm_std::testing::{mock_dependencies, MockApi};

// Helper to create a default instantiate message
fn default_init_msg(api: MockApi) -> InstantiateMsg {
    InstantiateMsg {
        admin: api.addr_make("admin").to_string(),
        oracles: vec![
            api.addr_make("oracle1").to_string(),
            api.addr_make("oracle2").to_string(),
        ],
        threshold: 1,
        extract_period: 10,
        valid_period: 1000,
    }
}

/// Tests the following scenario:
///     1.  A non-authorized address tries to update config's contract (error)
///     2.  An authorized address tries to update config's contract
#[test]
fn test_update_config() {
    let mut deps = mock_dependencies();

    // Instantiate
    let env = mock_env();
    let admin = message_info(&deps.api.addr_make("admin"), &[]);
    let msg = default_init_msg(deps.api);
    let init_res = instantiate(deps.as_mut(), env.clone(), admin.clone(), msg).unwrap();
    assert_eq!(init_res.messages.len(), 0);

    let config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(config.admin, admin.sender);

    let update_msg = ExecuteMsg::UpdateConfig {
        admin: Some(deps.api.addr_make("admin2").to_string()),
        oracles: Some(vec![
            deps.api.addr_make("oracle3").to_string(),
            deps.api.addr_make("oracle4").to_string(),
        ]),
        threshold: Some(2),
        extract_period: Some(100000),
        valid_period: Some(50000),
    };

    // Unauthorized update
    let stranger_info = message_info(&deps.api.addr_make("stranger"), &[]);
    let unauthorized_res = execute(
        deps.as_mut(),
        env.clone(),
        stranger_info,
        update_msg.clone(),
    );
    assert_eq!(unauthorized_res.err().unwrap(), Unauthorized {});

    // Authorized update
    let authorized_res = execute(deps.as_mut(), env.clone(), admin.clone(), update_msg);
    assert!(authorized_res.is_ok());

    // Config should have updated values
    let config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(config.admin, deps.api.addr_make("admin2"));
    assert_eq!(
        config.oracles,
        vec![deps.api.addr_make("oracle3"), deps.api.addr_make("oracle4")]
    );
    assert_eq!(config.threshold, 2);
    assert_eq!(config.extract_period, 100000);
    assert_eq!(config.valid_period, 50000);
}
