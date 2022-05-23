#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Order, Response, StdResult, Uint128, WasmMsg,
};
use cw0::{must_pay, nonpayable};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;
// use cw_storage_plus::Item;
// use serde::{de::DeserializeOwned, Serialize};

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, IcoInfo, IcoStage, InstantiateMsg, Prices, PricesForAmount, QueryMsg, UserBalance, Whitelist
};
use crate::state::{
    CoinSupply, IcoTimer, PrivateSaleCoinSupply, PublicSaleCoinSupply, SelfLoadAndSave, BALANCES,
    OWNER, PRICE_DENOM, REVENUE_DISTRIBUTION, TIMER, TOKENADDR, WHITELIST,
};
use crate::utils::{
    add_to_whitelist, deduct_tax, only_owner, remove_from_whitelist, set_revenue_distribution,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:ico-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// overshot in percentage => 2%
const PAY_OVERSHOT_PERCENTAGE: u64 = 2;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    OWNER.save(deps.storage, &info.sender)?;
    PRICE_DENOM.save(deps.storage, &msg.price_denom)?;

    // creating coin supply for private and public sales
    let private_coin_supply =
        PrivateSaleCoinSupply::new(msg.privatesale_allocation, msg.privatesale_price);
    private_coin_supply.save(deps.storage)?;

    let public_coin_supply = PublicSaleCoinSupply::new(
        msg.publicsale_allocation,
        msg.publicsale_initial_price,
        msg.publicsale_final_price,
    );
    public_coin_supply.save(deps.storage)?;

    // timer to track ico state
    let timer = IcoTimer::new(msg.privatesale_duration, msg.publicsale_duration);
    TIMER.save(deps.storage, &timer)?;

    // setting revenue distribution
    set_revenue_distribution(&mut deps, &msg.revenue_distribution)?;

    // setting initial whitelist
    add_to_whitelist(&mut deps, &msg.whitelist)?;

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
        ExecuteMsg::AddToWhitelist { addresses } => execute_add_to_whitelist(deps, env, info, addresses),
        ExecuteMsg::RemoveFromWhitelist { addresses } => {
            execute_remove_from_whitelist(deps, info, addresses)
        }
        ExecuteMsg::SetTokenAddress { addr } => execute_set_token_address(deps, info, addr),
        ExecuteMsg::StartIco {} => execute_start_ico(deps, env, info),
        ExecuteMsg::EndIco {} => execute_end_ico(deps, env, info),
        ExecuteMsg::Buy { amount } => execute_buy(deps, env, info, amount),
        ExecuteMsg::ReceiveTokens {} => execute_withdraw_tokens(deps, env, info),
    }
}

pub fn execute_add_to_whitelist(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    addresses: Vec<String>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let timer = TIMER.load(deps.storage)?;
    match timer.current_stage(&env) {
        IcoStage::NotStarted => {
            add_to_whitelist(&mut deps, &addresses)?;
            Ok(Response::default())
        },
        IcoStage::PrivateSale => Err(ContractError::ICOAlreadyStarted {}),
        IcoStage::PublicSale =>Err(ContractError::ICOAlreadyStarted {}),
        IcoStage::Ended => Err(ContractError::ICOEnded {}),
    }
    

    
}

pub fn execute_remove_from_whitelist(
    mut deps: DepsMut,
    info: MessageInfo,
    addresses: Vec<String>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info)?;

    remove_from_whitelist(&mut deps, &addresses)?;
    Ok(Response::default())
}

pub fn execute_set_token_address(
    deps: DepsMut,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info)?;

    let addr = deps.api.addr_validate(&addr)?;
    TOKENADDR.save(deps.storage, &addr)?;
    Ok(Response::new().add_attribute("action", "set token address"))
}

// Starts ICO
pub fn execute_start_ico(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info)?;

    TIMER.update(deps.storage, move |mut t| -> Result<_, ContractError> {
        t.start(&env);
        Ok(t)
    })?;
    Ok(Response::new().add_attribute("action", "ico start"))
}

