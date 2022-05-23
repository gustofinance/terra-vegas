use std::convert::TryInto;
use cosmwasm_bignumber::Uint256;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Reply,
    Response, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw0::nonpayable;
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg};
use moneymarket::market::{Cw20HookMsg as AnchorHookMsg, ExecuteMsg as AnchorMsg};

use crate::error::ContractError;
use crate::msg::{CurrentBalance, ExecuteMsg, Games, InstantiateMsg, QueryMsg, Threshold, BALANCEHISTORY};
use crate::state::{Config, CONFIG, GAMES, OWNER, REQUESTING_CONTRACT, BALANCE_HISTORY};
use crate::utils::*;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:reserve-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    OWNER.save(deps.storage, &info.sender)?;

    let anchor_market_address = deps.api.addr_validate(&msg.anchor_market_address)?;
    let anchor_token_address = deps.api.addr_validate(&msg.anchor_token_address)?;
    let gov_contract_address = deps.api.addr_validate(&msg.gov_contract_address)?;
    let config = Config {
        anchor_market_address,
        gov_contract_address,
        anchor_token_address,
        threshold: msg.threshold,
        native_denom: msg.native_denom,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ChangeThreshold { threshold } => {
            execute_change_threshold(deps, info, threshold)
        }
        ExecuteMsg::AddGame { addr } => execute_add_game(deps, info, addr),
        ExecuteMsg::RemoveGame { addr } => execute_remove_game(deps, info, addr),
        ExecuteMsg::RequestFunds { amount } => execute_requeset_funds(deps, env, info, amount),
        ExecuteMsg::DepositFunds {} => execute_deposit_funds(deps, env, info),
    }
}

pub fn execute_change_threshold(
    deps: DepsMut,
    info: MessageInfo,
    threshold: Uint128,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info)?;

    CONFIG.update(
        deps.storage,
        move |mut config| -> Result<_, ContractError> {
            config.threshold = threshold;
            Ok(config)
        },
    )?;
    Ok(Response::default())
}

pub fn execute_add_game(
    deps: DepsMut,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info)?;

    let game_addr = deps.api.addr_validate(&addr)?;
    GAMES.save(deps.storage, game_addr, &())?;

    Ok(Response::default())
}

pub fn execute_remove_game(
    deps: DepsMut,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info)?;

    let game_addr = deps.api.addr_validate(&addr)?;
    GAMES.remove(deps.storage, game_addr);

    Ok(Response::default())
}

pub fn execute_requeset_funds(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_game(deps.as_ref(), &info)?;

    let config = CONFIG.load(deps.storage)?;

    let contract_balance = deps
        .querier
        .query_balance(&env.contract.address, &config.native_denom)?;

    let coin = Coin {
        amount,
        denom: config.native_denom.clone(),
    };

    // because we need to send exact amount we check if we have
    // enoungh balance to do the transaction with taxes
    let tax = compute_tax(deps.as_ref(), &coin.clone())?;
    let with_tax = amount + tax;
    if contract_balance.amount >= with_tax {
        // sending exact amount
        send_to_game(info.sender.to_string(), coin)
    } else {
        REQUESTING_CONTRACT.save(deps.storage, &(info.sender, coin.amount))?;
        // adding aditional tax because we pay to get funds from anchor
        // and to send them to the game
        //
        // also adding 1 to help with rounding error when converting to the aUST
        let with_tax = with_tax + tax + Uint128::from(1u128);
        let request_amount = with_tax - contract_balance.amount;
        request_from_anchor(deps.as_ref(), &config, request_amount)
    }
}

pub fn execute_deposit_funds(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    only_game(deps.as_ref(), &info)?;

    let config = CONFIG.load(deps.storage)?;

    let contract_balance = deps
        .querier
        .query_balance(&env.contract.address, &config.native_denom)?;
    //Contract_balance is uusd and doesn't include aUST

    if contract_balance.amount > config.threshold {
        //We want to deposit to anchor
        //TODO Add min threshhold 
        let coin = Coin {
            amount: contract_balance.amount - config.threshold,
            denom: config.native_denom.clone(),
        };
        send_to_anchor(deps.as_ref(), &config, coin)
    } else { //the amount we deposited is small enough to keep it in uusd
        let current_total_balance=&query_current_balance(deps.as_ref(), &env).unwrap().balance;
        store_balance_in_state(deps, &env,current_total_balance)?;
        Ok(Response::default())    
    }
}

fn send_to_game(addr: String, coin: Coin) -> Result<Response, ContractError> {
    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: addr,
        amount: vec![coin.clone()],
    });
    Ok(Response::new()
        .add_attribute("action", "send to game")
        .add_message(msg))
}

