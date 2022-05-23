#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, Decimal, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdResult, Uint128, Uint64, WasmMsg,
};
use cw0::{must_pay, nonpayable};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
use reserve_contract::msg::ExecuteMsg as ReserveMsg;
use terra_vegas::common::MigrateMsg;

use crate::error::ContractError;
use crate::msg::{
    Bets, BettingLimit, CurrentRound, ExecuteMsg, InstantiateMsg, OutcomeHistory, QueryMsg,
    Rewards, TotalRewards, WinCoefficients,
};
use crate::state::{
    CasinoConfig, RoundStatus, RoundTimer, BETS, CASINO_CONFIG, LAST_RANDOMNESS_ROUND,
    OUTCOMES_HISTORY, OWNER, PLAYERS_REWARDS, PLAYER_BETS_ROUNDS, ROUND_TIMER, TOTAL_REWARDS,
};
use std::collections::HashSet;
use std::convert::TryInto;
use std::iter::FromIterator;
use std::ops::Sub;
use std::str::FromStr;

use crate::utils::{
    deduct_tax, get_randomness, get_reserve_balance, get_total_bets_round, only_owner,
    recalculate_win_coefficients,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:game-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    OWNER.save(deps.storage, &info.sender)?;

    let terrand_address = deps.api.addr_validate(&msg.terrand_address)?;
    let gov_contract_address = deps.api.addr_validate(&msg.gov_contract_address)?;
    let reserve_address = deps.api.addr_validate(&msg.reserve_address)?;
    let casino_config = CasinoConfig {
        native_denom: msg.native_denom,
        win_coefficents: recalculate_win_coefficients(&msg.advantage_value)?,
        win_tax: Decimal::one() - Decimal::from_str(&msg.win_tax)?,
        max_number_of_bets: msg.max_number_of_bets,
        max_betting_ratio: msg.max_betting_ratio,
        max_cashflow: msg.max_cashflow,
        terrand_address,
        reserve_address,
        gov_contract_address,
    };
    CASINO_CONFIG.save(deps.storage, &casino_config)?;

    let round_timer = RoundTimer::new(msg.round_duration, env);
    ROUND_TIMER.save(deps.storage, &round_timer)?;

    TOTAL_REWARDS.save(deps.storage, &Uint128::zero())?;

    LAST_RANDOMNESS_ROUND.save(deps.storage, &Uint64::zero())?;

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
        ExecuteMsg::ChangeAdwantageValue { advantage_value } => {
            execute_change_advantage_value(deps, info, advantage_value)
        }
        ExecuteMsg::ChangeWinTax { win_tax } => execute_change_win_tax(deps, info, win_tax),
        ExecuteMsg::ChangeMaxNumberOfBets { number_of_bets } => {
            execute_change_max_number_of_bets(deps, info, number_of_bets)
        }
        ExecuteMsg::ChangeMaxBettingRatio { ratio } => {
            execute_change_max_betting_ratio(deps, info, ratio)
        }
        ExecuteMsg::ChangeRoundDuration { duration } => {
            execute_change_round_duration(deps, info, env, duration)
        }
        ExecuteMsg::ChangeMaxCashflow { cashflow } => {
            execute_change_max_cashflow(deps, info, cashflow)
        }
        ExecuteMsg::Bet { outcome } => execute_bet(deps, env, info, outcome),
        ExecuteMsg::ReceiveRewards {} => execute_receive_rewards(deps, info),
        ExecuteMsg::DrainGame {} => execute_drain_game(deps, info, env),
        ExecuteMsg::StopGame {} => execute_stop_game(deps, info),

        #[cfg(feature = "debug")]
        ExecuteMsg::ChangeConfig {
            native_denom,
            advantage_value,
            win_tax,
            max_number_of_bets,
            max_betting_ratio,
            round_duration,
            max_cashflow,
            terrand_address,
            reserve_address,
        } => execute_change_config(
            deps,
            info,
            env,
            native_denom,
            advantage_value,
            win_tax,
            max_number_of_bets,
            max_betting_ratio,
            round_duration,
            max_cashflow,
            terrand_address,
            reserve_address,
        ),
    }
}

pub fn execute_change_advantage_value(
    deps: DepsMut,
    info: MessageInfo,
    advantage_value: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info)?;

    CASINO_CONFIG.update(
        deps.storage,
        move |mut casino_config| -> Result<_, ContractError> {
            casino_config.win_coefficents = recalculate_win_coefficients(&advantage_value)?;
            Ok(casino_config)
        },
    )?;
    Ok(Response::default())
}

pub fn execute_change_win_tax(
    deps: DepsMut,
    info: MessageInfo,
    win_tax: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info)?;

    CASINO_CONFIG.update(
        deps.storage,
        move |mut casino_config| -> Result<_, ContractError> {
            casino_config.win_tax = Decimal::one() - Decimal::from_str(&win_tax)?;
            Ok(casino_config)
        },
    )?;
    Ok(Response::default())
}

pub fn execute_change_max_number_of_bets(
    deps: DepsMut,
    info: MessageInfo,
    number_of_bets: u64,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info)?;

    CASINO_CONFIG.update(
        deps.storage,
        move |mut casino_config| -> Result<_, ContractError> {
            casino_config.max_number_of_bets = number_of_bets;
            Ok(casino_config)
        },
    )?;
    Ok(Response::default())
}

