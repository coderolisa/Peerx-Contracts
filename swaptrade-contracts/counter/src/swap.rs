use crate::emergency;
use crate::errors::PeerXError;

pub fn perform_swap(
    env: &Env,
    portfolio: &mut Portfolio,
    from: Symbol,
    to: Symbol,
    amount: i128,
    user: Address,
) -> Result<i128, PeerXError> {
    if emergency::is_paused(env) {
        return Err(PeerXError::TradingPaused);
    }
    if emergency::is_frozen(env, user.clone()) {
        return Err(PeerXError::UserFrozen);
    }

    // circuit breaker check
    let normal_volume = 1000;
    if emergency::would_trip_circuit_breaker(env, amount, normal_volume) {
        return Err(PeerXError::CircuitBreakerTripped);
    }

    // record volume
    emergency::record_volume(env, amount);

    // ... rest of swap code
    Ok(0)
}