// Ends ICO and sends coins to specified addresses
pub fn execute_end_ico(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info)?;

    // end ico
    TIMER.update(deps.storage, |mut t| -> Result<_, ContractError> {
        t.end();
        Ok(t)
    })?;

    // calculate how much revenue each address should receive
    let denom = PRICE_DENOM.load(deps.storage)?;
    let contract_balance = deps.querier.query_balance(&env.contract.address, &denom)?;
    let revenue_msgs = REVENUE_DISTRIBUTION
        .range(deps.storage, None, None, Order::Ascending)
        .map(|pair| {
            let (addr, percentage) = pair.unwrap();
            let addr = String::from_utf8(addr).unwrap();

            let payout = Coin {
                denom: denom.clone(),
                amount: contract_balance.amount * percentage,
            };

            CosmosMsg::Bank(BankMsg::Send {
                to_address: addr,
                amount: vec![deduct_tax(deps.as_ref(), payout).unwrap()],
            })
        })
        .collect::<Vec<_>>();

    // burn remain tokens
    let remains = PublicSaleCoinSupply::load(deps.storage)?.remains();
    let burn_msg = burn_coins(&deps, remains)?;

    Ok(Response::new()
        .add_attribute("action", "ico end")
        .add_messages(revenue_msgs)
        .add_message(burn_msg))
}

fn burn_coins(deps: &DepsMut, amount: Uint128) -> StdResult<CosmosMsg> {
    let msg = Cw20ExecuteMsg::Burn { amount };
    let exec = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: TOKENADDR.load(deps.storage)?.to_string(),
        msg: to_binary(&msg)?,
        funds: vec![],
    });
    Ok(exec)
}

pub fn execute_buy(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let timer = TIMER.load(deps.storage)?;
    match timer.current_stage(&env) {
        IcoStage::NotStarted => Err(ContractError::ICONotStarted {}),
        IcoStage::PrivateSale => {
            WHITELIST
                .may_load(deps.storage, info.sender.clone())?
                .ok_or(ContractError::NotWhitelisted {})?;
            buy::<PrivateSaleCoinSupply>(deps, env, info, amount, timer)
        }
        IcoStage::PublicSale => buy::<PublicSaleCoinSupply>(deps, env, info, amount, timer),
        IcoStage::Ended => Err(ContractError::ICOEnded {}),
    }
}

// This method represents shared logic for public and private sale
// It checks ramaining coin balance, amount of received coins
// and calculates price for requested amount.
// Amount of received coins must be equal to total purchase price, otherwise
// operation is aborted.
pub fn buy<C: CoinSupply + SelfLoadAndSave>(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    mut timer: IcoTimer,
) -> Result<Response, ContractError> {
    let mut coin_supply = C::load(deps.storage)?;

    if coin_supply.remains() < amount {
        Err(ContractError::NotEnoughCoinsLeft {
            remains: coin_supply.remains(),
            requested: amount,
        })
    } else {
        let price_denom = PRICE_DENOM.load(deps.storage)?;
        let provided_funds = must_pay(&info, &price_denom)?;
        let required_funds = coin_supply.price_for_amount(amount);
        let max_allowed_funds =
            required_funds * (Decimal::one() + Decimal::percent(PAY_OVERSHOT_PERCENTAGE));
        if provided_funds < required_funds || provided_funds > max_allowed_funds {
            return Err(ContractError::IncorrectAmountOfFunds {
                provided: provided_funds,
                required: required_funds,
                max_allowed: max_allowed_funds,
            });
        }
        BALANCES.update(deps.storage, info.sender, |balance| -> StdResult<_> {
            Ok(balance.unwrap_or_default() + amount)
        })?;

        coin_supply.record_sale(amount);
        coin_supply.save(deps.storage)?;

        if coin_supply.remains().is_zero() {
            timer.move_to_next_stage(&env);
            TIMER.save(deps.storage, &timer)?;
        }

        Ok(Response::new().add_attribute("action", "buy"))
    }
}

