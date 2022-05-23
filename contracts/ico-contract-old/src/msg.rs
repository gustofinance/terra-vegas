use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Uint128};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub privatesale_allocation: Uint128,
    pub privatesale_price: Uint128,
    pub privatesale_duration: u64,
    pub publicsale_allocation: Uint128,
    pub publicsale_initial_price: Uint128,
    pub publicsale_final_price: Uint128,
    pub publicsale_duration: u64,
    pub price_denom: String,
    pub revenue_distribution: Vec<RevenuePercentage>,
    pub whitelist: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RevenuePercentage {
    // Address to which send revenue
    pub addr: String,
    // Persentage of revenue that needs to be send.
    // Reprented in range [0..1],
    // so for example 5% would be 0.05
    // and 100% is 1
    pub percentage: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IcoTimes {
    pub start_time: Option<u64>,
    pub private_end: u64,
    pub public_end: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AddToWhitelist { addresses: Vec<String> },
    RemoveFromWhitelist { addresses: Vec<String> },
    SetTokenAddress { addr: String },
    StartIco {},
    EndIco {},
    Buy { amount: Uint128 },
    ReceiveTokens {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    IcoInfo {},
    Prices {},
    PricesForAmount { amount: Uint128 },
    Balance { addr: String },
    IcoTimes{},
    Whitelist{},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IcoInfo {
    pub stage: IcoStage,
    pub private_coins_remains: Uint128,
    pub public_coins_remains: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum IcoStage {
    NotStarted,
    PrivateSale,
    PublicSale,
    Ended,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Prices {
    pub private_sale_price: Uint128,
    pub public_sale_price: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PricesForAmount {
    pub amount: Uint128,
    pub private_sale_price: Uint128,
    pub public_sale_price: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserBalance {
    pub balance: Option<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Whitelist {
    pub whitelist: Vec<String>,
}