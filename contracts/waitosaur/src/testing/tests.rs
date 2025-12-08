use crate::contract::{execute, instantiate, query};
use crate::state::CONFIG;
use crate::testing::mock_querier::mock_dependencies;
use binance_aum_common::msg::GetDataResponse;
use binance_aum_common::types::{BinanceData, SpotBalance};
use consensus::consensus::ConsensusOutcome;
use cosmwasm_std::testing::{message_info, mock_env, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    from_json, to_json_binary, Addr, ContractResult, Env, OwnedDeps, SignedDecimal256, StdError,
    SystemResult, Uint64, WasmQuery,
};
use cw_ownable::Action;
use cw_ownable::OwnershipError::{NotOwner, NotPendingOwner};
use std::str::FromStr;
use waitosaur_common::error::ContractError;
use waitosaur_common::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, UpdateConfig};
use waitosaur_common::types::{Config, State};

#[test]
fn proper_initialization() {
    let deps = setup_contract();

    let config = CONFIG.load(&deps.storage).unwrap();
    let expected_config = Config {
        unlocker: deps.api.addr_make("unlocker"),
        locker: deps.api.addr_make("locker"),
        contract: deps.api.addr_make("receiver_contract"),
        asset: "asset".to_string(),
        aum_stale_period: Uint64::from(100u64),
    };
    let owner = deps.api.addr_make("owner");
    cw_ownable::assert_owner(&deps.storage, &owner).unwrap();
    assert_config_equals(&config, &expected_config);
}

#[test]
fn test_instantiate_with_invalid_owner() {
    let mut deps = mock_dependencies();
    let owner = deps.api.addr_make("owner");

    let msg = InstantiateMsg {
        owner: "invalid...address...".to_string(),
        config: Config {
            locker: deps.api.addr_make("locker"),
            unlocker: deps.api.addr_make("unlocker"),
            contract: deps.api.addr_make("receiver_contract"),
            asset: "asset".to_string(),
            aum_stale_period: Uint64::from(100u64),
        },
    };
    let info = message_info(&owner, &[]);
    let err = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert!(matches!(
        err,
        ContractError::Std(StdError::GenericErr { .. })
    ));
}

#[test]
fn test_instantiate_with_invalid_locker() {
    let mut deps = mock_dependencies();
    let owner = deps.api.addr_make("owner");

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        config: Config {
            locker: Addr::unchecked("invalid...address..."),
            unlocker: deps.api.addr_make("unlocker"),
            contract: deps.api.addr_make("receiver_contract"),
            asset: "asset".to_string(),
            aum_stale_period: Uint64::from(100u64),
        },
    };
    let info = message_info(&owner, &[]);
    let err = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert!(matches!(
        err,
        ContractError::Std(StdError::GenericErr { .. })
    ));
}

#[test]
fn test_instantiate_with_invalid_unlocker() {
    let mut deps = mock_dependencies();
    let owner = deps.api.addr_make("owner");

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        config: Config {
            locker: deps.api.addr_make("locker"),
            unlocker: Addr::unchecked("invalid...address..."),
            contract: deps.api.addr_make("receiver_contract"),
            asset: "asset".to_string(),
            aum_stale_period: Uint64::from(100u64),
        },
    };
    let info = message_info(&owner, &[]);
    let err = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert!(matches!(
        err,
        ContractError::Std(StdError::GenericErr { .. })
    ));
}

#[test]
fn test_instantiate_with_invalid_contract() {
    let mut deps = mock_dependencies();
    let owner = deps.api.addr_make("owner");

    let msg = InstantiateMsg {
        owner: owner.to_string(),
        config: Config {
            locker: deps.api.addr_make("locker"),
            unlocker: deps.api.addr_make("unlocker"),
            contract: Addr::unchecked("invalid...address..."),
            asset: "asset".to_string(),
            aum_stale_period: Uint64::from(100u64),
        },
    };
    let info = message_info(&owner, &[]);
    let err = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert!(matches!(
        err,
        ContractError::Std(StdError::GenericErr { .. })
    ));
}

