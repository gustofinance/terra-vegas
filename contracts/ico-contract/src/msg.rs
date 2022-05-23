use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LaunchConfig {
    pub amount: Uint128,
    // pahse 1: can deposit and withdraw
    pub phase1_start: u64,
    // phase2: can withdraw one time. Allowed withdraw decreases 100% to 0% over time.
    pub phase2_start: u64,
    pub phase2_end: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: String,
    pub token: String,
    pub base_denom: String,
    pub receivers: Vec<ReceiversePercentage>,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ReceiversePercentage {
    // Address to which send revenue
    pub addr: String,
    // Persentage of revenue that needs to be send.
    // Reprented in range [0..1],
    // so for example 5% would be 0.05
    // and 100% is 1
    pub percentage: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Deposit {},
    Withdraw { amount: Option<Uint128> },
    WithdrawTokens {},
    PostInitialize { launch_config: LaunchConfig },
    AdminWithdraw {},
    ReleaseTokens {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    DepositInfo { address: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub token: String,
    pub launch_config: Option<LaunchConfig>,
    pub base_denom: String,
    pub tokens_released: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DepositResponse {
    pub deposit: Uint128,
    pub total_deposit: Uint128,
    pub withdrawable_amount: Uint128,
    pub tokens_to_claim: Uint128,
    pub can_claim: bool,
}
