use crate::state::{read_holder, read_holders, store_holder, Config, Holder, State, CONFIG, STATE};

use cosmwasm_std::{
    from_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Response, StdError, StdResult, Uint128,
};

use crate::math::{
    decimal_multiplication_in_256, decimal_subtraction_in_256, decimal_summation_in_256,
};
use crate::taxation::deduct_tax;
use cw20::Cw20ReceiveMsg;
use std::str::FromStr;
use terra_vegas::distribution::{AccruedRewardsResponse, HolderResponse, HoldersResponse};
use terra_vegas::gov::Cw20HookMsg;

pub fn handle_claim_rewards(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: Option<String>,
) -> StdResult<Response> {
    let holder_addr = info.sender;
    let holder_addr_raw = deps.api.addr_canonicalize(&holder_addr.as_str())?;

    let recipient = match recipient {
        Some(value) => deps.api.addr_validate(value.as_str()).unwrap(),
        None => holder_addr.clone(),
    };

    let mut holder: Holder = read_holder(deps.storage, &holder_addr_raw)?;
    let mut state: State = STATE.load(deps.storage)?;
    let config: Config = CONFIG.load(deps.storage)?;

    let reward_with_decimals =
        calculate_decimal_rewards(state.global_index, holder.index, holder.balance)?;

    let all_reward_with_decimals =
        decimal_summation_in_256(reward_with_decimals, holder.pending_rewards);
    let decimals = get_decimals(all_reward_with_decimals).unwrap();

    let rewards = all_reward_with_decimals * Uint128::from(1u128);

    if rewards.is_zero() {
        return Err(StdError::generic_err("No rewards have accrued yet"));
    }
    //let f = state.prev_reward_balance.wrapping_sub(rewards);
    let new_balance = (state.prev_reward_balance.checked_sub(rewards))?;
    state.prev_reward_balance = new_balance;
    STATE.save(deps.storage, &state)?;

    holder.pending_rewards = decimals;
    holder.index = state.global_index;
    store_holder(deps.storage, &holder_addr_raw, &holder)?;

    Ok(Response::new()
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient.to_string(),
            amount: vec![deduct_tax(
                &deps.querier,
                Coin {
                    denom: config.reward_denom,
                    amount: rewards,
                },
            )?],
        }))
        .add_attribute("action", "claim_reward")
        .add_attribute("holder_address", holder_addr)
        .add_attribute("rewards", rewards))
}

pub fn handle_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    // only governance contract can send receieve msg
    if info.sender != config.governance_token_addr {
        return Err(StdError::generic_err(
            "only governance contract can send receive messages",
        ));
    }

    let msg: Cw20HookMsg = from_binary(&wrapper.msg)?;
    match msg {
        Cw20HookMsg::StakeVotingTokens {} => {
            handle_bond(deps, env, info, wrapper.sender, wrapper.amount)
        }
        _ => return Err(StdError::generic_err("only stake message allowed")),
    }
}

pub fn handle_bond(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    holder_addr: String,
    amount: Uint128,
) -> StdResult<Response> {
    if !info.funds.is_empty() {
        return Err(StdError::generic_err("Do not send funds with stake"));
    }
    if amount.is_zero() {
        return Err(StdError::generic_err("Amount required"));
    }

    let address_raw = deps.api.addr_canonicalize(&holder_addr.as_str())?;

    let mut state: State = STATE.load(deps.storage)?;
    let mut holder: Holder = read_holder(deps.storage, &address_raw)?;

    // get decimals
    let rewards = calculate_decimal_rewards(state.global_index, holder.index, holder.balance)?;

    holder.index = state.global_index;
    holder.pending_rewards = decimal_summation_in_256(rewards, holder.pending_rewards);
    holder.balance += amount;
    state.total_balance += amount;

    store_holder(deps.storage, &address_raw, &holder)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "bond_stake")
        .add_attribute("holder_address", holder_addr.as_str())
        .add_attribute("amount", &amount.to_string()))
}

pub fn handle_unbound(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: Addr,
    amount: Uint128,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    // only loterra cw20 contract can send receieve msg
    if info.sender != config.governance_token_addr {
        return Err(StdError::generic_err(
            "only loterra contract can send unbound messages",
        ));
    }

    if !info.funds.is_empty() {
        return Err(StdError::generic_err("Do not send funds with stake"));
    }
    if amount.is_zero() {
        return Err(StdError::generic_err("Amount required"));
    }

    let address_raw = deps.api.addr_canonicalize(&address.as_str())?;

    let mut state: State = STATE.load(deps.storage)?;
    let mut holder: Holder = read_holder(deps.storage, &address_raw)?;
    if holder.balance < amount {
        return Err(StdError::generic_err(format!(
            "Decrease amount cannot exceed user balance: {}",
            holder.balance
        )));
    }

    let rewards = calculate_decimal_rewards(state.global_index, holder.index, holder.balance)?;

    holder.index = state.global_index;
    holder.pending_rewards = decimal_summation_in_256(rewards, holder.pending_rewards);
    holder.balance = holder.balance.checked_sub(amount)?;
    state.total_balance = state.total_balance.checked_sub(amount)?;

    store_holder(deps.storage, &address_raw, &holder)?;
    STATE.save(deps.storage, &state)?;

    // distribution contract reflects governance contract state,
    // there is no need to create claim in this contract as it is handled in the governance contract

    // let release_height = Expiration::AtHeight(env.block.height + config.unbonding_period);
    // create_claim(deps.storage, address_raw, amount, release_height)?;

    Ok(Response::new()
        .add_attribute("action", "unbond_stake")
        .add_attribute("holder_address", info.sender.as_str())
        .add_attribute("amount", &amount.to_string()))
}

