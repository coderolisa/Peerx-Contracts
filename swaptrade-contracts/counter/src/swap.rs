use crate::emergency;
use crate::errors::SwapTradeError;

pub fn perform_swap(
    env: &Env,
    portfolio: &mut Portfolio,
    from: Symbol,
    to: Symbol,
    amount: i128,
    user: Address,
) -> Result<i128, SwapTradeError> {
    if emergency::is_paused(env) {
        return Err(SwapTradeError::TradingPaused);
    }
    if emergency::is_frozen(env, user.clone()) {
        return Err(SwapTradeError::UserFrozen);
    }

    // circuit breaker check
    let normal_volume = 1000;
    if emergency::would_trip_circuit_breaker(env, amount, normal_volume) {
        return Err(SwapTradeError::CircuitBreakerTripped);
    }

    // record volume
    emergency::record_volume(env, amount);

    // ... rest of swap code
    Ok(0)
}
