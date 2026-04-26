// Security Audit Enhancements
//
// Additional input validation and invariant checks layered on top of the
// existing audit trail. These helpers are called before state-mutating
// operations to surface violations early.

use std::collections::HashMap;

pub const MAX_TRADE_AMOUNT: u128 = 1_000_000_000_000;
pub const MIN_FEE_BPS: u16 = 1;
pub const MAX_FEE_BPS: u16 = 10_000;
pub const MAX_SLIPPAGE_BPS: u16 = 5_000;

#[derive(Debug, PartialEq, Eq)]
pub enum SecurityError {
    AmountExceedsLimit,
    InvalidFee,
    SlippageTooHigh,
    ZeroAmount,
    UnauthorizedCaller,
    RateLimitExceeded,
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityError::AmountExceedsLimit => write!(f, "amount exceeds protocol limit"),
            SecurityError::InvalidFee => write!(f, "fee bps out of valid range"),
            SecurityError::SlippageTooHigh => write!(f, "slippage tolerance too high"),
            SecurityError::ZeroAmount => write!(f, "amount must be non-zero"),
            SecurityError::UnauthorizedCaller => write!(f, "caller is not authorized"),
            SecurityError::RateLimitExceeded => write!(f, "rate limit exceeded"),
        }
    }
}

pub fn validate_trade_amount(amount: u128) -> Result<(), SecurityError> {
    if amount == 0 {
        return Err(SecurityError::ZeroAmount);
    }
    if amount > MAX_TRADE_AMOUNT {
        return Err(SecurityError::AmountExceedsLimit);
    }
    Ok(())
}

pub fn validate_fee(fee_bps: u16) -> Result<(), SecurityError> {
    if fee_bps < MIN_FEE_BPS || fee_bps > MAX_FEE_BPS {
        return Err(SecurityError::InvalidFee);
    }
    Ok(())
}

pub fn validate_slippage(slippage_bps: u16) -> Result<(), SecurityError> {
    if slippage_bps > MAX_SLIPPAGE_BPS {
        return Err(SecurityError::SlippageTooHigh);
    }
    Ok(())
}

pub fn assert_authorized(caller: &str, allowed: &[&str]) -> Result<(), SecurityError> {
    if allowed.contains(&caller) {
        Ok(())
    } else {
        Err(SecurityError::UnauthorizedCaller)
    }
}

pub struct RateLimiter {
    calls: HashMap<String, Vec<u64>>,
    window_secs: u64,
    max_calls: usize,
}

impl RateLimiter {
    pub fn new(window_secs: u64, max_calls: usize) -> Self {
        Self {
            calls: HashMap::new(),
            window_secs,
            max_calls,
        }
    }

    pub fn check(&mut self, caller: &str, now: u64) -> Result<(), SecurityError> {
        let window_start = now.saturating_sub(self.window_secs);
        let history = self.calls.entry(caller.to_string()).or_default();
        history.retain(|&t| t >= window_start);
        if history.len() >= self.max_calls {
            return Err(SecurityError::RateLimitExceeded);
        }
        history.push(now);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_trade_amount() {
        assert!(validate_trade_amount(0).is_err());
        assert!(validate_trade_amount(1).is_ok());
        assert!(validate_trade_amount(MAX_TRADE_AMOUNT + 1).is_err());
    }

    #[test]
    fn test_validate_fee() {
        assert!(validate_fee(0).is_err());
        assert!(validate_fee(30).is_ok());
        assert!(validate_fee(10_001).is_err());
    }

    #[test]
    fn test_validate_slippage() {
        assert!(validate_slippage(5_001).is_err());
        assert!(validate_slippage(100).is_ok());
    }

    #[test]
    fn test_assert_authorized() {
        assert!(assert_authorized("alice", &["alice", "bob"]).is_ok());
        assert!(assert_authorized("eve", &["alice", "bob"]).is_err());
    }

    #[test]
    fn test_rate_limiter() {
        let mut rl = RateLimiter::new(60, 3);
        assert!(rl.check("alice", 0).is_ok());
        assert!(rl.check("alice", 1).is_ok());
        assert!(rl.check("alice", 2).is_ok());
        assert!(rl.check("alice", 3).is_err());
        assert!(rl.check("alice", 70).is_ok());
    }
}