// unused
// pub fn handle_withdraw_stake(
//     deps: DepsMut,
//     env: Env,
//     info: MessageInfo,
//     cap: Option<Uint128>,
// ) -> StdResult<Response> {
//     let config = CONFIG.load(deps.storage)?;
//     let address_raw = deps.api.addr_canonicalize(&info.sender.as_str())?;

//     let amount = claim_tokens(deps.storage, address_raw, &env.block, cap)?;
//     if amount.is_zero() {
//         return Err(StdError::generic_err("Wait for the unbonding period"));
//     }

//     let cw20_human_addr = config.governance_token_addr;

//     let cw20_transfer_msg = Cw20ExecuteMsg::Transfer {
//         recipient: info.sender.to_string(),
//         amount,
//     };
//     Ok(Response::new()
//         .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
//             contract_addr: cw20_human_addr.to_string(),
//             msg: to_binary(&cw20_transfer_msg)?,
//             funds: vec![],
//         }))
//         .add_attribute("action", "withdraw_stake")
//         .add_attribute("holder_address", info.sender.as_str())
//         .add_attribute("amount", amount))
// }

pub fn query_accrued_rewards(deps: Deps, address: String) -> StdResult<AccruedRewardsResponse> {
    let global_index = STATE.load(deps.storage)?.global_index;

    let holder: Holder = read_holder(deps.storage, &deps.api.addr_canonicalize(&address)?)?;
    let reward_with_decimals =
        calculate_decimal_rewards(global_index, holder.index, holder.balance)?;
    let all_reward_with_decimals =
        decimal_summation_in_256(reward_with_decimals, holder.pending_rewards);

    let rewards = all_reward_with_decimals * Uint128::from(1u128);

    Ok(AccruedRewardsResponse { rewards })
}

pub fn query_holder(deps: Deps, address: String) -> StdResult<HolderResponse> {
    let holder: Holder = read_holder(deps.storage, &deps.api.addr_canonicalize(&address)?)?;
    Ok(HolderResponse {
        address,
        balance: holder.balance,
        index: holder.index,
        pending_rewards: holder.pending_rewards,
    })
}

pub fn query_holders(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<HoldersResponse> {
    let start_after = if let Some(start_after) = start_after {
        Some(deps.api.addr_validate(&start_after)?)
    } else {
        None
    };

    let holders: Vec<HolderResponse> = read_holders(deps, start_after, limit)?;

    Ok(HoldersResponse { holders })
}

// calculate the reward based on the sender's index and the global index.
fn calculate_decimal_rewards(
    global_index: Decimal,
    user_index: Decimal,
    user_balance: Uint128,
) -> StdResult<Decimal> {
    let decimal_balance = Decimal::from_ratio(user_balance, Uint128::from(1u128));
    Ok(decimal_multiplication_in_256(
        decimal_subtraction_in_256(global_index, user_index),
        decimal_balance,
    ))
}

// calculate the reward with decimal
fn get_decimals(value: Decimal) -> StdResult<Decimal> {
    let stringed: &str = &*value.to_string();
    let parts: &[&str] = &*stringed.split('.').collect::<Vec<&str>>();
    match parts.len() {
        1 => Ok(Decimal::zero()),
        2 => {
            let decimals = Decimal::from_str(&*("0.".to_owned() + parts[1]))?;
            Ok(decimals)
        }
        _ => Err(StdError::generic_err("Unexpected number of dots")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn proper_calculate_rewards() {
        let global_index = Decimal::from_ratio(Uint128::from(9u128), Uint128::from(100u128));
        let user_index = Decimal::zero();
        let user_balance = Uint128::from(1000u128);
        let reward = calculate_decimal_rewards(global_index, user_index, user_balance).unwrap();
        assert_eq!(reward.to_string(), "90");
    }

    #[test]
    pub fn proper_get_decimals() {
        let global_index =
            Decimal::from_ratio(Uint128::from(9999999u128), Uint128::from(100000000u128));
        let user_index = Decimal::zero();
        let user_balance = Uint128::from(10u128);
        let reward = get_decimals(
            calculate_decimal_rewards(global_index, user_index, user_balance).unwrap(),
        )
        .unwrap();
        assert_eq!(reward.to_string(), "0.9999999");
    }
}