#[test]
fn test_ownership() {
    let mut deps = cosmwasm_std::testing::mock_dependencies();
    let env = mock_env();
    let first_owner = deps.api.addr_make("first_owner");
    let second_owner = deps.api.addr_make("second_owner");
    let other = deps.api.addr_make("other");

    let owner_info = message_info(&first_owner, &[]);
    let second_owner_info = message_info(&second_owner, &[]);
    let other_info = message_info(&other, &[]);

    // Test empty vector for messengers
    let msg = InstantiateMsg {
        owner: first_owner.to_string(),
        config: Config {
            locker: deps.api.addr_make("locker"),
            unlocker: deps.api.addr_make("unlocker"),
            contract: deps.api.addr_make("receiver_contract"),
            asset: "asset".to_string(),
            aum_stale_period: Uint64::from(100u64),
        },
    };
    let result = instantiate(deps.as_mut(), env.clone(), owner_info.clone(), msg);
    assert!(result.is_ok());

    // owner is written in instantiate
    let ownership = cw_ownable::get_ownership(&deps.storage).unwrap();
    assert_eq!(ownership.owner.unwrap(), first_owner);

    // non-owner cannot transfer ownership
    let update_msg_1 = ExecuteMsg::UpdateOwnership(Action::TransferOwnership {
        new_owner: other.to_string(),
        expiry: None,
    });
    let result = execute(deps.as_mut(), env.clone(), other_info.clone(), update_msg_1).unwrap_err();
    assert_eq!(result, ContractError::Ownable(NotOwner));

    // transfer ownership writes pending owner
    let update_msg_2 = ExecuteMsg::UpdateOwnership(Action::TransferOwnership {
        new_owner: second_owner.to_string(),
        expiry: None,
    });
    let result = execute(deps.as_mut(), env.clone(), owner_info, update_msg_2);
    assert!(result.is_ok());
    let ownership = cw_ownable::get_ownership(&deps.storage).unwrap();
    assert_eq!(ownership.owner.unwrap(), first_owner);
    assert_eq!(ownership.pending_owner.unwrap(), second_owner);

    // other user cannot accept ownership
    let update_msg_3 = ExecuteMsg::UpdateOwnership(Action::AcceptOwnership {});
    let result = execute(deps.as_mut(), env.clone(), other_info, update_msg_3).unwrap_err();
    assert_eq!(result, ContractError::Ownable(NotPendingOwner));

    // pending owner can accept ownership
    let update_msg_4 = ExecuteMsg::UpdateOwnership(Action::AcceptOwnership {});
    let result = execute(deps.as_mut(), env.clone(), second_owner_info, update_msg_4);
    assert!(result.is_ok());
    let ownership = cw_ownable::get_ownership(&deps.storage).unwrap();
    assert_eq!(ownership.owner.unwrap(), second_owner);
}

#[test]
fn update_config_by_owner() {
    let mut deps = setup_contract();
    let owner = deps.api.addr_make("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_config: UpdateConfig {
            locker: Some(deps.api.addr_make("new_locker").to_string()),
            unlocker: Some(deps.api.addr_make("new_unlocker").to_string()),
            contract: Some(deps.api.addr_make("new_contract").to_string()),
            asset: Some("new_asset".to_string()),
            aum_stale_period: Some(Uint64::from(200u64)),
        },
    };

    let res = execute_msg(&mut deps, mock_env(), &owner, msg).unwrap();
    assert_eq!(0, res.messages.len());

    let config = CONFIG.load(&deps.storage).unwrap();
    let expected_config = Config {
        locker: deps.api.addr_make("new_locker"),
        unlocker: deps.api.addr_make("new_unlocker"),
        contract: deps.api.addr_make("new_contract"),
        asset: "new_asset".to_string(),
        aum_stale_period: Uint64::from(200u64),
    };
    assert_config_equals(&config, &expected_config);
}