fn store_balance_in_state(deps: DepsMut, env: &Env, amount: &Uint128) -> Result<Response, ContractError>{
    let height: u64 =u64::from_le(env.block.height.try_into().unwrap());
    
    BALANCE_HISTORY.save(deps.storage,  cw_storage_plus::U64Key::from(height), &amount)?;
    //BALANCE_HISTORY.save(deps.storage, env.block.height , &amount)?;
    Ok(Response::default())
}

fn request_from_anchor(
    deps: Deps,
    config: &Config,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // converting amount from uusd into aUST
    let exchange_rate = anchor_exchange_rate(deps, config.anchor_market_address.to_string())?;
    let amount = Uint256::from(amount) / exchange_rate;

    let msg = AnchorHookMsg::RedeemStable {};
    let msg = Cw20ExecuteMsg::Send {
        amount: amount.into(),
        contract: config.anchor_market_address.to_string(),
        msg: to_binary(&msg)?,
    };
    let msg = WasmMsg::Execute {
        contract_addr: config.anchor_token_address.to_string(),
        msg: to_binary(&msg)?,
        funds: vec![],
    };
    let msg = SubMsg::reply_on_success(msg, 1);
    Ok(Response::new()
        .add_attribute("action", "request from anchor")
        .add_submessage(msg))
}

