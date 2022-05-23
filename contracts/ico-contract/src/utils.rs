use crate::error::ContractError;
use crate::msg::ReceiversePercentage;
use crate::state::RECEIVERS;
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{Coin, Decimal, Deps, DepsMut, StdResult};
use terra_cosmwasm::TerraQuerier;

// Stores percentages and checks that their sum is equal to 1 (100%)
pub fn set_revenue_distribution(
    deps: &mut DepsMut,
    reseivers: &[ReceiversePercentage],
) -> Result<(), ContractError> {
    use std::str::FromStr;
    let mut percentage_sum = Decimal::zero();
    for reseiver in reseivers {
        let addr = deps.api.addr_validate(&reseiver.addr)?;
        let percentage = Decimal::from_str(&reseiver.percentage)?;
        RECEIVERS.save(deps.storage, addr, &percentage)?;
        percentage_sum = percentage_sum + percentage;
    }
    if percentage_sum != Decimal::one() {
        return Err(ContractError::InvalidTotalDistributionPercentages {
            expected: Decimal::one(),
            got: percentage_sum,
        });
    }
    Ok(())
}

pub fn compute_tax(deps: Deps, coin: &Coin) -> StdResult<Uint256> {
    let terra_querier = TerraQuerier::new(&deps.querier);
    let tax_rate = Decimal256::from((terra_querier.query_tax_rate()?).rate);
    let tax_cap = Uint256::from((terra_querier.query_tax_cap(coin.denom.clone())?).cap);
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