#[test]
fn update_config_by_unauthorized() {
    let mut deps = setup_contract();
    let oracle1 = deps.api.addr_make("oracle1");

    let msg = ExecuteMsg::UpdateConfig {
        new_config: UpdateConfig {
            locker: Some(deps.api.addr_make("new_locker").to_string()),
            unlocker: Some(deps.api.addr_make("new_unlocker").to_string()),
            contract: Some(deps.api.addr_make("new_contract").to_string()),
            asset: Some("new_asset".to_string()),
            aum_stale_period: Some(Uint64::from(200u64)),
        },
    };

    let err = execute_msg(&mut deps, mock_env(), &oracle1, msg).unwrap_err();
    assert_eq!(err, ContractError::Ownable(NotOwner));

    let config = CONFIG.load(&deps.storage).unwrap();
    let expected_config = Config {
        locker: deps.api.addr_make("locker"),
        unlocker: deps.api.addr_make("unlocker"),
        contract: deps.api.addr_make("receiver_contract"),
        asset: "asset".to_string(),
        aum_stale_period: Uint64::from(100u64),
    };
    assert_config_equals(&config, &expected_config);
}

#[test]
fn query_config() {
    let deps = setup_contract();
    let bin = query_msg(&deps, mock_env(), QueryMsg::GetConfig {}).unwrap();
    let config: Config = from_json(bin).unwrap();

    let expected_config = Config {
        locker: deps.api.addr_make("locker"),
        unlocker: deps.api.addr_make("unlocker"),
        contract: deps.api.addr_make("receiver_contract"),
        asset: "asset".to_string(),
        aum_stale_period: Uint64::from(100u64),
    };
    assert_config_equals(&config, &expected_config);
}

#[test]
fn query_default_state() {
    let deps = setup_contract();
    let bin = query_msg(&deps, mock_env(), QueryMsg::GetState {}).unwrap();
    let res: State = from_json(bin).unwrap();

    assert_eq!(res, State::Unlocked {});
}

#[test]
fn lock_and_query_state() {
    let mut deps = setup_contract();
    let locker = deps.api.addr_make("locker");

    let lock_msg = ExecuteMsg::Lock {
        amount: SignedDecimal256::from_str("100.50").unwrap(),
    };
    let res = execute_msg(&mut deps, mock_env(), &locker, lock_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let bin = query_msg(&deps, mock_env(), QueryMsg::GetState {}).unwrap();
    let res: State = from_json(bin).unwrap();

    assert_eq!(
        res,
        State::Locked {
            amount: SignedDecimal256::from_str("100.50").unwrap(),
            at_timestamp: Uint64::from(mock_env().block.time.nanos()),
        }
    );
}
#[test]
fn lock_already_locked() {
    let mut deps = setup_contract();
    let locker = deps.api.addr_make("locker");

    let lock_msg = ExecuteMsg::Lock {
        amount: SignedDecimal256::from_str("100.50").unwrap(),
    };
    let res = execute_msg(&mut deps, mock_env(), &locker, lock_msg.clone()).unwrap();
    assert_eq!(0, res.messages.len());

    let err = execute_msg(&mut deps, mock_env(), &locker, lock_msg).unwrap_err();
    assert_eq!(err, ContractError::AlreadyLocked {});
}

#[test]
fn lock_unauthorized() {
    let mut deps = setup_contract();
    let stranger = deps.api.addr_make("stranger");

    let lock_msg = ExecuteMsg::Lock {
        amount: SignedDecimal256::from_str("100.50").unwrap(),
    };
    let err = execute_msg(&mut deps, mock_env(), &stranger, lock_msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});
}

#[test]
fn unlock_not_locked() {
    let mut deps = setup_contract();
    let stranger = deps.api.addr_make("unlocker");

    let unlock_msg = ExecuteMsg::Unlock {};
    let err = execute_msg(&mut deps, mock_env(), &stranger, unlock_msg).unwrap_err();
    assert_eq!(err, ContractError::AlreadyUnlocked {});
}