pub fn execute_change_max_betting_ratio(
    deps: DepsMut,
    info: MessageInfo,
    ratio: u64,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info)?;

    CASINO_CONFIG.update(
        deps.storage,
        move |mut casino_config| -> Result<_, ContractError> {
            casino_config.max_betting_ratio = ratio;
            Ok(casino_config)
        },
    )?;
    Ok(Response::default())
}

pub fn execute_change_round_duration(
    mut deps: DepsMut,
    info: MessageInfo,
    env: Env,
    round_duration: u64,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info)?;

    let mut timer = ROUND_TIMER.load(deps.storage)?;
    let casino_config = CASINO_CONFIG.load(deps.storage)?;

    end_round(&mut deps, &env, &mut timer, &casino_config)?;

    ROUND_TIMER.update(deps.storage, move |mut timer| -> Result<_, ContractError> {
        timer.update_duration(round_duration);
        Ok(timer)
    })?;
    Ok(Response::default())
}

pub fn execute_change_max_cashflow(
    deps: DepsMut,
    info: MessageInfo,
    cashflow: Uint128,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info)?;

    CASINO_CONFIG.update(
        deps.storage,
        move |mut casino_config| -> Result<_, ContractError> {
            casino_config.max_cashflow = cashflow;
            Ok(casino_config)
        },
    )?;
    Ok(Response::default())
}

pub fn execute_drain_game(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info)?;
    let casino_config = CASINO_CONFIG.load(deps.storage)?;

    let contract_balance = deps
        .querier
        .query_balance(&env.contract.address, &casino_config.native_denom)?;
    let reserve_address = casino_config.reserve_address.to_string();

    let msg = WasmMsg::Execute {
        contract_addr: reserve_address,
        funds: vec![deduct_tax(
            deps.as_ref(),
            Coin::new(
                contract_balance.amount.u128(),
                casino_config.native_denom.clone(),
            ),
        )?],
        msg: to_binary(&ReserveMsg::DepositFunds {})?,
    };
    Ok(Response::new().add_message(msg))
}

pub fn execute_stop_game(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info)?;

    ROUND_TIMER.update(deps.storage, move |mut timer| -> Result<_, ContractError> {
        timer.stop();
        Ok(timer)
    })?;
    Ok(Response::default())
}

pub fn execute_change_config(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    native_denom: Option<String>,
    advantage_value: Option<String>,
    win_tax: Option<String>,
    max_number_of_bets: Option<u64>,
    max_betting_ratio: Option<u64>,
    round_duration: Option<u64>,
    max_cashflow: Option<Uint128>,
    terrand_address: Option<String>,
    reserve_address: Option<String>,
    gov_contract_address: Option<String>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info)?;
    let casino_config = CASINO_CONFIG.load(deps.storage)?;

    let new_config = CasinoConfig {
        native_denom: native_denom.unwrap_or(casino_config.native_denom.clone()),
        win_coefficents: advantage_value
            .map_or(Ok(casino_config.win_coefficents.clone()), |av| {
                recalculate_win_coefficients(&av)
            })?,
        win_tax: win_tax.map_or(Ok(casino_config.win_tax.clone()), |win_tax| {
            Decimal::from_str(&win_tax)
        })?,
        max_number_of_bets: max_number_of_bets.unwrap_or_else(|| casino_config.max_number_of_bets),
        max_betting_ratio: max_betting_ratio.unwrap_or_else(|| casino_config.max_betting_ratio),
        max_cashflow: max_cashflow.unwrap_or_else(|| casino_config.max_cashflow),
        terrand_address: terrand_address.map_or_else(
            || Ok(casino_config.terrand_address.clone()),
            |ta| deps.api.addr_validate(&ta),
        )?,
        reserve_address: reserve_address.map_or_else(
            || Ok(casino_config.reserve_address.clone()),
            |ra| deps.api.addr_validate(&ra),
        )?,
        gov_contract_address: gov_contract_address.map_or_else(
            || Ok(casino_config.gov_contract_address.clone()),
            |ga| deps.api.addr_validate(&ga),
        )?,
    };
    CASINO_CONFIG.save(deps.storage, &new_config)?;

    if let Some(rd) = round_duration {
        execute_change_round_duration(deps, info, env, rd)?;
    }

    Ok(Response::default())
}

