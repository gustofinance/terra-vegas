use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use coinflip_game::msg::{
    Bets, CurrentRound, ExecuteMsg, InstantiateMsg, QueryMsg, Rewards, WinCoefficients,
};
use coinflip_game::state::{CasinoConfig, RoundTimer};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(CasinoConfig), &out_dir);
    export_schema(&schema_for!(RoundTimer), &out_dir);
    export_schema(&schema_for!(WinCoefficients), &out_dir);
    export_schema(&schema_for!(Rewards), &out_dir);
    export_schema(&schema_for!(CurrentRound), &out_dir);
    export_schema(&schema_for!(Bets), &out_dir);
}
