use soroban_sdk::{contracttype, Address, Env, Symbol, Map, Vec};
use crate::tiers::UserTier;

/// Achievement categories for fee discounts
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum AchievementCategory {
    Consistency,    // 7-day trading streaks
    RiskManagement, // Max 5% loss per trade
    Community,      // Top 100 leaderboard
    Volume,         // 50k+ XLM traded in 30 days
}

/// Individual achievement with discount details
#[derive(Clone, Debug)]
#[contracttype]
pub struct Achievement {
    /// Achievement category
    pub category: AchievementCategory,
    /// Discount in basis points (1 bps = 0.01%)
    pub discount_bps: u32,
    /// When achievement was earned
    pub earned_at: u64,
    /// When achievement expires (90 days after earning)
    pub expires_at: u64,
    /// Achievement metadata (e.g., streak length, rank, volume)
    pub metadata: u64,
    /// Whether achievement is currently active
    pub is_active: bool,
}

/// User's achievement status and discount tracking
#[derive(Clone, Debug)]
#[contracttype]
pub struct AchievementStatus {
    /// All earned achievements
    pub achievements: Vec<Achievement>,
    /// Current trading streak (days)
    pub current_streak: u32,
    /// Last trading day timestamp
    pub last_trade_day: u64,
    /// Maximum loss percentage (for risk management)
    pub max_loss_percentage: u32,
    /// Current leaderboard rank
    pub leaderboard_rank: Option<u32>,
    /// 30-day trading volume
    pub volume_30_days: i128,
    /// Total achievement discounts (stacked)
    pub total_discount_bps: u32,
    /// Last time achievements were recalculated
    pub last_recalculation: u64,
}

/// Fee progression result with breakdown
#[derive(Clone, Debug)]
#[contracttype]
pub struct FeeCalculationResult {
    /// Base fee from user tier
    pub base_fee_bps: u32,
    /// Total achievement discount
    pub achievement_discount_bps: u32,
    /// Final effective fee after discounts
    pub effective_fee_bps: u32,
    /// Maximum allowed discount (30% of base fee)
    pub max_discount_bps: u32,
    /// Applied discounts breakdown
    pub applied_discounts: Vec<AchievementCategory>,
}

/// Fee progression engine for dynamic fee calculation
pub struct FeeProgression {
    /// User achievement status mapping
    user_achievements: Map<Address, AchievementStatus>,
    
    /// Global achievement definitions
    achievement_definitions: Map<AchievementCategory, AchievementDefinition>,
}

/// Achievement definition with criteria and rewards
#[derive(Clone, Debug)]
#[contracttype]
pub struct AchievementDefinition {
    /// Achievement category
    pub category: AchievementCategory,
    /// Discount awarded (in basis points)
    pub discount_bps: u32,
    /// Whether discounts can stack
    pub is_stackable: bool,
    /// Maximum stackable discount
    pub max_stackable_bps: u32,
    /// Achievement criteria
    pub criteria: AchievementCriteria,
}

/// Criteria for earning achievements
#[derive(Clone, Debug)]
#[contracttype]
pub struct AchievementCriteria {
    /// Minimum requirement value
    pub minimum_value: u64,
    /// Measurement type (days, percentage, rank, volume)
    pub measurement_type: MeasurementType,
    /// Time window for evaluation (if applicable)
    pub time_window_days: Option<u32>,
}

/// Measurement types for achievement criteria
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum MeasurementType {
    Days,
    Percentage,
    Rank,
    Volume,
}