pub fn execute_bet(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    outcome: u8,
) -> Result<Response, ContractError> {
    // possible outcomes are in range [3..12] inclusive
    if outcome < 3 || 12 < outcome {
        return Err(ContractError::InvalidBetPosition {
            current_position: outcome,
            min_position: 3,
            max_position: 12,
        });
    }

    let mut timer = ROUND_TIMER.load(deps.storage)?;
    let casino_config = CASINO_CONFIG.load(deps.storage)?;
    let mut exist_new_randomness: bool = false;
    let randomness = get_randomness(deps.as_ref(), casino_config.terrand_address.to_string())?;
    // checking if we have new round of randomness
    let last_randomness_round = LAST_RANDOMNESS_ROUND.load(deps.storage)?;
    if randomness.round > last_randomness_round.u64() {
        //There is a new randomness
        exist_new_randomness = true;
    }

    // if round is live just do the bet
    // if round ended(in pending state) then end it and start new one. Then bet as usual.
    let response = match timer.round_status(&env, &exist_new_randomness) {
        RoundStatus::Live => Response::default(),
        RoundStatus::Ready => end_round(&mut deps, &env, &mut timer, &casino_config)?,
        RoundStatus::WaitingOnRandomness => {
            return Err(ContractError::NewRandomnessNotYetAvailable {})
        }
        RoundStatus::Stopped => return Err(ContractError::GameStopped {}),
    };
    let current_round = timer.current_round();

    // only allow specific coins
    let current_bet = must_pay(&info, &casino_config.native_denom)?;

    let reserve_balance =
        get_reserve_balance(deps.as_ref(), casino_config.reserve_address.to_string())?;
    let total_bet_limit = reserve_balance.balance / Uint128::from(casino_config.max_betting_ratio);

    let total_bet = get_total_bets_round(deps.storage, current_round)?;

    let sender = info.sender.clone();
    BETS.update(
        deps.storage,
        (current_round.into(), sender),
        |bets| -> Result<_, ContractError> {
            match bets {
                Some(mut bets) => {
                    if bets.len() as u64 >= casino_config.max_number_of_bets {
                        Err(ContractError::MaxAmountOfBetsThisRound {
                            bets_this_round: (bets.len() as u128).into(),
                            max_bets_per_round: casino_config.max_number_of_bets.into(),
                        })
                    } else {
                        if total_bet + current_bet > total_bet_limit {
                            Err(ContractError::BetAmountExceedsLimit {
                                current_bet,
                                total_bet,
                                total_bet_limit,
                            })
                        } else {
                            bets.push((outcome, current_bet));
                            Ok(bets)
                        }
                    }
                }
                None => {
                    if total_bet + current_bet > total_bet_limit {
                        Err(ContractError::BetAmountExceedsLimit {
                            current_bet,
                            total_bet,
                            total_bet_limit,
                        })
                    } else {
                        Ok(vec![(outcome, current_bet)])
                    }
                }
            }
        },
    )?;

    PLAYER_BETS_ROUNDS.update(
        deps.storage,
        info.sender,
        |bets_rounds| -> Result<_, ContractError> {
            // bets_rounds.insert(value);
            match bets_rounds {
                None => Ok(HashSet::from_iter([current_round])),
                Some(mut rounds) => {
                    rounds.insert(current_round);
                    Ok(rounds)
                }
            }
        },
    )?;

    let random = get_randomness(deps.as_ref(), casino_config.terrand_address.to_string())?;
    LAST_RANDOMNESS_ROUND.save(deps.storage, &random.round.into())?;

    Ok(response)
}

pub fn end_round(
    deps: &mut DepsMut,
    env: &Env,
    timer: &mut RoundTimer,
    casino_config: &CasinoConfig,
) -> Result<Response, ContractError> {
    let randomness = get_randomness(deps.as_ref(), casino_config.terrand_address.to_string())?;
    // checking if we have new round of randomness
    let last_randomness_round = LAST_RANDOMNESS_ROUND.load(deps.storage)?;
    if randomness.round <= last_randomness_round.u64() {
        //This is a duplicate check. has been already done on execute bet
        return Err(ContractError::NewRandomnessNotYetAvailable {});
    }

    let random_data = randomness.randomness.to_array::<32>()?;
    // (random_uint % 6) + 1 is in range [1..6]
    let dice1 = u128::from_be_bytes(random_data[..16].try_into().unwrap()).rem_euclid(6) as u8 + 1;
    let dice2 = u128::from_be_bytes(random_data[16..].try_into().unwrap()).rem_euclid(6) as u8 + 1;

    let random_outcome = dice1 + dice2;
    let current_round = timer.current_round();
    OUTCOMES_HISTORY.save(deps.storage, current_round.into(), &random_outcome)?;

    let players: StdResult<Vec<_>> = BETS
        .prefix(current_round.into())
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    let players = players?;

    let winners: Vec<_> = players
        .iter()
        .filter_map(|(player, bets)| {
            let win = bets
                .iter()
                .fold(Uint128::zero(), |mut sum, (outcome, amount)| {
                    if outcome <= &random_outcome {
                        // the outcome is in range [3..12] and we need coefficients in range [0..9] so
                        // we subtract 3 from the outcome
                        //
                        // we tax the win amount
                        // overall formula is
                        //
                        // win = amount + amount * win_coefficent * win_tax
                        let win_coefficent = casino_config.win_coefficents[(*outcome - 3) as usize];
                        sum += *amount + *amount * win_coefficent * casino_config.win_tax
                    }
                    sum
                });
            if win.is_zero() {
                None
            } else {
                Some((player, win))
            }
        })
        .collect();

    let mut total_win_amount = Uint128::zero();
    for (player, win) in &winners {
        let player = Addr::unchecked(String::from_utf8(player.to_vec()).unwrap());
        PLAYERS_REWARDS.update(deps.storage, player, |reward| -> Result<_, ContractError> {
            // deducing tax from win
            match reward {
                Some(r) => Ok(r + win),
                None => Ok(*win),
            }
        })?;
        total_win_amount += win;
    }

    let total_rewards = TOTAL_REWARDS
        .update(deps.storage, |total_rewards| -> Result<_, ContractError> {
            Ok(total_rewards + total_win_amount)
        })?;

    timer.next_round(env);
    timer.update_drand(randomness.round);
    ROUND_TIMER.save(deps.storage, &timer)?;

    let contract_balance = deps
        .querier
        .query_balance(&env.contract.address, &casino_config.native_denom)?;
    //Request from the reserve when the game contract doesn't have enough funds for the claimable amount for the winners
    if total_rewards > contract_balance.amount {
        let msg = WasmMsg::Execute {
            contract_addr: casino_config.reserve_address.to_string(),
            funds: vec![],
            msg: to_binary(&ReserveMsg::RequestFunds {
                amount: total_rewards - contract_balance.amount,
            })?,
        };
        Ok(Response::new().add_message(msg))
    } else {
        let diff = contract_balance.amount - total_rewards;
        if diff > casino_config.max_cashflow {
            // send diff to reserve
            let msg = WasmMsg::Execute {
                contract_addr: casino_config.reserve_address.to_string(),
                funds: vec![deduct_tax(
                    deps.as_ref(),
                    Coin::new(diff.u128(), casino_config.native_denom.clone()),
                )?],
                msg: to_binary(&ReserveMsg::DepositFunds {})?,
            };
            Ok(Response::new().add_message(msg))
        } else {
            Ok(Response::default())
        }
    }
}

