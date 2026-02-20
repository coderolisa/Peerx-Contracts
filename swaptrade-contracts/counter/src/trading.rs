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

    // Swap logic is implemented in the top-level `trading.rs` that's included
    // by the crate root. This function acts as a thin wrapper used by the
    // contract interface. Keep it minimal to avoid duplication.

    Ok(())
}
