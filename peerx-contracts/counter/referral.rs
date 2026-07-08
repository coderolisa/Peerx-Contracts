use soroban_sdk::{contracttype, Address, Env, Symbol, Map, Vec, U256};
use crate::rate_limit::TimeWindow;

/// Commission tiers for referral structure
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum CommissionTier {
    Direct = 20,    // 20% for direct referrals
    Secondary = 10, // 10% for secondary referrals
    Tertiary = 5,   // 5% for tertiary referrals
}

/// Referral milestone badges
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum ReferralMilestone {
    Starter,        // 1 referral
    Recruiter,      // 10 referrals
    Influencer,     // 100 referrals
    Ambassador,     // 1000 referrals
    Legend,         // 10000 referrals
}

/// NFT badge for referral achievements
#[derive(Clone, Debug)]
#[contracttype]
pub struct ReferralBadge {
    /// Badge type/milestone
    pub milestone: ReferralMilestone,
    /// When the badge was earned
    pub earned_at: u64,
    /// Unique token ID for the NFT
    pub token_id: U256,
    /// Referral code associated with this badge
    pub referral_code: Symbol,
}

/// Enhanced referral information with chain support
#[derive(Clone, Debug)]
#[contracttype]
pub struct ReferralInfo {
    /// User's unique referral code
    pub referral_code: Symbol,
    /// Direct referrer (if any)
    pub referrer: Option<Address>,
    /// Timestamp when user registered
    pub registration_timestamp: u64,
    /// Total commission earned (in smallest unit)
    pub total_commission_earned: i128,
    /// Commission currently available to claim
    pub available_commission: i128,
    /// Number of direct referrals
    pub direct_referral_count: u32,
    /// Total referral count (all levels)
    pub total_referral_count: u32,
    /// Last commission claim timestamp
    pub last_claim_timestamp: u64,
    /// Earned badges
    pub badges: Vec<ReferralBadge>,
    /// Trading volume from referrals (for tier calculations)
    pub referral_trading_volume: i128,
}

/// Commission claim record for anti-gaming
#[derive(Clone, Debug)]
#[contracttype]
pub struct CommissionRecord {
    /// Amount of commission
    pub amount: i128,
    /// When it was earned
    pub earned_at: u64,
    /// When it becomes claimable (30 days later)
    pub claimable_at: u64,
    /// Source of commission (which referee)
    pub source: Address,
    /// Commission tier
    pub tier: CommissionTier,
}

/// Enhanced Referral System with multi-tier support and NFT integration
#[derive(Clone)]
#[contracttype]
pub struct ReferralSystem {
    // Maps user addresses to their referral information
    referral_info: Map<Address, ReferralInfo>,
    
    // Maps referral codes to user addresses
    code_to_user: Map<Symbol, Address>,
    
    // Pending commission records (for 30-day holding)
    pending_commissions: Map<Address, Vec<CommissionRecord>>,
    
    // Rate limiting for commission claims
    claim_rate_limits: Map<Address, u64>, // last claim timestamp
    
    // NFT token counter for unique badge IDs
    next_token_id: U256,
    
    // Global referral statistics
    total_referrals: u32,
    total_commission_distributed: i128,
}

impl ReferralSystem {
    pub fn new(env: &Env) -> Self {
        Self {
            referral_info: Map::new(env),
            code_to_user: Map::new(env),
            pending_commissions: Map::new(env),
            claim_rate_limits: Map::new(env),
            next_token_id: U256::from_u32(1),
            total_referrals: 0,
            total_commission_distributed: 0,
        }
    }

    /// Generate a unique referral code for a user with NFT proof
    pub fn generate_referral_code(&mut self, env: &Env, user: Address) -> Symbol {
        // Check if user already has a referral code
        if let Some(info) = self.referral_info.get(user.clone()) {
            return info.referral_code;
        }

        // Generate a unique 8-character alphanumeric referral code
        let code = self.generate_unique_code(env);
        
        // Create referral info for the user
        let info = ReferralInfo {
            referral_code: code,
            referrer: None,
            registration_timestamp: env.ledger().timestamp(),
            total_commission_earned: 0,
            available_commission: 0,
            direct_referral_count: 0,
            total_referral_count: 0,
            last_claim_timestamp: 0,
            badges: Vec::new(env),
            referral_trading_volume: 0,
        };
        
        // Store mappings
        self.referral_info.set(user.clone(), info.clone());
        self.code_to_user.set(code, user);

        // Mint initial NFT badge for referral code generation
        self.mint_referral_badge(env, user, ReferralMilestone::Starter, code);

        code
    }