fn send_to_anchor(deps: Deps, config: &Config, coin: Coin) -> Result<Response, ContractError> {
    let without_tax = deduct_tax(deps, coin)?;
    let msg = WasmMsg::Execute {
        contract_addr: config.anchor_market_address.to_string(),
        funds: vec![without_tax],
        msg: to_binary(&AnchorMsg::DepositStable {})?,
    };
    Ok(Response::new()
        .add_attribute("action", "send to anchor")
        .add_message(msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        1 => {
            let (contract_addr, amount) = REQUESTING_CONTRACT.load(deps.storage)?;
            let config = CONFIG.load(deps.storage)?;
            send_to_game(
                contract_addr.to_string(),
                Coin {
                    amount,
                    denom: config.native_denom,
                },
            )
        }
        _ => Err(ContractError::InvalidReplyId {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::CurrentBalance {} => to_binary(&query_current_balance(deps, &env)?),
        QueryMsg::GetThreshold {} => to_binary(&query_get_threshold(deps)?),
        QueryMsg::ListGames {} => to_binary(&query_list_games(deps)?),
        QueryMsg::BalanceHistory {} => to_binary(&query_balance_history(deps)?),
    }
}

fn query_current_balance(deps: Deps, env: &Env) -> StdResult<CurrentBalance> {
    let config = CONFIG.load(deps.storage)?;
    let native_balance = deps
        .querier
        .query_balance(&env.contract.address, &config.native_denom)?
        .amount;

    let anchor_balance = anchor_balance(
        deps,
        env.contract.address.to_string(),
        config.anchor_token_address.to_string(),
    )?;

    // converting amount from aUST into uusd
    let exchange_rate = anchor_exchange_rate(deps, config.anchor_market_address.to_string())?;
    let anchor_balance: Uint128 = (Uint256::from(anchor_balance) * exchange_rate).into();

    Ok(CurrentBalance {
        balance: native_balance + anchor_balance,
    })
}

fn query_get_threshold(deps: Deps) -> StdResult<Threshold> {
    let threshold = CONFIG.load(deps.storage)?.threshold;
    Ok(Threshold { threshold })
}

fn query_list_games(deps: Deps) -> StdResult<Games> {
    let games = GAMES
        .keys(deps.storage, None, None, Order::Ascending)
        .map(|game| String::from_utf8(game).unwrap())
        .collect();
    Ok(Games { games })
}
//fn query_balance_history(deps: Deps) -> StdResult<BALANCEHISTORY> {
//    Ok(BALANCEHISTORY {
//        balanceHistory: BALANCE_HISTORY
//            .range(deps.storage, None, None, Order::Ascending)
//            .collect::<StdResult<Vec<_>>>()?
//            .into_iter()
//            .map(|(timePoint, balance)| (u64::from_be_bytes(timePoint[0].try_into().unwrap()), balance))
//            .collect::<Vec<_>>(),
//    })
//}
fn query_balance_history(deps: Deps) -> StdResult<BALANCEHISTORY> {
    let balance_history = BALANCE_HISTORY
    .range(deps.storage, None, None, Order::Ascending)
    .map(|time_point| (u64::from_be_bytes(time_point.as_ref().unwrap().0[0..8].try_into().unwrap()) , time_point.unwrap().1))
    //.map(|time_point| (time_point.as_ref() , time_point.unwrap().1))
    .collect::<Vec<_>>();

    Ok( BALANCEHISTORY { balance_history})






//    Ok(BALANCEHISTORY {
//        balanceHistory: BALANCE_HISTORY
//            .range(deps.storage, None, None, Order::Ascending)
//            .collect::<StdResult<Vec<_>>>()?
//            .into_iter()
//            .map(|(timePoint, balance)| (u64::from_be_bytes(timePoint[0].try_into().unwrap()), balance))
//            .collect::<Vec<_>>(),
//    })
}



#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            gov_contract_address: "gov-contract".to_string(),
            anchor_market_address: "anchor-market".to_string(),
            anchor_token_address: "anchor-token".to_string(),
            threshold: 1000u128.into(),
            native_denom: "uusd".to_string(),
        };
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert!(res.messages.is_empty());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetThreshold {}).unwrap();
        let value: Threshold = from_binary(&res).unwrap();
        assert_eq!(
            value,
            Threshold {
                threshold: 1000u128.into(),
            }
        );
    }

    #[test]
    fn set_threshold() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            gov_contract_address: "gov-contract".to_string(),
            anchor_market_address: "anchor-market".to_string(),
            anchor_token_address: "anchor-token".to_string(),
            threshold: 1000u128.into(),
            native_denom: "uusd".to_string(),
        };
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert!(res.messages.is_empty());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetThreshold {}).unwrap();
        let value: Threshold = from_binary(&res).unwrap();
        assert_eq!(
            value,
            Threshold {
                threshold: 1000u128.into(),
            }
        );

        let env = mock_env();
        let user_info = mock_info("user", &[]);
        let msg = ExecuteMsg::ChangeThreshold {
            threshold: 69u128.into(),
        };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg);
        assert_eq!(res, Err(ContractError::Unauthorized {}));

        let env = mock_env();
        let user_info = mock_info("creator", &[]);
        let msg = ExecuteMsg::ChangeThreshold {
            threshold: 69u128.into(),
        };
        let _res = execute(deps.as_mut(), env.clone(), user_info, msg).unwrap();

        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetThreshold {}).unwrap();
        let value: Threshold = from_binary(&res).unwrap();
        assert_eq!(
            value,
            Threshold {
                threshold: 69u128.into(),
            }
        );
    }

    #[test]
    fn add_game() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            gov_contract_address: "gov-contract".to_string(),
            anchor_market_address: "anchor-market".to_string(),
            anchor_token_address: "anchor-token".to_string(),
            threshold: 1000u128.into(),
            native_denom: "uusd".to_string(),
        };
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert!(res.messages.is_empty());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::ListGames {}).unwrap();
        let value: Games = from_binary(&res).unwrap();
        assert_eq!(value, Games { games: vec![] });

        let env = mock_env();

        let user_info = mock_info("user", &[]);
        let msg = ExecuteMsg::AddGame {
            addr: "game1".to_string(),
        };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg);
        assert_eq!(res, Err(ContractError::Unauthorized {}));

        let user_info = mock_info("creator", &[]);
        let msg = ExecuteMsg::AddGame {
            addr: "game1".to_string(),
        };
        let _res = execute(deps.as_mut(), env.clone(), user_info, msg).unwrap();

        let res = query(deps.as_ref(), mock_env(), QueryMsg::ListGames {}).unwrap();
        let value: Games = from_binary(&res).unwrap();
        assert_eq!(
            value,
            Games {
                games: vec!["game1".to_string()],
            }
        );
    }

    use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::OwnedDeps;
    use terra_cosmwasm::TerraQueryWrapper;
    fn custom_deps() -> OwnedDeps<MockStorage, MockApi, MockQuerier<TerraQueryWrapper>> {
        use cosmwasm_std::testing::MockQuerierCustomHandlerResult;
        use cosmwasm_std::{ContractResult, Decimal};
        use terra_cosmwasm::{TaxCapResponse, TaxRateResponse, TerraQuery};

        let querier = MockQuerier::new(&[]).with_custom_handler(
            |q: &TerraQueryWrapper| -> MockQuerierCustomHandlerResult {
                let res = match q.query_data {
                    TerraQuery::TaxRate {} => to_binary(&TaxRateResponse {
                        rate: Decimal::zero(),
                    })
                    .unwrap(),
                    TerraQuery::TaxCap { .. } => to_binary(&TaxCapResponse {
                        cap: 1000u128.into(),
                    })
                    .unwrap(),
                    _ => unreachable!(),
                };
                MockQuerierCustomHandlerResult::Ok(ContractResult::Ok(res))
            },
        );

        OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier,
        }
    }

    #[test]
    fn request_funds() {
        let mut deps = custom_deps();

        let msg = InstantiateMsg {
            gov_contract_address: "gov-contract".to_string(),
            anchor_market_address: "anchor-market".to_string(),
            anchor_token_address: "anchor-token".to_string(),
            threshold: 1000u128.into(),
            native_denom: "uusd".to_string(),
        };
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert!(res.messages.is_empty());

        let env = mock_env();

        let user_info = mock_info("creator", &[]);
        let msg = ExecuteMsg::AddGame {
            addr: "game1".to_string(),
        };
        let _res = execute(deps.as_mut(), env.clone(), user_info, msg).unwrap();

        deps.querier.update_balance(
            env.contract.address.clone(),
            vec![Coin {
                denom: "uusd".to_string(),
                amount: 100u128.into(),
            }],
        );

        let user_info = mock_info("user", &[]);
        let msg = ExecuteMsg::RequestFunds {
            amount: 69u128.into(),
        };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg);
        assert_eq!(res, Err(ContractError::Unauthorized {}));

        // if there is enough balance to send to the game
        let game_info = mock_info("game1", &[]);
        let msg = ExecuteMsg::RequestFunds {
            amount: 69u128.into(),
        };
        let res = execute(deps.as_mut(), env.clone(), game_info, msg).unwrap();
        assert_eq!(
            res,
            Response::new()
                .add_attribute("action", "send to game")
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "game1".to_string(),
                    amount: vec![Coin {
                        denom: "uusd".to_string(),
                        amount: 69u128.into(),
                    }],
                }))
        );

        // if we need to request additional funds from Anchor
        let game_info = mock_info("game1", &[]);
        let msg = ExecuteMsg::RequestFunds {
            amount: 200u128.into(),
        };
        let res = execute(deps.as_mut(), env.clone(), game_info, msg).unwrap();
        assert_eq!(res, {
            let msg = AnchorHookMsg::RedeemStable {};
            let msg = Cw20ExecuteMsg::Send {
                amount: 100u128.into(),
                contract: "anchor-market".to_string(),
                msg: to_binary(&msg).unwrap(),
            };
            let msg = WasmMsg::Execute {
                contract_addr: "anchor-token".to_string(),
                msg: to_binary(&msg).unwrap(),
                funds: vec![],
            };
            let msg = SubMsg::reply_on_success(msg, 1);
            Response::new()
                .add_attribute("action", "request from anchor")
                .add_submessage(msg)
        });
    }

    #[test]
    fn deposit_funds() {
        let mut deps = custom_deps();

        let msg = InstantiateMsg {
            gov_contract_address: "gov-contract".to_string(),
            anchor_market_address: "anchor-market".to_string(),
            anchor_token_address: "anchor-token".to_string(),
            threshold: 1000u128.into(),
            native_denom: "uusd".to_string(),
        };
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert!(res.messages.is_empty());

        let env = mock_env();

        let user_info = mock_info("creator", &[]);
        let msg = ExecuteMsg::AddGame {
            addr: "game1".to_string(),
        };
        let _res = execute(deps.as_mut(), env.clone(), user_info, msg).unwrap();

        let user_info = mock_info("user", &[]);
        let msg = ExecuteMsg::DepositFunds {};
        let res = execute(deps.as_mut(), env.clone(), user_info, msg);
        assert_eq!(res, Err(ContractError::Unauthorized {}));

        let game_info = mock_info(
            "game1",
            &[Coin {
                amount: 100u128.into(),
                denom: "uusd".to_string(),
            }],
        );
        let msg = ExecuteMsg::DepositFunds {};
        let res = execute(deps.as_mut(), env.clone(), game_info, msg).unwrap();
        assert!(res.messages.is_empty());

        deps.querier.update_balance(
            env.contract.address.clone(),
            vec![Coin {
                denom: "uusd".to_string(),
                amount: 100u128.into(),
            }],
        );

        let game_info = mock_info(
            "game1",
            &[Coin {
                amount: 2000u128.into(),
                denom: "uusd".to_string(),
            }],
        );

        deps.querier.update_balance(
            env.contract.address.clone(),
            vec![Coin {
                denom: "uusd".to_string(),
                amount: 2100u128.into(),
            }],
        );

        let msg = ExecuteMsg::DepositFunds {};
        let res = execute(deps.as_mut(), env.clone(), game_info, msg).unwrap();
        assert_eq!(res, {
            let msg = WasmMsg::Execute {
                contract_addr: "anchor-market".to_string(),
                funds: vec![Coin {
                    amount: 1100u128.into(),
                    denom: "uusd".to_string(),
                }],
                msg: to_binary(&AnchorMsg::DepositStable {}).unwrap(),
            };
            Response::new()
                .add_attribute("action", "send to anchor")
                .add_message(msg)
        });
    }
}
