#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Report(u32),          // report_id => report
    ReportCount,
    Reward(u32),          // report_id => reward amount
    Status(u32),          // report_id => status
    Reporter(u32),        // report_id => user
}


#[derive(Clone, PartialEq, Debug)]
#[contracttype]
pub enum Badge {
    /// Complete your first swap - achieved at 1+ trades
    FirstTrade,
    
    /// Become an experienced trader - achieved at 10+ trades
    Trader,
    
    /// Build significant wealth - achieved when balance ≥ 10x starting amount
    WealthBuilder,
    
    /// Provide liquidity to the ecosystem - achieved at 1+ LP deposits
    LiquidityProvider,
    
    /// Explore diverse trading pairs - achieved when trading with 5+ different token pairs
    Diversifier,
    
    /// Trade consistently across blocks - achieved when trading on 7+ different ledger heights
    Consistency,
}