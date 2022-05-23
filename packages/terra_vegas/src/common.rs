use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Order;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrderBy {
    Asc,
    Desc,
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

impl From<OrderBy> for Order {
    fn from(o: OrderBy) -> Order {
        if o == OrderBy::Asc {
            Order::Ascending
        } else {
            Order::Descending
        }
    }
}
