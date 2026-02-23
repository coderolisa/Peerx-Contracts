use soroban_sdk::{symbol_short, Address, Env};

use crate::alerts::{check_portfolio_alerts, check_price_alerts};
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

    // Check price alerts for the XLM token against the swap amount.
    // In production, replace `amount` with the oracle price for the traded token.
    check_price_alerts(&env, &symbol_short!("XLM"), amount);

    // Check portfolio alerts for this user after the swap has been processed.
    // In production, pass the real current and reference portfolio values from
    // the portfolio module instead of `amount`.
    check_portfolio_alerts(&env, &user, amount, amount);

    Ok(())
}