    /// Register a new user with a referral code and return rewards NFT
    pub fn register_with_code(&mut self, env: &Env, referral_code: Symbol, new_user: Address) -> Result<ReferralBadge, &'static str> {
        // Check if user already exists
        if self.referral_info.contains_key(new_user.clone()) {
            return Err("User already registered");
        }

        // Validate referral code exists
        let referrer = self.code_to_user.get(referral_code)
            .ok_or("Invalid referral code")?;

        // Prevent self-referral
        if referrer == new_user {
            return Err("Cannot refer yourself");
        }

        // Create referral info for new user
        let user_info = ReferralInfo {
            referral_code: Symbol::new(env, ""), // No code yet
            referrer: Some(referrer.clone()),
            registration_timestamp: env.ledger().timestamp(),
            total_commission_earned: 0,
            available_commission: 0,
            direct_referral_count: 0,
            total_referral_count: 0,
            last_claim_timestamp: 0,
            badges: Vec::new(env),
            referral_trading_volume: 0,
        };

        // Store new user info
        self.referral_info.set(new_user.clone(), user_info);

        // Update referrer's referral count
        if let Some(mut referrer_info) = self.referral_info.get(referrer.clone()) {
            referrer_info.direct_referral_count += 1;
            referrer_info.total_referral_count += 1;
            self.referral_info.set(referrer.clone(), referrer_info.clone());
            
            // Check for milestone badges
            self.check_and_award_milestones(env, referrer, &referrer_info);
        }

        // Update global statistics
        self.total_referrals += 1;

        // Mint welcome badge for new user
        let welcome_badge = self.mint_referral_badge(env, new_user, ReferralMilestone::Starter, Symbol::new(env, "WELCOME"));

