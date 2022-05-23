use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::state::RoundStatus;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub native_denom: String,
    pub advantage_value: String,
    pub win_tax: String,
    pub max_number_of_bets: u64,
    pub max_betting_ratio: u64,
    pub round_duration: u64,
    pub max_cashflow: Uint128,
    pub terrand_address: String,
    pub reserve_address: String,
    pub gov_contract_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ChangeAdwantageValue {
        advantage_value: String,
    },
    ChangeWinTax {
        win_tax: String,
    },
    ChangeMaxNumberOfBets {
        number_of_bets: u64,
    },
    ChangeMaxBettingRatio {
        ratio: u64,
    },
    ChangeRoundDuration {
        duration: u64,
    },
    ChangeMaxCashflow {
        cashflow: Uint128,
    },
    Bet {
        outcome: u8,
    },
    ReceiveRewards {},
    DrainGame {},
    StopGame {},
    #[cfg(feature = "debug")]
    ChangeConfig {
        native_denom: Option<String>,
        advantage_value: Option<String>,
        win_tax: Option<String>,
        max_number_of_bets: Option<u64>,
        max_betting_ratio: Option<u64>,
        round_duration: Option<u64>,
        max_cashflow: Option<Uint128>,
        terrand_address: Option<String>,
        reserve_address: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    WinConfficients {},
    PlayerRewards {
        addr: String,
    },
    CurrentRound {},
    Bets {
        addr: String,
        round: u64,
    },
    OutcomeHistory {},
    GetConfig {},
    GetBettingLimit{},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WinCoefficients {
    pub coefficients: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Rewards {
    pub rewards: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CurrentRound {
    pub round: u64,
    pub status: RoundStatus,
    pub drand_round: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Bets {
    pub round: u64,
    pub bets: Option<Vec<(u8, Uint128)>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OutcomeHistory {
    pub outcomes: Vec<(u64, u8)>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BettingLimit {
    pub limit: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub config: crate::state::CasinoConfig,
    pub timer: crate::state::RoundTimer,
}
