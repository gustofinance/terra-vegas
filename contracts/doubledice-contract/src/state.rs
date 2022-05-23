use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal, Env, Uint128, Uint64};
use cw_storage_plus::{Item, Map, U64Key};

pub const OWNER: Item<Addr> = Item::new("owner");
pub const LAST_RANDOMNESS_ROUND: Item<Uint64> = Item::new("last_randomness_round");
// (round, address) -> vec<(bet, amount)>
pub const PLAYER_BETS_ROUNDS: Map<Addr, HashSet<u64>> = Map::new("player_bets_rounds");
pub const BETS: Map<(U64Key, Addr), Vec<(u8, Uint128)>> = Map::new("bets");
pub const OUTCOMES_HISTORY: Map<U64Key, u8> = Map::new("outcomes_history");
pub const TOTAL_REWARDS: Item<Uint128> = Item::new("total_rewards");
pub const PLAYERS_REWARDS: Map<Addr, Uint128> = Map::new("players_rewards");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CasinoConfig {
    pub native_denom: String,
    pub win_coefficents: Vec<Decimal>,
    pub win_tax: Decimal,
    pub max_number_of_bets: u64,
    pub max_betting_ratio: u64,
    pub max_cashflow: Uint128,
    pub terrand_address: Addr,
    pub reserve_address: Addr,
    pub gov_contract_address: Addr,
}

pub const CASINO_CONFIG: Item<CasinoConfig> = Item::new("casino_config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum RoundStatus {
    Live,
    WaitingOnRandomness,
    Ready,
    Stopped,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RoundTimer {
    pub round_duration: u64,
    pub current_round_start_time: u64,
    pub current_round: u64,
    pub stopped: bool,
    pub drand_round: u64, //This needs to go in OUTCOMES_HISTORY instead so we register the whole history of matching betwwen drand_rounds and vegas rounds
}

pub const DRAND_PERIOD_SECONDS: u64 = 30;

impl RoundTimer {
    pub fn new(round_duration: u64, env: Env) -> Self {
        Self {
            round_duration,
            current_round_start_time: env.block.time.seconds(),
            current_round: 0,
            stopped: false,
            drand_round: 0,
        }
    }

    pub fn update_duration(&mut self, duration: u64) {
        self.round_duration = duration;
    }
    pub fn update_drand(&mut self, drand_round: u64) {
        self.drand_round = drand_round;
    }

    pub fn stop(&mut self) {
        self.stopped = true;
    }

    pub fn round_status(&self, env: &Env, exist_new_randomness: &bool) -> RoundStatus {
        if self.stopped {
            RoundStatus::Stopped
        } else if env.block.time.seconds()
            >= (self.current_round_start_time + self.round_duration + DRAND_PERIOD_SECONDS)
        {
            //if Round time has elapsed
            if !exist_new_randomness {
                RoundStatus::WaitingOnRandomness
            } else {
                RoundStatus::Ready // Means ready to settle and play
            }
        } else {
            RoundStatus::Live
        }
    }

    pub fn current_round(&self) -> u64 {
        self.current_round
    }
    pub fn drand_round(&self) -> u64 {
        self.drand_round
    }

    pub fn next_round(&mut self, env: &Env) {
        self.current_round_start_time = env.block.time.seconds();
        self.current_round += 1;
    }
}

pub const ROUND_TIMER: Item<RoundTimer> = Item::new("round_timer");