#[test]
fn unlock_unauthorized() {
    let mut deps = setup_contract();
    let stranger = deps.api.addr_make("stranger");

    let unlock_msg = ExecuteMsg::Unlock {};
    let err = execute_msg(&mut deps, mock_env(), &stranger, unlock_msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});
}

#[test]

fn unlock_success() {
    let mut deps = setup_contract_with_standard_querier_and_config();
    let locker = deps.api.addr_make("locker");
    let unlocker = deps.api.addr_make("unlocker");

    deps.querier.update_wasm(mock_receiver_response(
        deps.api.addr_make("receiver_contract").to_string(),
        "asset".to_string(),
        Some(SignedDecimal256::from_str("200.75").unwrap()),
        Some(mock_env().block.time.seconds()),
    ));

    let lock_msg = ExecuteMsg::Lock {
        amount: SignedDecimal256::from_str("100.50").unwrap(),
    };
    let res = execute_msg(&mut deps, mock_env(), &locker, lock_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let unlock_msg = ExecuteMsg::Unlock {};
    let res = execute_msg(&mut deps, mock_env(), &unlocker, unlock_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let bin = query_msg(&deps, mock_env(), QueryMsg::GetState {}).unwrap();
    let res: State = from_json(bin).unwrap();

    assert_eq!(res, State::Unlocked {});
}

#[test]
fn unlock_wrong_asset() {
    let mut deps = setup_contract_with_standard_querier_and_config();
    let locker = deps.api.addr_make("locker");
    let unlocker = deps.api.addr_make("unlocker");

    deps.querier.update_wasm(mock_receiver_response(
        deps.api.addr_make("receiver_contract").to_string(),
        "wrong_asset".to_string(),
        Some(SignedDecimal256::from_str("200.75").unwrap()),
        Some(mock_env().block.time.seconds()),
    ));

    let lock_msg = ExecuteMsg::Lock {
        amount: SignedDecimal256::from_str("100.50").unwrap(),
    };
    let res = execute_msg(&mut deps, mock_env(), &locker, lock_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let bin = query_msg(&deps, mock_env(), QueryMsg::GetState {}).unwrap();
    let res: State = from_json(bin).unwrap();
    assert_eq!(
        res,
        State::Locked {
            amount: SignedDecimal256::from_str("100.50").unwrap(),
            at_timestamp: Uint64::from(mock_env().block.time.nanos()),
        }
    );

    let unlock_msg = ExecuteMsg::Unlock {};
    let res = execute_msg(&mut deps, mock_env(), &unlocker, unlock_msg).unwrap_err();
    assert_eq!("No asset found in the published data", res.to_string());

    let bin = query_msg(&deps, mock_env(), QueryMsg::GetState {}).unwrap();
    let res: State = from_json(bin).unwrap();

    assert_eq!(
        res,
        State::Locked {
            amount: SignedDecimal256::from_str("100.50").unwrap(),
            at_timestamp: Uint64::from(mock_env().block.time.nanos()),
        }
    );
}

#[test]
fn unlock_stale_aum() {
    let mut deps = setup_contract_with_standard_querier_and_config();
    let locker = deps.api.addr_make("locker");
    let unlocker = deps.api.addr_make("unlocker");

    deps.querier.update_wasm(mock_receiver_response(
        deps.api.addr_make("receiver_contract").to_string(),
        "asset".to_string(),
        Some(SignedDecimal256::from_str("200.75").unwrap()),
        Some(mock_env().block.time.seconds() - 200u64),
    ));

    let lock_msg = ExecuteMsg::Lock {
        amount: SignedDecimal256::from_str("100.50").unwrap(),
    };
    let res = execute_msg(&mut deps, mock_env(), &locker, lock_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let bin = query_msg(&deps, mock_env(), QueryMsg::GetState {}).unwrap();
    let res: State = from_json(bin).unwrap();
    assert_eq!(
        res,
        State::Locked {
            amount: SignedDecimal256::from_str("100.50").unwrap(),
            at_timestamp: Uint64::from(mock_env().block.time.nanos()),
        }
    );

    let unlock_msg = ExecuteMsg::Unlock {};
    let res = execute_msg(&mut deps, mock_env(), &unlocker, unlock_msg).unwrap_err();
    assert_eq!("AUM data is stale", res.to_string());

    let bin = query_msg(&deps, mock_env(), QueryMsg::GetState {}).unwrap();
    let res: State = from_json(bin).unwrap();

    assert_eq!(
        res,
        State::Locked {
            amount: SignedDecimal256::from_str("100.50").unwrap(),
            at_timestamp: Uint64::from(mock_env().block.time.nanos()),
        }
    );
}

#[test]
fn unlock_no_data() {
    let mut deps = setup_contract_with_standard_querier_and_config();
    let locker = deps.api.addr_make("locker");
    let unlocker = deps.api.addr_make("unlocker");

    deps.querier.update_wasm(mock_receiver_response(
        deps.api.addr_make("receiver_contract").to_string(),
        "asset".to_string(),
        None,
        None,
    ));

    let lock_msg = ExecuteMsg::Lock {
        amount: SignedDecimal256::from_str("100.50").unwrap(),
    };
    let res = execute_msg(&mut deps, mock_env(), &locker, lock_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let bin = query_msg(&deps, mock_env(), QueryMsg::GetState {}).unwrap();
    let res: State = from_json(bin).unwrap();
    assert_eq!(
        res,
        State::Locked {
            amount: SignedDecimal256::from_str("100.50").unwrap(),
            at_timestamp: Uint64::from(mock_env().block.time.nanos()),
        }
    );

    let unlock_msg = ExecuteMsg::Unlock {};
    let res = execute_msg(&mut deps, mock_env(), &unlocker, unlock_msg).unwrap_err();
    assert_eq!("No data available in the target contract", res.to_string());

    let bin = query_msg(&deps, mock_env(), QueryMsg::GetState {}).unwrap();
    let res: State = from_json(bin).unwrap();

    assert_eq!(
        res,
        State::Locked {
            amount: SignedDecimal256::from_str("100.50").unwrap(),
            at_timestamp: Uint64::from(mock_env().block.time.nanos()),
        }
    );
}

#[test]
fn unlock_little_amount() {
    let mut deps = setup_contract_with_standard_querier_and_config();
    let locker = deps.api.addr_make("locker");
    let unlocker = deps.api.addr_make("unlocker");

    deps.querier.update_wasm(mock_receiver_response(
        deps.api.addr_make("receiver_contract").to_string(),
        "asset".to_string(),
        Some(SignedDecimal256::from_str("50.25").unwrap()),
        Some(mock_env().block.time.seconds()),
    ));

    let lock_msg = ExecuteMsg::Lock {
        amount: SignedDecimal256::from_str("300.50").unwrap(),
    };
    let res = execute_msg(&mut deps, mock_env(), &locker, lock_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let bin = query_msg(&deps, mock_env(), QueryMsg::GetState {}).unwrap();
    let res: State = from_json(bin).unwrap();
    assert_eq!(
        res,
        State::Locked {
            amount: SignedDecimal256::from_str("300.50").unwrap(),
            at_timestamp: Uint64::from(mock_env().block.time.nanos()),
        }
    );

    let unlock_msg = ExecuteMsg::Unlock {};
    let res = execute_msg(&mut deps, mock_env(), &unlocker, unlock_msg).unwrap_err();
    assert_eq!("Insufficient asset amount to unlock", res.to_string());

    let bin = query_msg(&deps, mock_env(), QueryMsg::GetState {}).unwrap();
    let res: State = from_json(bin).unwrap();

    assert_eq!(
        res,
        State::Locked {
            amount: SignedDecimal256::from_str("300.50").unwrap(),
            at_timestamp: Uint64::from(mock_env().block.time.nanos()),
        }
    );
}

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Creates a contract with custom mock querier
fn setup_contract() -> OwnedDeps<MockStorage, MockApi, crate::testing::mock_querier::WasmMockQuerier>
{
    let mut deps = mock_dependencies();
    let owner = deps.api.addr_make("owner");

    let msg = InstantiateMsg {
        owner: deps.api.addr_make("owner").to_string(),
        config: Config {
            locker: deps.api.addr_make("locker"),
            unlocker: deps.api.addr_make("unlocker"),
            contract: deps.api.addr_make("receiver_contract"),
            asset: "asset".to_string(),
            aum_stale_period: Uint64::from(100u64),
        },
    };
    let info = message_info(&owner, &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    deps
}

// /// Creates a contract with standard querier
// fn setup_contract_with_standard_querier() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
//     setup_contract_with_standard_querier_and_config(ContractSetup::default())
// }

/// Creates a contract with standard querier and custom configuration
fn setup_contract_with_standard_querier_and_config() -> OwnedDeps<MockStorage, MockApi, MockQuerier>
{
    let mut deps = cosmwasm_std::testing::mock_dependencies();
    let owner = deps.api.addr_make("owner");
    let msg = InstantiateMsg {
        owner: owner.to_string(),
        config: Config {
            locker: deps.api.addr_make("locker"),
            unlocker: deps.api.addr_make("unlocker"),
            contract: deps.api.addr_make("receiver_contract"),
            asset: "asset".to_string(),
            aum_stale_period: Uint64::from(100u64),
        },
    };
    let info = message_info(&owner, &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    deps
}

/// Mock receiver response helper
fn mock_receiver_response(
    receiver_contract: String,
    asset: String,
    amount: Option<SignedDecimal256>,
    timestamp: Option<u64>,
) -> impl Fn(&WasmQuery) -> SystemResult<ContractResult<cosmwasm_std::Binary>> {
    move |query| match query {
        WasmQuery::Smart {
            msg: _,
            contract_addr,
        } => {
            if *contract_addr == receiver_contract {
                let res = GetDataResponse {
                    last_published_data: amount.map(|amount| ConsensusOutcome {
                        round: 1,
                        timestamp: timestamp.unwrap_or(0u64),
                        data: BinanceData {
                            unimmr: SignedDecimal256::zero(),
                            positions: vec![],
                            um_balance_usdt: SignedDecimal256::zero(),
                            spot_balances: vec![SpotBalance {
                                asset: asset.clone(),
                                amount,
                            }],
                            pm_account_actual_equity: SignedDecimal256::zero(),
                            withdrawable_usdt: SignedDecimal256::zero(),
                        },
                    }),
                };
                let bin = to_json_binary(&res).unwrap();
                return SystemResult::Ok(ContractResult::Ok(bin));
            }
            panic!("unexpected contract address");
        }
        _ => panic!("unexpected query"),
    }
}

/// Execute message helper
fn execute_msg<T>(
    deps: &mut OwnedDeps<MockStorage, MockApi, T>,
    env: Env,
    sender: &Addr,
    msg: ExecuteMsg,
) -> Result<cosmwasm_std::Response, ContractError>
where
    T: cosmwasm_std::Querier,
{
    let info = message_info(sender, &[]);
    execute(deps.as_mut(), env, info, msg)
}

/// Query helper
fn query_msg<T>(
    deps: &OwnedDeps<MockStorage, MockApi, T>,
    env: Env,
    msg: QueryMsg,
) -> Result<cosmwasm_std::Binary, ContractError>
where
    T: cosmwasm_std::Querier,
{
    query(deps.as_ref(), env, msg)
}

/// Config verification helper
fn assert_config_equals(config: &Config, expected_config: &Config) {
    assert_eq!(config.locker, expected_config.locker);
    assert_eq!(config.unlocker, expected_config.unlocker);
    assert_eq!(config.asset, expected_config.asset);
}
