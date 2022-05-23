use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use ico_contract::msg::{IcoInfo, Prices, PricesForAmount, UserBalance, ExecuteMsg, InstantiateMsg, QueryMsg};
use ico_contract::state::{PrivateSaleCoinSupply, PublicSaleCoinSupply, IcoTimer};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(IcoInfo), &out_dir);
    export_schema(&schema_for!(Prices), &out_dir);
    export_schema(&schema_for!(PricesForAmount), &out_dir);
    export_schema(&schema_for!(UserBalance), &out_dir);

    export_schema(&schema_for!(PrivateSaleCoinSupply), &out_dir);
    export_schema(&schema_for!(PublicSaleCoinSupply), &out_dir);
    export_schema(&schema_for!(IcoTimer), &out_dir);
}