        Ok(welcome_badge)
    }

    /// Distribute commission across 3-tier referral chain
    pub fn distribute_commission(&mut self, env: &Env, trader: Address, trade_fee: i128, fee_tier: u32) -> Vec<(Address, i128, CommissionTier)> {
        let mut distributions = Vec::new(env);
        let current_timestamp = env.ledger().timestamp();
        
        // Get the referral chain (up to 3 levels)
        let referral_chain = self.get_referral_chain(env, trader, 3);
        
        for (level, referrer) in referral_chain.iter().enumerate() {
            let tier = match level {
                0 => CommissionTier::Direct,
                1 => CommissionTier::Secondary,
                2 => CommissionTier::Tertiary,
                _ => break, // Only 3 tiers supported
            };
            
            let commission_rate = match tier {
                CommissionTier::Direct => 20,
                CommissionTier::Secondary => 10,
                CommissionTier::Tertiary => 5,
            };
            
            let commission_amount = (trade_fee * commission_rate as i128) / 100;
            
            if commission_amount > 0 {
                // Create commission record with 30-day holding period
                let record = CommissionRecord {
                    amount: commission_amount,
                    earned_at: current_timestamp,
                    claimable_at: current_timestamp + (30 * 24 * 60 * 60), // 30 days
                    source: trader.clone(),
                    tier,
                };
                
                // Add to pending commissions
                let mut pending = self.pending_commissions.get(referrer.clone()).unwrap_or_else(|| Vec::new(env));
                pending.push_back(record);
                self.pending_commissions.set(referrer.clone(), pending);
                
                distributions.push_back((referrer.clone(), commission_amount, tier));
            }
        }
        
        distributions
    }

    /// Get comprehensive referral statistics for a user
    pub fn get_referral_stats(&self, env: &Env, user: Address) -> ReferralInfo {
        self.referral_info.get(user).unwrap_or_else(|| ReferralInfo {
            referral_code: Symbol::new(env, ""),
            referrer: None,
            registration_timestamp: 0,
            total_commission_earned: 0,
            available_commission: 0,
            direct_referral_count: 0,
            total_referral_count: 0,
            last_claim_timestamp: 0,
            badges: Vec::new(env),
            referral_trading_volume: 0,
        })
    }

    /// Claim available commission with rate limiting
    pub fn claim_commission(&mut self, env: &Env, user: Address) -> Result<i128, &'static str> {
        let current_timestamp = env.ledger().timestamp();
        
        // Rate limiting: max one claim per hour
        if let Some(last_claim) = self.claim_rate_limits.get(user.clone()) {
            if current_timestamp < last_claim + 3600 {
                return Err("Rate limit: Please wait before claiming again");
            }
        }
        
        // Process pending commissions
        let mut total_claimable = 0i128;
        let mut remaining_pending = Vec::new(env);
        
        if let Some(pending) = self.pending_commissions.get(user.clone()) {
            for record in pending.iter() {
                if current_timestamp >= record.claimable_at {
                    total_claimable += record.amount;
                } else {
                    remaining_pending.push_back(record);
                }
            }
        }
        
        if total_claimable == 0 {
            return Err("No commission available to claim");
        }
        
        // Update user info
        if let Some(mut info) = self.referral_info.get(user.clone()) {
            info.available_commission -= total_claimable;
            info.total_commission_earned += total_claimable;
            info.last_claim_timestamp = current_timestamp;
            self.referral_info.set(user.clone(), info);
        }
        
        // Update pending commissions
        if remaining_pending.is_empty() {
            self.pending_commissions.remove(user);
        } else {
            self.pending_commissions.set(user, remaining_pending);
        }
        
        // Update rate limit
        self.claim_rate_limits.set(user, current_timestamp);
        
        // Update global statistics
        self.total_commission_distributed += total_claimable;
        
        Ok(total_claimable)
    }

    /// Get referral chain up to specified depth
    fn get_referral_chain(&self, env: &Env, user: Address, max_depth: usize) -> Vec<Address> {
        let mut chain = Vec::new(env);
        let mut current_user = user;
        
        for _ in 0..max_depth {
            if let Some(info) = self.referral_info.get(current_user.clone()) {
                if let Some(referrer) = info.referrer {
                    chain.push_back(referrer.clone());
                    current_user = referrer;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        
        chain
    }

    /// Check and award milestone badges
    fn check_and_award_milestones(&mut self, env: &Env, user: Address, info: &ReferralInfo) {
        let milestones = [
            (1, ReferralMilestone::Starter),
            (10, ReferralMilestone::Recruiter),
            (100, ReferralMilestone::Influencer),
            (1000, ReferralMilestone::Ambassador),
            (10000, ReferralMilestone::Legend),
        ];
        
        for (threshold, milestone) in milestones.iter() {
            if info.direct_referral_count >= *threshold {
                // Check if badge already earned
                let has_badge = info.badges.iter().any(|badge| badge.milestone == *milestone);
                if !has_badge {
                    self.mint_referral_badge(env, user.clone(), milestone.clone(), info.referral_code);
                }
            }
        }
    }

    /// Mint NFT badge for achievement
    fn mint_referral_badge(&mut self, env: &Env, user: Address, milestone: ReferralMilestone, referral_code: Symbol) -> ReferralBadge {
        let token_id = self.next_token_id;
        self.next_token_id = token_id + U256::from_u32(1);
        
        let badge = ReferralBadge {
            milestone,
            earned_at: env.ledger().timestamp(),
            token_id,
            referral_code,
        };
        
        // Update user's badges
        if let Some(mut info) = self.referral_info.get(user.clone()) {
            info.badges.push_back(badge.clone());
            self.referral_info.set(user, info);
        }
        
        badge
    }

    /// Generate a unique referral code
    fn generate_unique_code(&self, env: &Env) -> Symbol {
        let mut attempts = 0;
        loop {
            let code_str = self.create_random_code(env, attempts);
            let code = Symbol::new(env, &code_str);
            
            if !self.code_to_user.contains_key(code) {
                return code;
            }
            
            attempts += 1;
            if attempts > 1000 {
                panic!("Could not generate unique referral code after 1000 attempts");
            }
        }
    }

    /// Create a random-looking referral code
    fn create_random_code(&self, env: &Env, attempt: u32) -> String {
        let ledger_seq = env.ledger().sequence();
        let seed = ledger_seq as u64 + attempt as u64;
        
        let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut result = String::new();
        let mut temp_seed = seed;
        
        for _ in 0..8 {
            let idx = (temp_seed % 36) as usize;
            if let Some(c) = chars.chars().nth(idx) {
                result.push(c);
            }
            temp_seed /= 36;
        }
        
        while result.len() < 8 {
            result.push('A');
        }
        
        result[..8.min(result.len())].to_string()
    }

    /// Get pending commission amount for a user
    pub fn get_pending_commission(&self, env: &Env, user: Address) -> i128 {
        if let Some(pending) = self.pending_commissions.get(user.clone()) {
            let current_timestamp = env.ledger().timestamp();
            let mut total = 0i128;
            
            for record in pending.iter() {
                if current_timestamp >= record.claimable_at {
                    total += record.amount;
                }
            }
            
            total
        } else {
            0
        }
    }

    /// Get global referral statistics
    pub fn get_global_stats(&self) -> (u32, i128) {
        (self.total_referrals, self.total_commission_distributed)
    }
}