pub fn execute_receive_rewards(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let response = match PLAYERS_REWARDS.may_load(deps.storage, info.sender.clone())? {
        Some(reward) => {
            if reward.is_zero() {
                Response::default()
            } else {
                let _ = TOTAL_REWARDS
                    .update(deps.storage, |total_rewards| -> Result<_, ContractError> {
                        Ok(total_rewards - reward)
                    })?;
                PLAYERS_REWARDS.save(deps.storage, info.sender.clone(), &Uint128::zero())?;
                let denom = CASINO_CONFIG.load(deps.storage)?.native_denom;
                let msg = BankMsg::Send {
                    to_address: info.sender.to_string(),
                    amount: vec![deduct_tax(deps.as_ref(), Coin::new(reward.u128(), denom))?],
                };
                Response::new().add_message(msg)
            }
        }
        None => Response::default(),
    };
    Ok(response)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::WinConfficients {} => to_binary(&query_win_coefficients(deps)?),
        QueryMsg::PlayerRewards { addr } => to_binary(&query_player_rewards(deps, addr)?),
        QueryMsg::CurrentRound {} => to_binary(&query_current_round(deps, env)?),
        QueryMsg::PlayerBetsForRound { addr, round } => {
            to_binary(&query_bets_address_for_round(deps, addr, round)?)
        }
        QueryMsg::PlayerBetsAllRounds { addr } => to_binary(&query_bets_address(deps, addr)?),
        QueryMsg::AllBets {
            last_evaluated_key,
            page_size,
        } => to_binary(&query_all_bets(deps, last_evaluated_key, page_size)?),
        QueryMsg::OutcomeHistory {} => to_binary(&query_outcome_history(deps)?),
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::GetBettingLimit {} => to_binary(&query_betting_limit(deps)?),
        QueryMsg::GetActiveBettingLimit {} => to_binary(&query_active_betting_limit(deps, env)?),
        QueryMsg::GetTotalRewards {} => to_binary(&query_total_rewards(deps)?),
    }
}

fn query_win_coefficients(deps: Deps) -> StdResult<WinCoefficients> {
    let coefficients = CASINO_CONFIG
        .load(deps.storage)?
        .win_coefficents
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    Ok(WinCoefficients { coefficients })
}

fn query_player_rewards(deps: Deps, addr: String) -> StdResult<Rewards> {
    let player = deps.api.addr_validate(&addr)?;
    let rewards = PLAYERS_REWARDS
        .may_load(deps.storage, player)?
        .unwrap_or_else(|| 0u128.into());
    Ok(Rewards { rewards })
}

fn query_current_round(deps: Deps, env: Env) -> StdResult<CurrentRound> {
    let mut exist_new_randomness: bool = false;
    let casino_config = CASINO_CONFIG.load(deps.storage)?;
    let timer = ROUND_TIMER.load(deps.storage)?;
    let randomness = get_randomness(deps, casino_config.terrand_address.to_string())?;
    // checking if we have new round of randomness
    let last_randomness_round = LAST_RANDOMNESS_ROUND.load(deps.storage)?;
    if randomness.round > last_randomness_round.u64() {
        //There is a new randomness
        exist_new_randomness = true;
    }
    Ok(CurrentRound {
        round: timer.current_round(),
        status: timer.round_status(&env, &exist_new_randomness),
        drand_round: timer.drand_round(),
    })
}

fn query_bets_address(deps: Deps, addr: String) -> StdResult<Vec<Bets>> {
    let player = deps.api.addr_validate(&addr)?;

    let player_active_in_rounds = PLAYER_BETS_ROUNDS.load(deps.storage, player.clone())?;

    let result = player_active_in_rounds
        .into_iter()
        .flat_map(|round| {
            return BETS
                .may_load(deps.storage, (round.into(), player.clone()))
                .and_then(|val| return Ok((round, val)));
        })
        .map(|(round, bet)| Bets {
            round: round,
            bets: bet,
        })
        .collect::<Vec<Bets>>();

    Ok(result)
}

