use crate::contract::RESERVE_REQUEST_REPLY_ID;
use crate::state::{Config, State, CONFIG, STATE};

use crate::math::decimal_summation_in_256;
use crate::utils::only_owner;
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, SubMsg, Uint128, WasmMsg, WasmQuery,
};
use reserve_contract::msg::{CurrentBalance, ExecuteMsg, QueryMsg};

pub fn query_reserve(deps: &Deps) -> StdResult<Uint128> {
    let msg = QueryMsg::CurrentBalance {};
    let reserve_addr = CONFIG.load(deps.storage)?.reserve_contract_addr;

    let wasm = WasmQuery::Smart {
        contract_addr: reserve_addr.into(),
        msg: to_binary(&msg)?,
    };
    let anchor_balance: CurrentBalance = deps.querier.query(&wasm.into())?;
    Ok(anchor_balance.balance)
}

pub fn calculate_reserve_request_amount(
    reserve_balance: Uint128,
    config: &Config,
    state: &State,
) -> StdResult<Uint128> {
    let distribution_ratio = config.distribution_ratio;
    let last_reserve_residue = state.last_reserve_residue;

    let request_reserve_amount = reserve_balance
        .checked_sub(last_reserve_residue)
        .unwrap_or_else(|_| Uint128::zero())
        .checked_div(Uint128::from(distribution_ratio))?;

    Ok(request_reserve_amount)
}

pub fn update_reserve(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> StdResult<Response> {
    only_owner(deps.as_ref(), &info)?;

    let mut state = STATE.load(deps.storage)?;
    state.last_reserve_residue = amount;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attribute("last_reserve_residue", state.last_reserve_residue))
}

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    governance_token_addr: Option<Addr>,
    reserve_contract_addr: Option<Addr>,
    reward_denom: Option<String>,
    distribution_ratio: Option<u64>,
    unbonding_period: Option<u64>,
) -> StdResult<Response> {
    only_owner(deps.as_ref(), &info)?;

    let mut config = CONFIG.load(deps.storage)?;

    if let Some(g) = governance_token_addr {
        config.governance_token_addr = g;
    };
    if let Some(res) = reserve_contract_addr {
        config.reserve_contract_addr = res;
    };
    if let Some(rew) = reward_denom {
        config.reward_denom = rew;
    };
    if let Some(d) = distribution_ratio {
        config.distribution_ratio = d;
    };
    if let Some(u) = unbonding_period {
        config.unbonding_period = u;
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("governance_token_addr", config.governance_token_addr)
        .add_attribute("reserve_contract_addr", config.reserve_contract_addr)
        .add_attribute("reward_denom", config.reward_denom)
        .add_attribute("distribution_ratio", config.distribution_ratio.to_string())
        .add_attribute("unbonding_period", config.unbonding_period.to_string()))
}

pub fn request_reserve_or_update_global_index(deps: DepsMut, env: Env) -> StdResult<Response> {
    let state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;

    if state.total_balance.is_zero() {
        return Err(StdError::generic_err("No asset is bonded by Hub"));
    }

    let reserve_balance = query_reserve(&deps.as_ref())?;

    let request_reserve_amount =
        calculate_reserve_request_amount(reserve_balance, &config, &state)?;

    if request_reserve_amount > Uint128::zero() {
        let msg = WasmMsg::Execute {
            contract_addr: config.reserve_contract_addr.into(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::RequestFunds {
                amount: request_reserve_amount,
            })?,
        };

        let submessage = SubMsg::reply_on_success(CosmosMsg::Wasm(msg), RESERVE_REQUEST_REPLY_ID);

        return Ok(Response::new().add_submessage(submessage));
    } else {
        return handle_update_global_index(deps, &env);
    }
}

/// Increase global_index according to claimed rewards amount
/// Only hub_contract is allowed to execute
pub fn handle_update_global_index(deps: DepsMut, env: &Env) -> StdResult<Response> {
    let mut state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;

    // anybody can trigger update_global_index
    /*
    if config.lottery_contract != deps.api.canonical_address(&env.message.sender)? {
        return Err(StdError::unauthorized());
    }
     */

    let reward_denom = config.reward_denom;

    // Load the reward contract balance
    let balance = deps
        .querier
        .query_balance(&env.contract.address, reward_denom.as_str())?;

    let previous_balance = state.prev_reward_balance;

    // claimed_rewards = current_balance - prev_balance;
    let claimed_rewards = balance.amount.checked_sub(previous_balance)?;

    state.prev_reward_balance = balance.amount;

    // global_index += claimed_rewards / total_balance;
    state.global_index = decimal_summation_in_256(
        state.global_index,
        Decimal::from_ratio(claimed_rewards, state.total_balance),
    );

    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "update_global_index")
        .add_attribute("claimed_rewards", claimed_rewards))
}
