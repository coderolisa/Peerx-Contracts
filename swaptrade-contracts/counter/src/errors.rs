use soroban_sdk::contracterror;

/// Main contract errors for SwapTrade
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SwapTradeError {
    NotAdmin = 1,
    TradingPaused = 2,
}

/// Extended errors including security/validation errors
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ContractError {
    /// Invalid amount provided (zero or negative)
    InvalidAmount = 100,
    /// Amount exceeds maximum allowed
    AmountOverflow = 101,
    /// Invalid token symbol
    InvalidTokenSymbol = 102,
    /// Invalid swap pair (same token)
    InvalidSwapPair = 103,
    /// Insufficient balance for operation
    InsufficientBalance = 104,
    /// Zero amount swap not allowed
    ZeroAmountSwap = 105,
    /// Contract invariant violation - security issue
    InvariantViolation = 200,
    /// Price oracle data is stale
    StalePrice = 201,
    /// Invalid price from oracle
    InvalidPrice = 202,
    /// Price not set in oracle
    PriceNotSet = 203,
    /// Rate limit exceeded
    RateLimitExceeded = 300,
    /// Slippage tolerance exceeded
    SlippageExceeded = 301,
    /// LP position not found
    LPPositionNotFound = 400,
    /// Insufficient LP tokens
    InsufficientLPTokens = 401,
}