fn query_all_bets(
    deps: Deps,
    last_evaluated_key: Option<(u64, String)>,
    page_size: Option<u16>,
) -> StdResult<Vec<(String, Bets)>> {
    let page_size = match page_size {
        Some(size) => size,
        None => 50,
    };

    let all: StdResult<Vec<(String, Bets)>> = BETS
        .range(
            deps.storage,
            last_evaluated_key.map(|(round, address)| {
                let mut prefix: Vec<u8> = vec![0, 8];
                let mut round_bytes: Vec<u8> = (round).to_be_bytes().into();
                let mut address_bytes: Vec<u8> = address.clone().as_bytes().into();

                prefix.append(&mut round_bytes);
                prefix.append(&mut address_bytes);

                Bound::Exclusive(prefix)
            }),
            None,
            Order::Descending,
        )
        .take(page_size.into())
        .map(|bets| {
            bets.map(|mut bet| {
                let addr_bytes = bet.0.split_off(10);
                // first two bytes offset
                let mut round_bytes = bet.0;
                round_bytes.drain(0..2);

                let round_bytes: &[u8; 8] = round_bytes
                    .as_slice()
                    .try_into()
                    .expect("slice with incorrect length");

                let round = u64::from_be_bytes(*round_bytes);
                let addr: String =
                    String::from_utf8(addr_bytes).expect("addr string incorrect length");
                return (
                    addr,
                    Bets {
                        bets: Some(bet.1),
                        round,
                    },
                );
            })
        })
        .collect();

    return all;
}

fn query_bets_address_for_round(deps: Deps, addr: String, round: u64) -> StdResult<Bets> {
    let player = deps.api.addr_validate(&addr)?;
    Ok(Bets {
        round,
        bets: BETS.may_load(deps.storage, (round.into(), player))?,
    })
}