impl FeeProgression {
    pub fn new(env: &Env) -> Self {
        let mut definitions = Map::new(env);
        
        // Consistency: 7-day trading streak → 2 bps discount (stackable up to 10 bps)
        definitions.set(
            AchievementCategory::Consistency,
            AchievementDefinition {
                category: AchievementCategory::Consistency,
                discount_bps: 2,
                is_stackable: true,
                max_stackable_bps: 10,
                criteria: AchievementCriteria {
                    minimum_value: 7,
                    measurement_type: MeasurementType::Days,
                    time_window_days: Some(7),
                },
            },
        );
        
        // Risk Management: Max 5% loss per trade → 3 bps discount
        definitions.set(
            AchievementCategory::RiskManagement,
            AchievementDefinition {
                category: AchievementCategory::RiskManagement,
                discount_bps: 3,
                is_stackable: false,
                max_stackable_bps: 3,
                criteria: AchievementCriteria {
                    minimum_value: 5,
                    measurement_type: MeasurementType::Percentage,
                    time_window_days: None,
                },
            },
        );
        
        // Community: Top 100 leaderboard position → 5 bps discount
        definitions.set(
            AchievementCategory::Community,
            AchievementDefinition {
                category: AchievementCategory::Community,
                discount_bps: 5,
                is_stackable: false,
                max_stackable_bps: 5,
                criteria: AchievementCriteria {
                    minimum_value: 100,
                    measurement_type: MeasurementType::Rank,
                    time_window_days: None,
                },
            },
        );
        
        // Volume: 50k+ XLM traded in 30 days → 4 bps discount
        definitions.set(
            AchievementCategory::Volume,
            AchievementDefinition {
                category: AchievementCategory::Volume,
                discount_bps: 4,
                is_stackable: false,
                max_stackable_bps: 4,
                criteria: AchievementCriteria {
                    minimum_value: 50000,
                    measurement_type: MeasurementType::Volume,
                    time_window_days: Some(30),
                },
            },
        );
        
        Self {
            user_achievements: Map::new(env),
            achievement_definitions: definitions,
        }
    }

    /// Calculate effective fee with achievement bonuses
    pub fn calculate_effective_fee(&mut self, env: &Env, user: &Address, user_tier: &UserTier) -> FeeCalculationResult {
        let base_fee_bps = user_tier.effective_fee_bps();
        let max_discount_bps = (base_fee_bps * 30) / 100; // Max 30% reduction
        
        // Get or create user achievement status
        let mut status = self.user_achievements.get(user.clone()).unwrap_or_else(|| {
            AchievementStatus {
                achievements: Vec::new(env),
                current_streak: 0,
                last_trade_day: 0,
                max_loss_percentage: 0,
                leaderboard_rank: None,
                volume_30_days: 0,
                total_discount_bps: 0,
                last_recalculation: 0,
            }
        });
        
        // Update achievement status based on current data
        self.update_achievement_status(env, user, &mut status);
        
        // Calculate total discount from active achievements
        let mut total_discount = 0u32;
        let mut applied_discounts = Vec::new(env);
        
        for achievement in status.achievements.iter() {
            if achievement.is_active {
                if let Some(definition) = self.achievement_definitions.get(achievement.category.clone()) {
                    if definition.is_stackable {
                        // Stackable achievements add up to max
                        let current_category_discount = total_discount;
                        let max_allowed = definition.max_stackable_bps;
                        if current_category_discount < max_allowed {
                            let additional = definition.discount_bps.min(max_allowed - current_category_discount);
                            total_discount += additional;
                        }
                    } else {
                        // Non-stackable achievements just add their discount
                        total_discount += achievement.discount_bps;
                    }
                    applied_discounts.push_back(achievement.category.clone());
                }
            }
        }
        
        // Cap discount at maximum allowed
        let final_discount = total_discount.min(max_discount_bps);
        let effective_fee_bps = base_fee_bps.saturating_sub(final_discount);
        
        // Update user status
        status.total_discount_bps = final_discount;
        status.last_recalculation = env.ledger().timestamp();
        self.user_achievements.set(user.clone(), status);
        
        FeeCalculationResult {
            base_fee_bps,
            achievement_discount_bps: final_discount,
            effective_fee_bps,
            max_discount_bps,
            applied_discounts,
        }
    }

    /// Check user's progression toward next tier
    pub fn check_tier_progression(&self, env: &Env, user: &Address) -> TierProgressionInfo {
        let status = self.user_achievements.get(user.clone()).unwrap_or_else(|| {
            AchievementStatus {
                achievements: Vec::new(env),
                current_streak: 0,
                last_trade_day: 0,
                max_loss_percentage: 0,
                leaderboard_rank: None,
                volume_30_days: 0,
                total_discount_bps: 0,
                last_recalculation: 0,
            }
        });
        
        // This would integrate with actual user statistics
        // For now, return basic progression info
        TierProgressionInfo {
            current_tier: UserTier::Novice, // Would be determined from actual user data
            next_tier: UserTier::Trader,
            trades_to_next_tier: 10,
            volume_to_next_tier: 100,
            current_trades: 0,
            current_volume: status.volume_30_days,
            achievement_count: status.achievements.len(),
        }
    }

