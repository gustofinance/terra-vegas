use crate::error::ContractError;
use crate::state::{GAMES, OWNER, CONFIG};
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{to_binary, Coin, Deps, MessageInfo, StdResult, Uint128, WasmQuery};
use cw20::{BalanceResponse, Cw20QueryMsg};
use moneymarket::market::{QueryMsg as MarketQueryMsg, StateResponse};
use terra_cosmwasm::TerraQuerier;

pub fn anchor_balance(
    deps: Deps,
    contract_address: String,
    anchor_token_address: String,
) -> StdResult<Uint128> {
    let msg = Cw20QueryMsg::Balance {
        address: contract_address,
    };
    let wasm = WasmQuery::Smart {
        contract_addr: anchor_token_address,
        msg: to_binary(&msg)?,
    };
    let anchor_balance: BalanceResponse = deps.querier.query(&wasm.into())?;
    Ok(anchor_balance.balance)
}

pub fn anchor_exchange_rate(deps: Deps, anchor_market_address: String) -> StdResult<Decimal256> {
    let msg = MarketQueryMsg::State { block_height: None };
    let wasm = WasmQuery::Smart {
        contract_addr: anchor_market_address,
        msg: to_binary(&msg)?,
    };
    let state: StateResponse = deps.querier.query(&wasm.into())?;
    Ok(state.prev_exchange_rate)
}

pub fn only_owner(deps: Deps, info: &MessageInfo) -> Result<(), ContractError> {
    let owner = OWNER.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;

    if info.sender == owner || info.sender == config.gov_contract_address {
        Ok(())
    } else {
        Err(ContractError::Unauthorized {})
    }
}

pub fn only_game(deps: Deps, info: &MessageInfo) -> Result<(), ContractError> {
    match GAMES.may_load(deps.storage, info.sender.clone())? {
        Some(_) => Ok(()),
        None => Err(ContractError::Unauthorized {}),
    }
}

pub fn compute_tax(deps: Deps, coin: &Coin) -> StdResult<Uint128> {
    let terra_querier = TerraQuerier::new(&deps.querier);
    let tax_rate = Decimal256::from((terra_querier.query_tax_rate()?).rate);
    let tax_cap = Uint256::from((terra_querier.query_tax_cap(coin.denom.to_string())?).cap);
    let amount = Uint256::from(coin.amount);
    Ok(std::cmp::min(
        amount * Decimal256::one() - amount / (Decimal256::one() + tax_rate),
        tax_cap,
    )
    .into())
}

pub fn deduct_tax(deps: Deps, coin: Coin) -> StdResult<Coin> {
    let tax_amount = compute_tax(deps, &coin)?;
    Ok(Coin {
        denom: coin.denom,
        amount: coin.amount - tax_amount,
    })
}
