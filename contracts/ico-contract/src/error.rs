use cosmwasm_std::{StdError, Decimal};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("post_initialize called multiple times")]
    DuplicatePostInit {},

    #[error("Invalid launch config")]
    InvalidLaunchConfig {},

    #[error("Invalid deposit: {reason}")]
    InvalidDeposit { reason: String },

    #[error("Invalid withdraw: {reason}")]
    InvalidWithdraw { reason: String },

    #[error("Invalid withdraw tokens: {reason}")]
    InvalidWithdrawTokens { reason: String },

    #[error("Invalid admin withdraw: {reason}")]
    InvalidAdminWithdraw { reason: String },

    #[error("Invalid release tokens: {reason}")]
    InvalidReleaseTokens { reason: String },

    #[error("Distribution percentages do not add up to the expected values")]
    InvalidTotalDistributionPercentages { expected: Decimal, got: Decimal },

    #[error("Fee can not be bigger than 1")]
    InvalidFee {},
}