    /// Apply achievement bonus to user
    pub fn apply_achievement_bonus(&mut self, env: &Env, user: &Address, achievement: Achievement) -> Result<(), &'static str> {
        let mut status = self.user_achievements.get(user.clone()).unwrap_or_else(|| {
            AchievementStatus {
                achievements: Vec::new(env),
                current_streak: 0,
                last_trade_day: 0,
                max_loss_percentage: 0,
                leaderboard_rank: None,
                volume_30_days: 0,
                total_discount_bps: 0,
                last_recalculation: 0,
            }
        });
        
        // Check if user already has this achievement type
        let has_existing = status.achievements.iter().any(|existing| {
            existing.category == achievement.category && existing.is_active
        });
        
        if has_existing {
            return Err("Achievement already active");
        }
        
        // Add new achievement
        status.achievements.push_back(achievement.clone());
        self.user_achievements.set(user.clone(), status);
        
        // Emit achievement event
        env.events().publish(
            (symbol_short!("achievement_earned"), user.clone(), achievement.category, achievement.discount_bps),
        );
        
        Ok(())
    }

    /// Update achievement status based on current user data
    fn update_achievement_status(&mut self, env: &Env, user: &Address, status: &mut AchievementStatus) {
        let current_timestamp = env.ledger().timestamp();
        
        // Check consistency achievement (7-day streak)
        self.check_consistency_achievement(env, status, current_timestamp);
        
        // Check risk management achievement
        self.check_risk_management_achievement(env, status);
        
        // Check community achievement (leaderboard)
        self.check_community_achievement(env, status);
        
        // Check volume achievement
        self.check_volume_achievement(env, status, current_timestamp);
        
        // Clean up expired achievements
        self.cleanup_expired_achievements(env, status, current_timestamp);
    }

    /// Check and update consistency achievement
    fn check_consistency_achievement(&self, env: &Env, status: &mut AchievementStatus, current_timestamp: u64) {
        if let Some(definition) = self.achievement_definitions.get(AchievementCategory::Consistency) {
            let current_day = current_timestamp / (24 * 60 * 60); // Convert to days
            
            // Check if user traded today (this would be updated by trading system)
            let traded_today = current_day == status.last_trade_day;
            
            if traded_today {
                status.current_streak += 1;
            } else {
                // Check if it's been more than 1 day since last trade
                if current_day > status.last_trade_day + 1 {
                    status.current_streak = 1; // Reset streak
                }
            }
            
            status.last_trade_day = current_day;
            
            // Check if streak qualifies for achievement
            if status.current_streak >= definition.criteria.minimum_value {
                let new_achievement = Achievement {
                    category: AchievementCategory::Consistency,
                    discount_bps: definition.discount_bps,
                    earned_at: current_timestamp,
                    expires_at: current_timestamp + (90 * 24 * 60 * 60), // 90 days
                    metadata: status.current_streak as u64,
                    is_active: true,
                };
                
                // Remove existing consistency achievement if any
                status.achievements.retain(|achievement| achievement.category != AchievementCategory::Consistency);
                
                // Add new achievement
                status.achievements.push_back(new_achievement);
                
                // Emit event
                env.events().publish(
                    (symbol_short!("streak_achievement"), status.current_streak, definition.discount_bps),
                );
            }
        }
    }

    /// Check and update risk management achievement
    fn check_risk_management_achievement(&self, env: &Env, status: &mut AchievementStatus) {
        if let Some(definition) = self.achievement_definitions.get(AchievementCategory::RiskManagement) {
            // This would be updated by trading system to track maximum loss
            // For now, assume user meets criteria if max_loss_percentage <= 5
            if status.max_loss_percentage <= definition.criteria.minimum_value {
                let current_timestamp = env.ledger().timestamp();
                
                // Check if user already has this achievement
                let has_achievement = status.achievements.iter().any(|achievement| {
                    achievement.category == AchievementCategory::RiskManagement && achievement.is_active
                });
                
                if !has_achievement {
                    let new_achievement = Achievement {
                        category: AchievementCategory::RiskManagement,
                        discount_bps: definition.discount_bps,
                        earned_at: current_timestamp,
                        expires_at: current_timestamp + (90 * 24 * 60 * 60),
                        metadata: status.max_loss_percentage as u64,
                        is_active: true,
                    };
                    
                    status.achievements.push_back(new_achievement);
                    
                    // Emit event
                    env.events().publish(
                        (symbol_short!("risk_achievement"), status.max_loss_percentage, definition.discount_bps),
                    );
                }
            }
        }
    }

    /// Check and update community achievement
    fn check_community_achievement(&self, env: &Env, status: &mut AchievementStatus) {
        if let Some(definition) = self.achievement_definitions.get(AchievementCategory::Community) {
            // This would be updated by leaderboard system
            // For now, assume user is in top 100 if rank <= 100
            if let Some(rank) = status.leaderboard_rank {
                if rank <= definition.criteria.minimum_value {
                    let current_timestamp = env.ledger().timestamp();
                    
                    // Check if user already has this achievement
                    let has_achievement = status.achievements.iter().any(|achievement| {
                        achievement.category == AchievementCategory::Community && achievement.is_active
                    });
                    
                    if !has_achievement {
                        let new_achievement = Achievement {
                            category: AchievementCategory::Community,
                            discount_bps: definition.discount_bps,
                            earned_at: current_timestamp,
                            expires_at: current_timestamp + (90 * 24 * 60 * 60),
                            metadata: rank as u64,
                            is_active: true,
                        };
                        
                        status.achievements.push_back(new_achievement);
                        
                        // Emit event
                        env.events().publish(
                            (symbol_short!("community_achievement"), rank, definition.discount_bps),
                        );
                    }
                }
            }
        }
    }

    /// Check and update volume achievement
    fn check_volume_achievement(&self, env: &Env, status: &mut AchievementStatus, current_timestamp: u64) {
        if let Some(definition) = self.achievement_definitions.get(AchievementCategory::Volume) {
            // Check if 30-day volume meets criteria
            if status.volume_30_days >= definition.criteria.minimum_value.into() {
                // Check if user already has this achievement
                let has_achievement = status.achievements.iter().any(|achievement| {
                    achievement.category == AchievementCategory::Volume && achievement.is_active
                });
                
                if !has_achievement {
                    let new_achievement = Achievement {
                        category: AchievementCategory::Volume,
                        discount_bps: definition.discount_bps,
                        earned_at: current_timestamp,
                        expires_at: current_timestamp + (90 * 24 * 60 * 60),
                        metadata: status.volume_30_days,
                        is_active: true,
                    };
                    
                    status.achievements.push_back(new_achievement);
                    
                    // Emit event
                    env.events().publish(
                        (symbol_short!("volume_achievement"), status.volume_30_days, definition.discount_bps),
                    );
                }
            }
        }
    }

    /// Remove expired achievements
    fn cleanup_expired_achievements(&self, env: &Env, status: &mut AchievementStatus, current_timestamp: u64) {
        let mut active_achievements = Vec::new(env);
        
        for achievement in status.achievements.iter() {
            if current_timestamp < achievement.expires_at {
                active_achievements.push_back(achievement.clone());
            } else {
                // Emit expiration event
                env.events().publish(
                    (symbol_short!("achievement_expired"), achievement.category, achievement.discount_bps),
                );
            }
        }
        
        status.achievements = active_achievements;
    }

    /// Get user's current achievement status
    pub fn get_achievement_status(&self, user: &Address) -> Option<AchievementStatus> {
        self.user_achievements.get(user.clone())
    }

    /// Update user trading data (called by trading system)
    pub fn update_trading_activity(&mut self, env: &Env, user: &Address, trade_volume: i128, loss_percentage: Option<u32>) {
        let mut status = self.user_achievements.get(user.clone()).unwrap_or_else(|| {
            AchievementStatus {
                achievements: Vec::new(env),
                current_streak: 0,
                last_trade_day: 0,
                max_loss_percentage: 0,
                leaderboard_rank: None,
                volume_30_days: 0,
                total_discount_bps: 0,
                last_recalculation: 0,
            }
        });
        
        // Update volume (simplified - would use proper rolling window in production)
        status.volume_30_days += trade_volume;
        
        // Update max loss percentage
        if let Some(loss_pct) = loss_percentage {
            status.max_loss_percentage = status.max_loss_percentage.max(loss_pct);
        }
        
        self.user_achievements.set(user.clone(), status);
        
        // Trigger achievement recalculation
        self.calculate_effective_fee(env, user, &UserTier::Novice); // Tier would be determined from user data
    }
}

/// Information about tier progression
#[derive(Clone, Debug)]
#[contracttype]
pub struct TierProgressionInfo {
    /// Current user tier
    pub current_tier: UserTier,
    /// Next achievable tier
    pub next_tier: UserTier,
    /// Trades needed for next tier
    pub trades_to_next_tier: u32,
    /// Volume needed for next tier
    pub volume_to_next_tier: i128,
    /// Current trade count
    pub current_trades: u32,
    /// Current trading volume
    pub current_volume: i128,
    /// Number of active achievements
    pub achievement_count: u32,
}