pub fn execute_withdraw_tokens(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    match TIMER.load(deps.storage)?.current_stage(&env) {
        IcoStage::NotStarted | IcoStage::PrivateSale | IcoStage::PublicSale => {
            Err(ContractError::CanNotWithdrawUntilICOEnd {})
        }
        IcoStage::Ended => {
            let user_balance = BALANCES.load(deps.storage, info.sender.clone())?;
            if user_balance.is_zero() {
                Ok(Response::new()
                    .add_attribute("action", "withdraw tokens")
                    .add_attribute("amount", "none"))
            } else {
                BALANCES.save(deps.storage, info.sender.clone(), &Uint128::zero())?;
                let msg = send_coins(&deps, &info.sender, user_balance)?;
                Ok(Response::new()
                    .add_attribute("action", "withdraw tokens")
                    .add_attribute("amount", user_balance.to_string())
                    .add_message(msg))
            }
        }
    }
}

fn send_coins(deps: &DepsMut, addr: &Addr, amount: Uint128) -> StdResult<CosmosMsg> {
    let msg = Cw20ExecuteMsg::Transfer {
        recipient: addr.into(),
        amount,
    };
    let exec = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: TOKENADDR.load(deps.storage)?.to_string(),
        msg: to_binary(&msg)?,
        funds: vec![],
    });
    Ok(exec)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::IcoInfo {} => to_binary(&query_info(deps, env)?),
        QueryMsg::Prices {} => to_binary(&query_prices(deps)?),
        QueryMsg::PricesForAmount { amount } => to_binary(&query_prices_for_amount(deps, amount)?),
        QueryMsg::Balance { addr } => to_binary(&query_balance(deps, addr)?),
        QueryMsg::IcoTimes {} => to_binary(&query_ico_times(deps)?),
        QueryMsg::Whitelist {} => to_binary(&query_whitelist(deps)?),
    }
}

fn query_info(deps: Deps, env: Env) -> StdResult<IcoInfo> {
    let stage = TIMER.load(deps.storage).unwrap().current_stage(&env);
    let info = match stage {
        IcoStage::NotStarted | IcoStage::PrivateSale => {
            let private_coins_remains = PrivateSaleCoinSupply::load(deps.storage)?.remains();
            // when PublicSaleCoinSupply is read from the storage it adds remains of the private
            // sale, so we need to subtract private_remains to get the correct value
            let public_coins_remains =
                PublicSaleCoinSupply::load(deps.storage)?.remains() - private_coins_remains;
            IcoInfo {
                stage,
                private_coins_remains,
                public_coins_remains,
            }
        }
        IcoStage::PublicSale => IcoInfo {
            stage,
            // in public sale stage all unsold coins from private sale are moved into public sale
            // pool, so private_remains become 0
            private_coins_remains: Uint128::zero(),
            public_coins_remains: PublicSaleCoinSupply::load(deps.storage)?.remains(),
        },
        IcoStage::Ended => IcoInfo {
            stage,
            // after ICO ends it burns all remaining coins
            private_coins_remains: Uint128::zero(),
            public_coins_remains: Uint128::zero(),
        },
    };
    Ok(info)
}
fn query_ico_times(deps: Deps) -> StdResult<crate::msg::IcoTimes> {
    Ok(crate::msg::IcoTimes {
        start_time: TIMER.load(deps.storage)?.get_start_ime(),
        private_end: TIMER.load(deps.storage)?.get_private_end(),
        public_end: TIMER.load(deps.storage)?.get_public_end(),

    })
}


fn query_prices(deps: Deps) -> StdResult<Prices> {
    Ok(Prices {
        private_sale_price: PrivateSaleCoinSupply::load(deps.storage)?.price(),
        public_sale_price: PublicSaleCoinSupply::load(deps.storage)?.price(),
    })
}

fn query_prices_for_amount(deps: Deps, amount: Uint128) -> StdResult<PricesForAmount> {
    Ok(PricesForAmount {
        amount,
        private_sale_price: PrivateSaleCoinSupply::load(deps.storage)?.price_for_amount(amount),
        public_sale_price: PublicSaleCoinSupply::load(deps.storage)?.price_for_amount(amount),
    })
}