fn query_outcome_history(deps: Deps) -> StdResult<OutcomeHistory> {
    Ok(OutcomeHistory {
        outcomes: OUTCOMES_HISTORY
            .range(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?
            .into_iter()
            .map(|(round, outcome)| (u64::from_be_bytes(round[0..8].try_into().unwrap()), outcome))
            .collect::<Vec<_>>(),
    })
}

fn query_config(deps: Deps) -> StdResult<crate::msg::Config> {
    Ok(crate::msg::Config {
        config: CASINO_CONFIG.load(deps.storage)?,
        timer: ROUND_TIMER.load(deps.storage)?,
    })
}

fn query_betting_limit(deps: Deps) -> StdResult<BettingLimit> {
    let casino_config = CASINO_CONFIG.load(deps.storage)?;
    let reserve_balance = get_reserve_balance(deps, casino_config.reserve_address.to_string())?;
    let total_bet_limit = reserve_balance.balance / Uint128::from(casino_config.max_betting_ratio);
    Ok(BettingLimit {
        limit: total_bet_limit,
    })
}

fn query_active_betting_limit(deps: Deps, env: Env) -> StdResult<BettingLimit> {
    let mut exist_new_randomness: bool = false;
    let casino_config = CASINO_CONFIG.load(deps.storage)?;
    let timer = ROUND_TIMER.load(deps.storage)?;
    let randomness = get_randomness(deps, casino_config.terrand_address.to_string())?;
    // checking if we have new round of randomness
    let last_randomness_round = LAST_RANDOMNESS_ROUND.load(deps.storage)?;
    if randomness.round > last_randomness_round.u64() {
        //There is a new randomness
        exist_new_randomness = true;
    }
    let current_round = timer.current_round();
    let reserve_balance = get_reserve_balance(deps, casino_config.reserve_address.to_string())?;
    let total_bet = get_total_bets_round(deps.storage, current_round)?;
    let total_bet_limit = match timer.round_status(&env, &exist_new_randomness) {
        RoundStatus::Live => 
          (reserve_balance.balance / Uint128::from(casino_config.max_betting_ratio)).sub(total_bet),
        RoundStatus::Ready => reserve_balance.balance / Uint128::from(casino_config.max_betting_ratio),
        RoundStatus::WaitingOnRandomness => reserve_balance.balance / Uint128::from(casino_config.max_betting_ratio),
        RoundStatus::Stopped => reserve_balance.balance / Uint128::from(casino_config.max_betting_ratio),
    };
    

    

    Ok(BettingLimit {
        limit: total_bet_limit,
    })
}

fn query_total_rewards(deps: Deps) -> StdResult<TotalRewards> {
    let total_rewards = TOTAL_REWARDS.load(deps.storage)?;
    Ok(TotalRewards { total_rewards })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockStorage};
    use cosmwasm_std::{coins, from_binary, OwnedDeps};

    use crate::state::DRAND_PERIOD_SECONDS;
    use crate::utils::tests_utils::CustomQuerier;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            gov_contract_address: "gov-contract".to_string(),
            native_denom: "uusd".to_string(),
            advantage_value: "0.01".to_string(),
            win_tax: "0.01".to_string(),
            max_number_of_bets: 1,
            max_betting_ratio: 1,
            round_duration: 10,
            max_cashflow: 10000u128.into(),
            terrand_address: "terrand".to_string(),
            reserve_address: "reserve".to_string(),
        };
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert!(res.messages.is_empty());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::WinConfficients {}).unwrap();
        let value: WinCoefficients = from_binary(&res).unwrap();
        assert_eq!(
            value,
            WinCoefficients {
                coefficients: vec![
                    "0.018285714285714285".to_string(),
                    "0.08".to_string(),
                    "0.188".to_string(),
                    "0.370769230769230769".to_string(),
                    "0.697142857142857142".to_string(),
                    "1.376".to_string(),
                    "2.564".to_string(),
                    "4.94".to_string(),
                    "10.88".to_string(),
                    "34.64".to_string(),
                ]
            }
        );

        #[cfg(feature = "debug")]
        {
            let res = query(deps.as_ref(), mock_env(), QueryMsg::GetConfig {}).unwrap();
            let value: crate::msg::Config = from_binary(&res).unwrap();
            println!("{:#?}", value);

            let info = mock_info("creator", &[]);
            let msg = ExecuteMsg::ChangeConfig {
                native_denom: Some("some_denom".to_string()),
                advantage_value: None,
                win_tax: Some("0.02".to_string()),
                max_number_of_bets: Some(2),
                max_betting_ratio: Some(2),
                round_duration: Some(20),
                max_cashflow: None,
                terrand_address: Some("terrand_new".to_string()),
                reserve_address: Some("reserve_new".to_string()),
            };
            let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

            let res = query(deps.as_ref(), mock_env(), QueryMsg::GetConfig {}).unwrap();
            let value: crate::msg::Config = from_binary(&res).unwrap();
            println!("{:#?}", value);
        }
    }

    #[test]
    fn betting() {
        use cw0::PaymentError;

        let mut deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: CustomQuerier::default(),
        };

        let msg = InstantiateMsg {
            gov_contract_address: "gov-contract".to_string(),
            native_denom: "uusd".to_string(),
            advantage_value: "0.01".to_string(),
            win_tax: "0.01".to_string(),
            max_number_of_bets: 3,
            max_betting_ratio: 1,
            round_duration: 10,
            max_cashflow: 10000u128.into(),
            terrand_address: "terrand".to_string(),
            reserve_address: "reserve".to_string(),
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();

        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert!(res.messages.is_empty());

        // checking for different invalid inputs
        let user_info = mock_info("user", &[]);
        let msg = ExecuteMsg::Bet { outcome: 99 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg);
        assert_eq!(
            res,
            Err(ContractError::InvalidBetPosition {
                current_position: 99,
                min_position: 3,
                max_position: 12,
            })
        );

        let user_info = mock_info("user", &[]);
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg);
        assert_eq!(res, Err(ContractError::Payment(PaymentError::NoFunds {})));

        let user_info = mock_info("user", &coins(123, "token"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg);
        assert_eq!(
            res,
            Err(ContractError::Payment(PaymentError::MissingDenom(
                "uusd".to_string()
            )))
        );

        // if called after round ended, new round starts
        let mut env_in_future = mock_env();

        env_in_future.block.time = env.block.time.plus_seconds(20 + DRAND_PERIOD_SECONDS);
        let user_info = mock_info("user", &coins(123, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env_in_future, user_info, msg);
        assert_eq!(res, Ok(Response::default()));

        // betting too much
        let user_info = mock_info("user", &coins(2000, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg);
        assert_eq!(
            res,
            Err(ContractError::BetAmountExceedsLimit {
                current_bet: 2000u128.into(),
                total_bet: 123u128.into(),
                total_bet_limit: 1000u128.into(),
            })
        );

        // successful bet
        let user_info = mock_info("user", &coins(100, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg).unwrap();
        assert!(res.messages.is_empty());

        // another successful bet
        let user_info = mock_info("user", &coins(23, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg).unwrap();
        assert!(res.messages.is_empty());

        let user_info = mock_info("user-2", &coins(999, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg);
        assert_eq!(
            res,
            Err(ContractError::BetAmountExceedsLimit {
                current_bet: 999u128.into(),
                total_bet: 246u128.into(),
                total_bet_limit: 1000u128.into(),
            })
        );

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PlayerBetsForRound {
                addr: "user".to_string(),
                round: 1,
            },
        )
        .unwrap();
        let value: Bets = from_binary(&res).unwrap();
        assert_eq!(
            value,
            Bets {
                round: 1u64.into(),
                bets: Some(vec![
                    (5, 123u128.into()),
                    (5, 100u128.into()),
                    (5, 23u128.into())
                ])
            }
        );

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PlayerBetsForRound {
                addr: "user".to_string(),
                round: 2,
            },
        )
        .unwrap();
        let value: Bets = from_binary(&res).unwrap();
        assert_eq!(
            value,
            Bets {
                round: 2u64.into(),
                bets: None
            }
        );

        // checking for exceeding bets amount
        let user_info = mock_info("user", &coins(123, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg);
        assert_eq!(
            res,
            Err(ContractError::MaxAmountOfBetsThisRound {
                bets_this_round: 3u128.into(),
                max_bets_per_round: 3u128.into(),
            })
        );

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PlayerRewards {
                addr: "user".to_string(),
            },
        )
        .unwrap();
        let value: Rewards = from_binary(&res).unwrap();
        assert_eq!(
            value,
            Rewards {
                rewards: 0u128.into()
            }
        );
    }

    #[test]
    fn betting_limit() {
        let mut deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: CustomQuerier::default(),
        };

        let msg = InstantiateMsg {
            gov_contract_address: "gov-contract".to_string(),
            native_denom: "uusd".to_string(),
            advantage_value: "0.01".to_string(),
            win_tax: "0.01".to_string(),
            max_number_of_bets: 3,
            max_betting_ratio: 1,
            round_duration: 10,
            max_cashflow: 10000u128.into(),
            terrand_address: "terrand".to_string(),
            reserve_address: "reserve".to_string(),
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();

        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert!(res.messages.is_empty());

        // if called after round ended, new round starts
        let mut env_in_future = mock_env();

        env_in_future.block.time = env.block.time.plus_seconds(20 + DRAND_PERIOD_SECONDS);
        let user_info = mock_info("user", &coins(123, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env_in_future, user_info, msg);
        assert_eq!(res, Ok(Response::default()));

        // betting too much
        let user_info = mock_info("user", &coins(2000, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg);
        assert_eq!(
            res,
            Err(ContractError::BetAmountExceedsLimit {
                current_bet: 2000u128.into(),
                total_bet: 123u128.into(),
                total_bet_limit: 1000u128.into(),
            })
        );

        // betting too much other user
        let user_info = mock_info("user-2", &coins(2000, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg);
        assert_eq!(
            res,
            Err(ContractError::BetAmountExceedsLimit {
                current_bet: 2000u128.into(),
                total_bet: 123u128.into(),
                total_bet_limit: 1000u128.into(),
            })
        );

        // successful bet
        let user_info = mock_info("user", &coins(100, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg).unwrap();
        assert!(res.messages.is_empty());

        // another successful bet
        let user_info = mock_info("user", &coins(23, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg).unwrap();
        assert!(res.messages.is_empty());

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetActiveBettingLimit {},
        )
        .unwrap();
        let value: BettingLimit = from_binary(&res).unwrap();
        assert_eq!(
            value,
            BettingLimit {
                limit: 754u128.into()
            }
        );

        // another successful bet
        let user_info = mock_info("user-2", &coins(46, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg).unwrap();
        assert!(res.messages.is_empty());

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetActiveBettingLimit {},
        )
        .unwrap();
        let value: BettingLimit = from_binary(&res).unwrap();
        assert_eq!(
            value,
            BettingLimit {
                limit: 708u128.into()
            }
        );
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetActiveBettingLimit {},
        )
        .unwrap();
        let value: BettingLimit = from_binary(&res).unwrap();
        assert_eq!(
            value,
            BettingLimit {
                limit: 708u128.into()
            }
        );

        let user_info = mock_info("user-2", &coins(900, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg);
        assert_eq!(
            res,
            Err(ContractError::BetAmountExceedsLimit {
                current_bet: 900u128.into(),
                total_bet: 292u128.into(),
                total_bet_limit: 1000u128.into(),
            })
        );
    }

    #[test]
    fn queries() {
        let mut deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: CustomQuerier::default(),
        };

        let msg = InstantiateMsg {
            gov_contract_address: "gov-contract".to_string(),
            native_denom: "uusd".to_string(),
            advantage_value: "0.01".to_string(),
            win_tax: "0.01".to_string(),
            max_number_of_bets: 3,
            max_betting_ratio: 1,
            round_duration: 10,
            max_cashflow: 10000u128.into(),
            terrand_address: "terrand".to_string(),
            reserve_address: "reserve".to_string(),
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();

        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert!(res.messages.is_empty());

        // if called after round ended, new round starts
        let mut env_in_future = mock_env();

        env_in_future.block.time = env.block.time.plus_seconds(20 + DRAND_PERIOD_SECONDS);
        let user_info = mock_info("user", &coins(123, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env_in_future.clone(), user_info, msg);
        assert_eq!(res, Ok(Response::default()));

        // successful bet
        let user_info = mock_info("user", &coins(100, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg).unwrap();
        assert!(res.messages.is_empty());

        // another successful bet
        let user_info = mock_info("user", &coins(23, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg).unwrap();
        assert!(res.messages.is_empty());

        let user_info = mock_info("user-2", &coins(10, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg).unwrap();
        assert!(res.messages.is_empty());

        let user_info = mock_info("user-2", &coins(100, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg).unwrap();
        assert!(res.messages.is_empty());

        let user_info = mock_info("user-2", &coins(23, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env_in_future.clone(), user_info, msg).unwrap();
        assert!(res.messages.is_empty());

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PlayerBetsForRound {
                addr: "user".to_string(),
                round: 1,
            },
        )
        .unwrap();
        let value: Bets = from_binary(&res).unwrap();
        assert_eq!(
            value,
            Bets {
                round: 1u64.into(),
                bets: Some(vec![
                    (5, 123u128.into()),
                    (5, 100u128.into()),
                    (5, 23u128.into())
                ])
            }
        );

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetActiveBettingLimit {},
        )
        .unwrap();
        let value: BettingLimit = from_binary(&res).unwrap();
        assert_eq!(
            value,
            BettingLimit {
                limit: 621u128.into()
            }
        );

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PlayerBetsForRound {
                addr: "user".to_string(),
                round: 2,
            },
        )
        .unwrap();
        let value: Bets = from_binary(&res).unwrap();
        assert_eq!(
            value,
            Bets {
                round: 2u64.into(),
                bets: None
            }
        );

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PlayerBetsAllRounds {
                addr: "user-2".to_string(),
            },
        )
        .unwrap();
        let value: Vec<Bets> = from_binary(&res).unwrap();
        assert_eq!(
            value,
            [Bets {
                round: 1u64.into(),
                bets: Some(vec![
                    (5, 10u128.into()),
                    (5, 100u128.into()),
                    (5, 23u128.into())
                ])
            }]
        );

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::AllBets {
                last_evaluated_key: None,
                page_size: None,
            },
        )
        .unwrap();
        let value: Vec<(String, Bets)> = from_binary(&res).unwrap();
        assert_eq!(
            value,
            [
                (
                    String::from("user-2"),
                    Bets {
                        round: 1,
                        bets: Some(vec![
                            (5, Uint128::from(10u128)),
                            (5, Uint128::from(100u128)),
                            (5, Uint128::from(23u128))
                        ])
                    }
                ),
                (
                    String::from("user"),
                    Bets {
                        round: 1,
                        bets: Some(vec![
                            (5, Uint128::from(123u128)),
                            (5, Uint128::from(100u128)),
                            (5, Uint128::from(23u128))
                        ])
                    }
                )
            ]
        );
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::AllBets {
                last_evaluated_key: Some((1u64, "user".to_string())),
                page_size: None,
            },
        )
        .unwrap();
        let value: Vec<(String, Bets)> = from_binary(&res).unwrap();
        assert_eq!(
            value,
            [(
                String::from("user-2"),
                Bets {
                    round: 1,
                    bets: Some(vec![
                        (5, Uint128::from(10u128)),
                        (5, Uint128::from(100u128)),
                        (5, Uint128::from(23u128))
                    ])
                }
            )]
        );
    }

    #[test]
    fn ending_round() {
        let mut deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: CustomQuerier::default(),
        };

        let init_msg = InstantiateMsg {
            gov_contract_address: "gov-contract".to_string(),
            native_denom: "uusd".to_string(),
            advantage_value: "0.01".to_string(),
            win_tax: "0.01".to_string(),
            max_number_of_bets: 1,
            max_betting_ratio: 1,
            round_duration: 10,
            max_cashflow: 10000u128.into(),
            terrand_address: "terrand".to_string(),
            reserve_address: "reserve".to_string(),
        };
        let info = mock_info("creator", &[]);
        let mut env = mock_env();

        let res = instantiate(deps.as_mut(), env.clone(), info, init_msg.clone()).unwrap();
        assert!(res.messages.is_empty());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::CurrentRound {}).unwrap();
        let value: CurrentRound = from_binary(&res).unwrap();
        assert_eq!(
            value,
            CurrentRound {
                round: 0u64.into(),
                status: RoundStatus::Live,
                drand_round: 0u64.into(),
            }
        );

        let user_info = mock_info("user", &coins(1000, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg).unwrap();
        assert!(res.messages.is_empty());

        // round ended
        env.block.time = env
            .block
            .time
            .plus_seconds(init_msg.round_duration + DRAND_PERIOD_SECONDS + 1);

        let res = query(deps.as_ref(), env.clone(), QueryMsg::CurrentRound {}).unwrap();
        let value: CurrentRound = from_binary(&res).unwrap();
        assert_eq!(
            value,
            CurrentRound {
                round: 0u64.into(),
                status: RoundStatus::WaitingOnRandomness,
                drand_round: 0u64.into(),
            }
        );

        // imitating new randomness round
        LAST_RANDOMNESS_ROUND
            .save(&mut deps.storage, &0u64.into())
            .unwrap();

        // dice1 -> 305419898 mod 6 + 1 = 3
        // dice2 -> 2417112152 mod 6 + 1 = 3
        // outcome is 6, bet position is 5, so user won
        // coeficient for 5 is 1.188 and bet was 1000, so reward is 188
        // with 1% tax 188 is 186
        // currend balance is 0 so we need 1186 to pay the winner
        let user_info = mock_info("user", &coins(1000, "uusd"));
        let msg = ExecuteMsg::Bet { outcome: 5 };
        let res = execute(deps.as_mut(), env.clone(), user_info, msg).unwrap();
        assert_eq!(res, {
            let msg = WasmMsg::Execute {
                contract_addr: "reserve".to_string(),
                funds: vec![],
                msg: to_binary(&ReserveMsg::RequestFunds {
                    amount: 1186u128.into(),
                })
                .unwrap(),
            };
            Response::new().add_message(msg)
        });

        let res = query(deps.as_ref(), env.clone(), QueryMsg::OutcomeHistory {}).unwrap();
        let value: OutcomeHistory = from_binary(&res).unwrap();
        assert_eq!(
            value,
            OutcomeHistory {
                outcomes: vec![(0, 6)],
            }
        );

        let total_rewards = TOTAL_REWARDS.load(&deps.storage).unwrap();
        assert_eq!(total_rewards, 1186u128.into());

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PlayerRewards {
                addr: "user".to_string(),
            },
        )
        .unwrap();
        let value: Rewards = from_binary(&res).unwrap();
        assert_eq!(
            value,
            Rewards {
                rewards: 1186u128.into()
            }
        );

        let user_info = mock_info("user", &[]);
        let msg = ExecuteMsg::ReceiveRewards {};
        let res = execute(deps.as_mut(), env.clone(), user_info, msg).unwrap();
        assert_eq!(res.messages.len(), 1);

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PlayerRewards {
                addr: "user".to_string(),
            },
        )
        .unwrap();
        let value: Rewards = from_binary(&res).unwrap();
        assert_eq!(
            value,
            Rewards {
                rewards: Uint128::zero(),
            }
        );

        let total_rewards = TOTAL_REWARDS.load(&deps.storage).unwrap();
        assert_eq!(total_rewards, 0u128.into());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::CurrentRound {}).unwrap();
        let value: CurrentRound = from_binary(&res).unwrap();
        assert_eq!(
            value,
            CurrentRound {
                round: 1u64.into(),
                status: RoundStatus::Live,
                drand_round: 1u64.into(),
            }
        );
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
