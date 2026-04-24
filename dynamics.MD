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


impl UserTier {
    /// Returns the base fee in basis points (bps) for this tier
    /// 1 bps = 0.01%, so 30 bps = 0.3%
    pub fn effective_fee_bps(&self) -> u32 {
        match self {
            UserTier::Novice => 30, // 0.3%
            UserTier::Trader => 25, // 0.25%
            UserTier::Expert => 20, // 0.20%
            UserTier::Whale => 15, // 0.15%
        }
    }

    /// Calculate the fee amount for a given swap amount (base fee only)
    /// swap_amount should be in the smallest unit (e.g., with decimals)
    pub fn calculate_fee(&self, swap_amount: i128) -> i128 {
        let bps = self.effective_fee_bps() as i128;
        // Fee = (swap_amount * bps) / 10000
        // Using integer arithmetic to avoid floating point
        (swap_amount * bps) / 10000
    }

    /// Calculate effective fee with achievement discounts applied
    /// This method integrates with the FeeProgression system
    pub fn calculate_effective_fee_with_achievements(
        &self,
        fee_progression: &mut FeeProgression,
        env: &soroban_sdk::Env,
        user: &soroban_sdk::Address,
    ) -> crate::fee_progression::FeeCalculationResult {
        fee_progression.calculate_effective_fee(env, user, self)
    }
}