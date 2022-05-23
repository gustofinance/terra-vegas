use cosmwasm_std::StdError;
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
    #[error("InvalidReplyId")]
    InvalidReplyId {},
}
