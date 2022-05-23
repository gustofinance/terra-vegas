use crate::global::{
    calculate_reserve_request_amount, handle_update_global_index, query_reserve,
    request_reserve_or_update_global_index, update_config, update_reserve,
};
use crate::state::{Config, State, CONFIG, OWNER, STATE};
use crate::user::{
    handle_claim_rewards, handle_receive, handle_unbound, query_accrued_rewards, query_holder,
    query_holders,
};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError, StdResult, Uint128,
};

use crate::claim::query_claims;
use terra_vegas::distribution::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ReserveRequestFundsResponse,
    StateResponse,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let conf = Config {
        governance_token_addr: deps
            .api
            .addr_validate(&msg.governance_token_addr.as_str())?,
        reward_denom: msg.reward_denom,
        distribution_ratio: msg.distribution_ratio,
        reserve_contract_addr: deps
            .api
            .addr_validate(&msg.reserve_contract_addr.as_str())?,
        unbonding_period: msg.unbonding_period,
    };

    OWNER.save(deps.storage, &info.sender)?;
    CONFIG.save(deps.storage, &conf)?;
    STATE.save(
        deps.storage,
        &State {
            global_index: Decimal::zero(),
            total_balance: Uint128::zero(),
            prev_reward_balance: Uint128::zero(),
            last_reserve_residue: match msg.initial_reserve_amount {
                Some(v) => v,
                None => Uint128::zero(),
            },
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateReserve { amount } => update_reserve(deps, env, info, amount),
        ExecuteMsg::UpdateConfig {
            governance_token_addr,
            reserve_contract_addr,
            reward_denom,
            distribution_ratio,
            unbonding_period,
        } => update_config(
            deps,
            env,
            info,
            governance_token_addr,
            reserve_contract_addr,
            reward_denom,
            distribution_ratio,
            unbonding_period,
        ),
        ExecuteMsg::ClaimRewards { recipient } => handle_claim_rewards(deps, env, info, recipient),
        ExecuteMsg::UpdateGlobalIndex {} => request_reserve_or_update_global_index(deps, env),
        ExecuteMsg::UnbondStake { address, amount } => {
            handle_unbound(deps, env, info, address, amount)
        }
        // unused
        // ExecuteMsg::WithdrawStake { cap } => handle_withdraw_stake(deps, env, info, cap),
        ExecuteMsg::Receive(msg) => handle_receive(deps, env, info, msg),
    }
}

pub const RESERVE_REQUEST_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> StdResult<Response> {
    match msg.id {
        RESERVE_REQUEST_REPLY_ID => {
            let mut state = STATE.load(deps.storage)?;

            let reserve_balance = query_reserve(&deps.as_ref())?;
            state.last_reserve_residue = reserve_balance;

            STATE.save(deps.storage, &state)?;

            return handle_update_global_index(deps, &env);
        }
        _ => Err(StdError::generic_err("unknown reply id")),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps, _env, msg)?),
        QueryMsg::State {} => to_binary(&query_state(deps, _env, msg)?),
        QueryMsg::AccruedRewards { address } => to_binary(&query_accrued_rewards(deps, address)?),
        QueryMsg::Holder { address } => to_binary(&query_holder(deps, address)?),
        QueryMsg::Holders { start_after, limit } => {
            to_binary(&query_holders(deps, start_after, limit)?)
        }
        QueryMsg::Claims { address } => to_binary(&query_claims(deps, address)?),
        QueryMsg::ReserveRequestFunds {} => to_binary(&query_reserve_funds(deps)?),
    }
}

pub fn query_config(deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse {
        governance_token_addr: config.governance_token_addr.to_string(),
        reward_denom: config.reward_denom,
        unbonding_period: config.unbonding_period,
    })
}
pub fn query_state(deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(StateResponse {
        global_index: state.global_index,
        total_balance: state.total_balance,
        prev_reward_balance: state.prev_reward_balance,
    })
}

pub fn query_reserve_funds(deps: Deps) -> StdResult<ReserveRequestFundsResponse> {
    let state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;

    let reserve_balance = query_reserve(&deps)?;

    let request_reserve_amount =
        calculate_reserve_request_amount(reserve_balance, &config, &state)?;

    Ok(ReserveRequestFundsResponse {
        reserve_request_funds: request_reserve_amount,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
