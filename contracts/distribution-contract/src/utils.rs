use cosmwasm_std::{Deps, MessageInfo, StdError, StdResult};

use crate::state::{CONFIG, OWNER};

pub fn only_owner(deps: Deps, info: &MessageInfo) -> StdResult<()> {
    let owner = OWNER.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;

    if info.sender == owner || info.sender == config.governance_token_addr {
        Ok(())
    } else {
        Err(StdError::GenericErr {
            msg: "unauthorized".into(),
        })
    }
}
