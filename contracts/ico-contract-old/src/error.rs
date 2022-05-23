use cosmwasm_std::{Decimal, StdError, Uint128};
use cw0::PaymentError;
use thiserror::Error;

#[derive(Error, PartialEq, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("{0}")]
    Payment(#[from] PaymentError),
    #[error("Distribution percentages do not add up to the expected values")]
    InvalidTotalDistributionPercentages { expected: Decimal, got: Decimal },
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("ICO is not started yet")]
    ICONotStarted {},
    #[error("ICO already started")]
    ICOAlreadyStarted {},
    #[error("ICO already ended")]
    ICOEnded {},
    #[error("Not whitelisted")]
    NotWhitelisted {},
    #[error("Incorrect amount of funds")]
    IncorrectAmountOfFunds {
        provided: Uint128,
        required: Uint128,
        max_allowed: Uint128,
    },
    #[error("Not enough coins left")]
    NotEnoughCoinsLeft {
        remains: Uint128,
        requested: Uint128,
    },
    #[error("Can not withdraw until ICO end")]
    CanNotWithdrawUntilICOEnd {},
}
