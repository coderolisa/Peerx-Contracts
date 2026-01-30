use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SwapTradeError {
    NotAdmin = 1,
    TradingPaused = 2,
}
