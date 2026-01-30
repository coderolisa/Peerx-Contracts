use soroban_sdk::{Address, Env};

use crate::errors::SwapTradeError;
use crate::storage::PAUSED_KEY;

pub fn swap(env: Env, user: Address, amount: i128) -> Result<(), SwapTradeError> {
    user.require_auth();

    let paused = env
        .storage()
        .persistent()
        .get::<_, bool>(&PAUSED_KEY)
        .unwrap_or(false);

    if paused {
        return Err(SwapTradeError::TradingPaused);
    }

    // existing swap logic continues here...

    Ok(())
}