fn query_balance(deps: Deps, addr: String) -> StdResult<UserBalance> {
    let addr = deps.api.addr_validate(&addr)?;
    let balance = BALANCES.may_load(deps.storage, addr)?;
    Ok(UserBalance { balance })
}

fn query_whitelist(deps: Deps) -> StdResult<Whitelist> {
    let whitelist = WHITELIST.keys(deps.storage, None,None, Order::Ascending)
        .map(|address| String::from_utf8(address).unwrap())
        .collect();
    Ok(Whitelist { whitelist })

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::RevenuePercentage;
    use crate::state::PublicSaleCoinSupplyData;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Decimal};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            privatesale_allocation: 100u128.into(),
            privatesale_price: 1u128.into(),
            privatesale_duration: 10,
            publicsale_allocation: 200u128.into(),
            publicsale_initial_price: 1u128.into(),
            publicsale_final_price: 5u128.into(),
            publicsale_duration: 20,
            price_denom: "uusd".to_string(),
            revenue_distribution: vec![
                RevenuePercentage {
                    addr: "user1".to_string(),
                    percentage: "0.1".to_string(),
                },
                RevenuePercentage {
                    addr: "user2".to_string(),
                    percentage: "0.5".to_string(),
                },
            ],
            whitelist: vec![],
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();
        let res = instantiate(deps.as_mut(), env, info.clone(), msg.clone());
        assert_eq!(
            res,
            Err(ContractError::InvalidTotalDistributionPercentages {
                expected: Decimal::one(),
                got: Decimal::percent(60),
            })
        );

        let msg = InstantiateMsg {
            privatesale_allocation: 100u128.into(),
            privatesale_price: 1u128.into(),
            privatesale_duration: 10,
            publicsale_allocation: 200u128.into(),
            publicsale_initial_price: 1u128.into(),
            publicsale_final_price: 5u128.into(),
            publicsale_duration: 20,
            price_denom: "uusd".to_string(),
            revenue_distribution: vec![
                RevenuePercentage {
                    addr: "user1".to_string(),
                    percentage: "0.5".to_string(),
                },
                RevenuePercentage {
                    addr: "user2".to_string(),
                    percentage: "0.5".to_string(),
                },
            ],
            whitelist: vec![],
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();

        let res = instantiate(deps.as_mut(), env, info.clone(), msg.clone()).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(info.sender, OWNER.load(&deps.storage).unwrap());
        assert_eq!(
            PrivateSaleCoinSupply {
                total_amount: msg.privatesale_allocation,
                coins_sold: 0u128.into(),
                coin_price: msg.privatesale_price,
            },
            PrivateSaleCoinSupply::load(&deps.storage).unwrap()
        );
        assert_eq!(
            PublicSaleCoinSupply {
                // because we inherited coins from
                inherited: msg.privatesale_allocation,
                data: PublicSaleCoinSupplyData {
                    total_amount: msg.publicsale_allocation,
                    coins_sold: 0u128.into(),
                    coin_price_start: msg.publicsale_initial_price,
                    coin_price_end: msg.publicsale_final_price,
                }
            },
            PublicSaleCoinSupply::load(&deps.storage).unwrap()
        );

        let res = query(deps.as_ref(), mock_env(), QueryMsg::IcoInfo {}).unwrap();
        let value: IcoInfo = from_binary(&res).unwrap();
        assert_eq!(
            IcoInfo {
                stage: IcoStage::NotStarted,
                private_coins_remains: msg.privatesale_allocation,
                public_coins_remains: msg.publicsale_allocation,
            },
            value
        );
    }

    #[test]
    fn start_and_state_change() {
        let mut deps = mock_dependencies(&[]);

        let init_msg = InstantiateMsg {
            privatesale_allocation: 100u128.into(),
            privatesale_price: 1u128.into(),
            privatesale_duration: 10,
            publicsale_allocation: 200u128.into(),
            publicsale_initial_price: 1u128.into(),
            publicsale_final_price: 5u128.into(),
            publicsale_duration: 20,
            price_denom: "uusd".to_string(),
            revenue_distribution: vec![RevenuePercentage {
                addr: "user1".to_string(),
                percentage: "1".to_string(),
            }],
            whitelist: vec![],
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();
        let _res = instantiate(deps.as_mut(), env.clone(), info, init_msg.clone()).unwrap();

        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::StartIco {};
        let res = execute(deps.as_mut(), env.clone(), info, msg);
        assert_eq!(res, Err(ContractError::Unauthorized {}));

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::StartIco {};
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let res = query(deps.as_ref(), env.clone(), QueryMsg::IcoInfo {}).unwrap();
        let value: IcoInfo = from_binary(&res).unwrap();
        assert_eq!(
            IcoInfo {
                stage: IcoStage::PrivateSale,
                private_coins_remains: init_msg.privatesale_allocation,
                public_coins_remains: init_msg.publicsale_allocation,
            },
            value
        );

        // user buys all tokens in private sale
        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::AddToWhitelist {
            addresses: vec!["user".to_string()],
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(res, Response::default(),);

        let info = mock_info("user", &coins(100, "uusd"));
        let msg = ExecuteMsg::Buy {
            amount: 100u128.into(),
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(res, Response::new().add_attribute("action", "buy"));

        let res = query(deps.as_ref(), env.clone(), QueryMsg::IcoInfo {}).unwrap();
        let value: IcoInfo = from_binary(&res).unwrap();
        assert_eq!(
            IcoInfo {
                stage: IcoStage::PublicSale,
                private_coins_remains: Uint128::zero(),
                public_coins_remains: init_msg.publicsale_allocation,
            },
            value
        );

        // user buys all tokens in public sale
        let info = mock_info("user", &coins(600, "uusd"));
        let msg = ExecuteMsg::Buy {
            amount: 200u128.into(),
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(res, Response::new().add_attribute("action", "buy"));

        let res = query(deps.as_ref(), env.clone(), QueryMsg::IcoInfo {}).unwrap();
        let value: IcoInfo = from_binary(&res).unwrap();
        assert_eq!(
            IcoInfo {
                stage: IcoStage::Ended,
                private_coins_remains: Uint128::zero(),
                public_coins_remains: Uint128::zero(),
            },
            value
        );

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::SetTokenAddress {
            addr: "token_addr".to_string(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    }

    #[test]
    fn start_end_and_send_funds() {
        use cosmwasm_std::testing::{
            MockApi, MockQuerier, MockQuerierCustomHandlerResult, MockStorage,
        };
        use cosmwasm_std::{ContractResult, OwnedDeps};
        use terra_cosmwasm::{TaxCapResponse, TaxRateResponse, TerraQuery, TerraQueryWrapper};

        let querier = MockQuerier::new(&[]).with_custom_handler(
            |q: &TerraQueryWrapper| -> MockQuerierCustomHandlerResult {
                let res = match q.query_data {
                    TerraQuery::TaxRate {} => to_binary(&TaxRateResponse {
                        rate: Decimal::zero(),
                    })
                    .unwrap(),
                    TerraQuery::TaxCap { .. } => to_binary(&TaxCapResponse {
                        cap: 100u128.into(),
                    })
                    .unwrap(),
                    _ => unreachable!(),
                };
                MockQuerierCustomHandlerResult::Ok(ContractResult::Ok(res))
            },
        );

        let mut deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier,
        };

        let init_msg = InstantiateMsg {
            privatesale_allocation: 100u128.into(),
            privatesale_price: 1u128.into(),
            privatesale_duration: 10,
            publicsale_allocation: 200u128.into(),
            publicsale_initial_price: 1u128.into(),
            publicsale_final_price: 5u128.into(),
            publicsale_duration: 20,
            price_denom: "uusd".to_string(),
            revenue_distribution: vec![
                RevenuePercentage {
                    addr: "user1".to_string(),
                    percentage: "0.5".to_string(),
                },
                RevenuePercentage {
                    addr: "user2".to_string(),
                    percentage: "0.5".to_string(),
                },
            ],
            whitelist: vec![],
        };
        let info = mock_info("creator", &[]);
        let mut env = mock_env();
        let _res = instantiate(deps.as_mut(), env.clone(), info, init_msg.clone()).unwrap();

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::SetTokenAddress {
            addr: "token_addr".to_string(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::StartIco {};
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        env.block.time = env.block.time.plus_seconds(init_msg.privatesale_duration);

        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::PricesForAmount {
                amount: 100u128.into(),
            },
        )
        .unwrap();
        let value: PricesForAmount = from_binary(&res).unwrap();
        assert_eq!(
            PricesForAmount {
                amount: 100u128.into(),
                private_sale_price: 100u128.into(),
                public_sale_price: 166u128.into(),
            },
            value
        );

        let info = mock_info("user", &coins(166, "uusd"));
        let msg = ExecuteMsg::Buy {
            amount: 100u128.into(),
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(res, Response::new().add_attribute("action", "buy"));

        let res = query(deps.as_ref(), env.clone(), QueryMsg::IcoInfo {}).unwrap();
        let value: IcoInfo = from_binary(&res).unwrap();
        assert_eq!(
            IcoInfo {
                stage: IcoStage::PublicSale,
                private_coins_remains: Uint128::zero(),
                public_coins_remains: 200u128.into(),
            },
            value
        );

        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::PricesForAmount {
                amount: 100u128.into(),
            },
        )
        .unwrap();
        let value: PricesForAmount = from_binary(&res).unwrap();
        assert_eq!(
            PricesForAmount {
                amount: 100u128.into(),
                private_sale_price: 100u128.into(),
                public_sale_price: 299u128.into(),
            },
            value
        );

        deps.querier.update_balance(
            env.contract.address.clone(),
            vec![Coin {
                denom: "uusd".to_string(),
                amount: 100u128.into(),
            }],
        );

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::EndIco {};
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(
            res,
            Response::new()
                .add_attribute("action", "ico end")
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "user1".to_string(),
                    amount: vec![Coin {
                        denom: "uusd".to_string(),
                        amount: 50u128.into(),
                    }],
                }))
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "user2".to_string(),
                    amount: vec![Coin {
                        denom: "uusd".to_string(),
                        amount: 50u128.into(),
                    }],
                }))
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "token_addr".to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Burn {
                        amount: 200u128.into()
                    })
                    .unwrap(),
                    funds: vec![],
                }))
        );

        let res = query(deps.as_ref(), env.clone(), QueryMsg::IcoInfo {}).unwrap();
        let value: IcoInfo = from_binary(&res).unwrap();
        assert_eq!(
            IcoInfo {
                stage: IcoStage::Ended,
                private_coins_remains: Uint128::zero(),
                public_coins_remains: Uint128::zero(),
            },
            value
        );
    }

    #[test]
    fn add_remove_whitelist() {
        let mut deps = mock_dependencies(&[]);

        let init_msg = InstantiateMsg {
            privatesale_allocation: 100u128.into(),
            privatesale_price: 1u128.into(),
            privatesale_duration: 10,
            publicsale_allocation: 200u128.into(),
            publicsale_initial_price: 1u128.into(),
            publicsale_final_price: 5u128.into(),
            publicsale_duration: 20,
            price_denom: "uusd".to_string(),
            revenue_distribution: vec![RevenuePercentage {
                addr: "user1".to_string(),
                percentage: "1".to_string(),
            }],
            whitelist: vec![],
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();

        let _res = instantiate(deps.as_mut(), env.clone(), info, init_msg.clone()).unwrap();

        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::AddToWhitelist {
            addresses: vec!["1234".to_string()],
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg);
        assert_eq!(res, Err(ContractError::Unauthorized {}));

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::AddToWhitelist {
            addresses: vec!["1234".to_string()],
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(res, Response::default(),);

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::AddToWhitelist {
            addresses: vec!["1234".to_string()],
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(res, Response::default(),);

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::RemoveFromWhitelist {
            addresses: vec!["1234".to_string()],
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(res, Response::default(),);

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::RemoveFromWhitelist {
            addresses: vec!["1234".to_string()],
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(res, Response::default(),);
    }

    #[test]
    fn buy_and_withdraw() {
        let mut deps = mock_dependencies(&[]);

        let init_msg = InstantiateMsg {
            privatesale_allocation: 0u128.into(),
            privatesale_price: 1u128.into(),
            privatesale_duration: 10,
            publicsale_allocation: 200u128.into(),
            publicsale_initial_price: 1u128.into(),
            publicsale_final_price: 5u128.into(),
            publicsale_duration: 20,
            price_denom: "uusd".to_string(),
            revenue_distribution: vec![RevenuePercentage {
                addr: "user1".to_string(),
                percentage: "1".to_string(),
            }],
            whitelist: vec![],
        };
        let info = mock_info("creator", &[]);
        let mut env = mock_env();

        let _res = instantiate(deps.as_mut(), env.clone(), info, init_msg.clone()).unwrap();

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::SetTokenAddress {
            addr: "token_addr".to_string(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::StartIco {};
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        env.block.time = env.block.time.plus_seconds(init_msg.privatesale_duration);

        let info = mock_info("user", &[]);
        let msg = ExecuteMsg::Buy {
            amount: 100u128.into(),
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg);
        assert_eq!(
            res,
            Err(ContractError::Payment(cw0::PaymentError::NoFunds {}))
        );

        // testing witn overpay
        let info = mock_info("user", &coins(204, "uusd"));
        let msg = ExecuteMsg::Buy {
            amount: 100u128.into(),
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(res, Response::new().add_attribute("action", "buy"));

        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Balance {
                addr: "user".to_string(),
            },
        )
        .unwrap();
        let value: UserBalance = from_binary(&res).unwrap();
        assert_eq!(
            UserBalance {
                balance: Some(100u128.into())
            },
            value
        );

        let info = mock_info("user", &coins(100, "uusd"));
        let msg = ExecuteMsg::Buy {
            amount: 100u128.into(),
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg);
        assert_eq!(
            res,
            Err(ContractError::IncorrectAmountOfFunds {
                provided: 100u128.into(),
                required: 400u128.into(),
                max_allowed: 408u128.into(),
            })
        );

        let info = mock_info("user", &[]);
        let msg = ExecuteMsg::ReceiveTokens {};
        let res = execute(deps.as_mut(), env.clone(), info, msg);
        assert_eq!(res, Err(ContractError::CanNotWithdrawUntilICOEnd {}));

        env.block.time = env.block.time.plus_seconds(init_msg.publicsale_duration);

        let info = mock_info("user", &[]);
        let msg = ExecuteMsg::ReceiveTokens {};
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(
            res,
            Response::new()
                .add_attribute("action", "withdraw tokens")
                .add_attribute("amount", "100")
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "token_addr".to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "user".to_string(),
                        amount: 100u128.into(),
                    })
                    .unwrap(),
                    funds: vec![],
                }))
        );
        let info = mock_info("user", &[]);
        let msg = ExecuteMsg::ReceiveTokens {};
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(
            res,
            Response::new()
                .add_attribute("action", "withdraw tokens")
                .add_attribute("amount", "none")
        );
    }

    #[test]
    fn price_change() {
        let mut coin_supply =
            PublicSaleCoinSupply::new(10000u128.into(), 1u128.into(), 5u128.into());

        // ((1 + 4 * 0.1) + (1))/2 = 1.2
        assert_eq!(
            coin_supply.price_for_amount(1000u128.into()),
            1200u128.into()
        );

        coin_supply.record_sale(1000u128.into());
        // ((1 + 4 * 0.1) + (1 + 4 * 0.2))/2 = 1.6
        assert_eq!(
            coin_supply.price_for_amount(1000u128.into()),
            1600u128.into()
        );

        coin_supply.record_sale(1000u128.into());
        // ((1 + 4 * 0.2) + (1 + 4 * 0.201))/2 = 1.802
        assert_eq!(coin_supply.price_for_amount(10u128.into()), 18u128.into());
        // ((1 + 4 * 0.1) + (1 + 4 * 0.2001))/2 = 1.8002
        assert_eq!(coin_supply.price_for_amount(1u128.into()), 1u128.into());
    }
}
