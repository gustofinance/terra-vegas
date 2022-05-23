use crate::error::ContractError;
use crate::state::BETS;
use crate::state::{CASINO_CONFIG, OWNER};
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    to_binary, Coin, Decimal, Deps, MessageInfo, Order, StdError, StdResult, Storage, Uint128,
    WasmQuery,
};
use terra_cosmwasm::TerraQuerier;

pub fn get_randomness(
    deps: Deps,
    terrand_address: String,
) -> StdResult<terrand::msg::LatestRandomResponse> {
    let msg = terrand::msg::QueryMsg::LatestDrand {};
    let wasm = WasmQuery::Smart {
        contract_addr: terrand_address,
        msg: to_binary(&msg)?,
    };
    let random: terrand::msg::LatestRandomResponse = deps.querier.query(&wasm.into())?;
    Ok(random)
}

pub fn get_reserve_balance(
    deps: Deps,
    reserve_address: String,
) -> StdResult<reserve_contract::msg::CurrentBalance> {
    let msg = reserve_contract::msg::QueryMsg::CurrentBalance {};
    let wasm = WasmQuery::Smart {
        contract_addr: reserve_address,
        msg: to_binary(&msg)?,
    };
    let balance: reserve_contract::msg::CurrentBalance = deps.querier.query(&wasm.into())?;
    Ok(balance)
}

pub fn get_total_bets_round(storage: &dyn Storage, round: u64) -> StdResult<Uint128> {
    return BETS
        .prefix(round.into())
        .range(storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?
        .iter()
        .fold(Ok(Uint128::zero()), |total_per_round, bet| {
            bet.1.iter().fold(
                total_per_round,
                |total_per_address, b| match total_per_address {
                    Ok(t) => t.checked_add(b.1),
                    Err(e) => Err(e),
                },
            )
        })
        .map_err(|e| StdError::Overflow { source: e });
}

pub fn recalculate_win_coefficients(advantage_value: &str) -> Result<Vec<Decimal>, ContractError> {
    use cosmwasm_std::Fraction;
    use std::str::FromStr;
    // win probability goes from 1/36 to 35/36
    // advantage_value converts to `c` as
    //
    // c = 1 - advantage_value
    //
    // win coefficient can be calculated as
    //
    //       c
    // wi = ----
    //      i/36
    //
    // where i - from 1 to 35
    //
    // this calculation can be rewritten as
    //
    // c.numerator * 36
    // -----------------
    // c.denominator * i

    let c = Decimal::one() - Decimal::from_str(advantage_value)?;

    let i = [1, 3, 6, 10, 15, 21, 26, 30, 33, 35];

    Ok(i.iter()
        .rev()
        .map(|i| Decimal::from_ratio(c.numerator() * 36, c.denominator() * i) - Decimal::one())
        .collect())
}

pub fn only_owner(deps: Deps, info: &MessageInfo) -> Result<(), ContractError> {
    let owner = OWNER.load(deps.storage)?;
    let config = CASINO_CONFIG.load(deps.storage)?;

    if info.sender == owner || info.sender == config.gov_contract_address {
        Ok(())
    } else {
        Err(ContractError::Unauthorized {})
    }
}

pub fn compute_tax(deps: Deps, coin: &Coin) -> StdResult<Uint256> {
    let terra_querier = TerraQuerier::new(&deps.querier);
    let tax_rate = Decimal256::from((terra_querier.query_tax_rate()?).rate);
    let tax_cap = Uint256::from((terra_querier.query_tax_cap(coin.denom.to_string())?).cap);
    let amount = Uint256::from(coin.amount);
    Ok(std::cmp::min(
        amount * Decimal256::one() - amount / (Decimal256::one() + tax_rate),
        tax_cap,
    ))
}

pub fn deduct_tax(deps: Deps, coin: Coin) -> StdResult<Coin> {
    let tax_amount = compute_tax(deps, &coin)?;
    Ok(Coin {
        denom: coin.denom,
        amount: (Uint256::from(coin.amount) - tax_amount).into(),
    })
}

#[cfg(test)]
pub mod tests_utils {
    // we inplement custom moc querier because default one from cosmwasm does not support quering
    // contracts
    use cosmwasm_std::{
        from_slice,
        testing::{BankQuerier, MockQuerierCustomHandlerResult},
        to_binary, Binary, ContractResult, Decimal, Querier, QuerierResult, QueryRequest,
        SystemError, SystemResult, WasmQuery,
    };
    use terra_cosmwasm::{TaxCapResponse, TaxRateResponse, TerraQuery, TerraQueryWrapper};

    pub struct CustomQuerier {
        bank: BankQuerier,
        wasm: CustomWasmQuerier,
        custom_handler:
            Box<dyn for<'a> Fn(&'a TerraQueryWrapper) -> MockQuerierCustomHandlerResult>,
    }

    impl Default for CustomQuerier {
        fn default() -> Self {
            Self {
                bank: BankQuerier::default(),
                wasm: CustomWasmQuerier::default(),
                custom_handler: Box::new(
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
                ),
            }
        }
    }

    impl Querier for CustomQuerier {
        fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
            let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
                Ok(v) => v,
                Err(e) => {
                    return SystemResult::Err(SystemError::InvalidRequest {
                        error: format!("Parsing query request: {}", e),
                        request: bin_request.into(),
                    })
                }
            };
            match request {
                QueryRequest::Bank(bank_query) => self.bank.query(&bank_query),
                QueryRequest::Wasm(wasm_query) => self.wasm.query(&wasm_query),
                QueryRequest::Custom(custom_query) => (*self.custom_handler)(&custom_query),
                _ => {
                    unreachable!()
                }
            }
        }
    }

    #[derive(Default)]
    pub struct CustomWasmQuerier {}

    impl CustomWasmQuerier {
        fn query(&self, query: &WasmQuery) -> QuerierResult {
            match query {
                WasmQuery::Smart { contract_addr, .. } => {
                    match contract_addr.as_str() {
                        "reserve" => SystemResult::Ok(ContractResult::Ok(
                            to_binary(&reserve_contract::msg::CurrentBalance {
                                balance: 1000u128.into(),
                            })
                            .unwrap(),
                        )),
                        "terrand" => {
                            SystemResult::Ok(ContractResult::Ok(
                                to_binary(&terrand::msg::LatestRandomResponse {
                                    // first 16 bytes represent 305419898
                                    // second 16 bytes represent 2417112152
                                    round: 1,
                                    randomness: Binary::from(&[
                                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x12, 0x34, 0x56, 0x80,
                                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x90, 0x12, 0x34, 0x58,
                                    ]),
                                    worker: "".to_string(),
                                })
                                .unwrap(),
                            ))
                        }
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}
