use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map, U64Key};

pub const OWNER: Item<Addr> = Item::new("owner");
pub const BALANCE_HISTORY: Map<U64Key,Uint128> = Map::new("balance_history");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub anchor_market_address: Addr,
    pub gov_contract_address: Addr,
    pub anchor_token_address: Addr,
    pub threshold: Uint128,
    pub native_denom: String,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const GAMES: Map<Addr, ()> = Map::new("games");
pub const REQUESTING_CONTRACT: Item<(Addr, Uint128)> = Item::new("requesting_contract");
