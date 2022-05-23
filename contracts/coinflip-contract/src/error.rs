use cosmwasm_std::{StdError, Uint128};
use cw0::PaymentError;
use thiserror::Error;

#[derive(Error, PartialEq, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("{0}")]
    Payment(#[from] PaymentError),
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("Advantage value is out of range")]
    AdvantageValueOutOfRange {},
    #[error("New randomness round is not available yet")]
    NewRandomnessNotYetAvailable {},
    #[error("Bet amount exceeds limit")]
    BetAmountExceedsLimit {
        current_bet: Uint128,
        total_bet: Uint128,
        total_bet_limit: Uint128,
    },
    #[error("Invalid bet position")]
    InvalidBetPosition {
        current_position: u8,
        min_position: u8,
        max_position: u8,
    },
    #[error("Max amount of bets this round")]
    MaxAmountOfBetsThisRound {
        bets_this_round: Uint128,
        max_bets_per_round: Uint128,
    },
    #[error("Round ended")]
    RoundEnded {},
    #[error("Game stopped")]
    GameStopped {},
}
