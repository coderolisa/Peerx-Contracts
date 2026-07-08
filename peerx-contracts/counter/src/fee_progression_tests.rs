use soroban_sdk::{Env, Symbol, Address};
use crate::fee_progression::{
    FeeProgression, AchievementCategory, Achievement, AchievementStatus, 
    FeeCalculationResult, TierProgressionInfo
};
use crate::tiers::UserTier;

#[test]
fn test_fee_calculation_without_achievements() {
    let env = Env::default();
    let mut fee_progression = FeeProgression::new(&env);
    let user = Address::generate(&env);
    let user_tier = UserTier::Novice;

    // Calculate fee without any achievements
    let result = fee_progression.calculate_effective_fee(&env, &user, &user_tier);

    assert_eq!(result.base_fee_bps, 30); // Novice base fee
    assert_eq!(result.achievement_discount_bps, 0); // No discounts
    assert_eq!(result.effective_fee_bps, 30); // Same as base
    assert_eq!(result.max_discount_bps, 9); // 30% of 30 = 9 bps
    assert_eq!(result.applied_discounts.len(), 0); // No discounts applied
}

#[test]
fn test_consistency_achievement_7_day_streak() {
    let env = Env::default();
    let mut fee_progression = FeeProgression::new(&env);
    let user = Address::generate(&env);
    let user_tier = UserTier::Trader;

    // Simulate 7-day trading streak
    let mut status = AchievementStatus {
        achievements: Vec::new(&env),
        current_streak: 6,
        last_trade_day: 0,
        max_loss_percentage: 0,
        leaderboard_rank: None,
        volume_30_days: 0,
        total_discount_bps: 0,
        last_recalculation: 0,
    };

    // Update streak to 7 days
    status.current_streak = 7;
    status.last_trade_day = env.ledger().timestamp() / (24 * 60 * 60);
    
    fee_progression.user_achievements.set(user.clone(), status);

    // Calculate fee - should include consistency discount
    let result = fee_progression.calculate_effective_fee(&env, &user, &user_tier);

    assert_eq!(result.base_fee_bps, 25); // Trader base fee
    assert_eq!(result.achievement_discount_bps, 2); // 2 bps consistency discount
    assert_eq!(result.effective_fee_bps, 23); // 25 - 2 = 23
    assert!(result.applied_discounts.contains(&AchievementCategory::Consistency));
}

#[test]
fn test_risk_management_achievement() {
    let env = Env::default();
    let mut fee_progression = FeeProgression::new(&env);
    let user = Address::generate(&env);
    let user_tier = UserTier::Expert;

    // Simulate user with good risk management (max 5% loss)
    let mut status = AchievementStatus {
        achievements: Vec::new(&env),
        current_streak: 0,
        last_trade_day: 0,
        max_loss_percentage: 4, // Good risk management
        leaderboard_rank: None,
        volume_30_days: 0,
        total_discount_bps: 0,
        last_recalculation: 0,
    };

    fee_progression.user_achievements.set(user.clone(), status);

    // Calculate fee - should include risk management discount
    let result = fee_progression.calculate_effective_fee(&env, &user, &user_tier);

    assert_eq!(result.base_fee_bps, 20); // Expert base fee
    assert_eq!(result.achievement_discount_bps, 3); // 3 bps risk management discount
    assert_eq!(result.effective_fee_bps, 17); // 20 - 3 = 17
    assert!(result.applied_discounts.contains(&AchievementCategory::RiskManagement));
}

#[test]
fn test_community_achievement_top_100() {
    let env = Env::default();
    let mut fee_progression = FeeProgression::new(&env);
    let user = Address::generate(&env);
    let user_tier = UserTier::Whale;

    // Simulate user in top 100 leaderboard
    let mut status = AchievementStatus {
        achievements: Vec::new(&env),
        current_streak: 0,
        last_trade_day: 0,
        max_loss_percentage: 0,
        leaderboard_rank: Some(50), // Top 100
        volume_30_days: 0,
        total_discount_bps: 0,
        last_recalculation: 0,
    };

    fee_progression.user_achievements.set(user.clone(), status);

    // Calculate fee - should include community discount
    let result = fee_progression.calculate_effective_fee(&env, &user, &user_tier);

    assert_eq!(result.base_fee_bps, 15); // Whale base fee
    assert_eq!(result.achievement_discount_bps, 5); // 5 bps community discount
    assert_eq!(result.effective_fee_bps, 10); // 15 - 5 = 10
    assert!(result.applied_discounts.contains(&AchievementCategory::Community));
}

#[test]
fn test_volume_achievement_50k_xlm() {
    let env = Env::default();
    let mut fee_progression = FeeProgression::new(&env);
    let user = Address::generate(&env);
    let user_tier = UserTier::Trader;

    // Simulate user with high volume
    let mut status = AchievementStatus {
        achievements: Vec::new(&env),
        current_streak: 0,
        last_trade_day: 0,
        max_loss_percentage: 0,
        leaderboard_rank: None,
        volume_30_days: 60000, // 60k XLM volume
        total_discount_bps: 0,
        last_recalculation: 0,
    };

    fee_progression.user_achievements.set(user.clone(), status);

    // Calculate fee - should include volume discount
    let result = fee_progression.calculate_effective_fee(&env, &user, &user_tier);

    assert_eq!(result.base_fee_bps, 25); // Trader base fee
    assert_eq!(result.achievement_discount_bps, 4); // 4 bps volume discount
    assert_eq!(result.effective_fee_bps, 21); // 25 - 4 = 21
    assert!(result.applied_discounts.contains(&AchievementCategory::Volume));
}

#[test]
fn test_achievement_stacking_consistency() {
    let env = Env::default();
    let mut fee_progression = FeeProgression::new(&env);
    let user = Address::generate(&env);
    let user_tier = UserTier::Novice;

    // Simulate user with multiple consistency achievements (should stack up to 10 bps)
    let mut status = AchievementStatus {
        achievements: Vec::new(&env),
        current_streak: 0,
        last_trade_day: 0,
        max_loss_percentage: 0,
        leaderboard_rank: None,
        volume_30_days: 0,
        total_discount_bps: 0,
        last_recalculation: 0,
    };

    // Add multiple consistency achievements manually to test stacking
    for i in 0..5 {
        let achievement = Achievement {
            category: AchievementCategory::Consistency,
            discount_bps: 2,
            earned_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + (90 * 24 * 60 * 60),
            metadata: (7 + i * 7) as u64,
            is_active: true,
        };
        status.achievements.push_back(achievement);
    }

    fee_progression.user_achievements.set(user.clone(), status);

    // Calculate fee - should cap consistency discount at 10 bps
    let result = fee_progression.calculate_effective_fee(&env, &user, &user_tier);

    assert_eq!(result.base_fee_bps, 30); // Novice base fee
    assert_eq!(result.achievement_discount_bps, 10); // Capped at 10 bps
    assert_eq!(result.effective_fee_bps, 20); // 30 - 10 = 20
}

#[test]
fn test_discount_capping_30_percent_max() {
    let env = Env::default();
    let mut fee_progression = FeeProgression::new(&env);
    let user = Address::generate(&env);
    let user_tier = UserTier::Expert;

    // Create user with all possible achievements (should exceed 30% cap)
    let mut status = AchievementStatus {
        achievements: Vec::new(&env),
        current_streak: 0,
        last_trade_day: 0,
        max_loss_percentage: 0,
        leaderboard_rank: Some(1),
        volume_30_days: 100000,
        total_discount_bps: 0,
        last_recalculation: 0,
    };

    // Add all achievement types
    let achievements = vec![
        Achievement {
            category: AchievementCategory::Consistency,
            discount_bps: 10, // Max stacked
            earned_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + (90 * 24 * 60 * 60),
            metadata: 14,
            is_active: true,
        },
        Achievement {
            category: AchievementCategory::RiskManagement,
            discount_bps: 3,
            earned_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + (90 * 24 * 60 * 60),
            metadata: 4,
            is_active: true,
        },
        Achievement {
            category: AchievementCategory::Community,
            discount_bps: 5,
            earned_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + (90 * 24 * 60 * 60),
            metadata: 1,
            is_active: true,
        },
        Achievement {
            category: AchievementCategory::Volume,
            discount_bps: 4,
            earned_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + (90 * 24 * 60 * 60),
            metadata: 100000,
            is_active: true,
        },
    ];

    for achievement in achievements {
        status.achievements.push_back(achievement);
    }

    fee_progression.user_achievements.set(user.clone(), status);

    // Calculate fee - should be capped at 30% discount
    let result = fee_progression.calculate_effective_fee(&env, &user, &user_tier);

    assert_eq!(result.base_fee_bps, 20); // Expert base fee
    assert_eq!(result.max_discount_bps, 6); // 30% of 20 = 6 bps
    assert_eq!(result.achievement_discount_bps, 6); // Capped at 6 bps
    assert_eq!(result.effective_fee_bps, 14); // 20 - 6 = 14
}

#[test]
fn test_achievement_expiration() {
    let env = Env::default();
    let mut fee_progression = FeeProgression::new(&env);
    let user = Address::generate(&env);
    let user_tier = UserTier::Trader;

    // Create expired achievement
    let past_timestamp = env.ledger().timestamp() - (100 * 24 * 60 * 60); // 100 days ago
    let expired_achievement = Achievement {
        category: AchievementCategory::Consistency,
        discount_bps: 2,
        earned_at: past_timestamp,
        expires_at: past_timestamp + (90 * 24 * 60 * 60), // Expired 10 days ago
        metadata: 7,
        is_active: true,
    };

    let mut status = AchievementStatus {
        achievements: Vec::new(&env),
        current_streak: 0,
        last_trade_day: 0,
        max_loss_percentage: 0,
        leaderboard_rank: None,
        volume_30_days: 0,
        total_discount_bps: 0,
        last_recalculation: 0,
    };

    status.achievements.push_back(expired_achievement);
    fee_progression.user_achievements.set(user.clone(), status);

    // Calculate fee - expired achievement should not be counted
    let result = fee_progression.calculate_effective_fee(&env, &user, &user_tier);

    assert_eq!(result.base_fee_bps, 25); // Trader base fee
    assert_eq!(result.achievement_discount_bps, 0); // No active discounts
    assert_eq!(result.effective_fee_bps, 25); // Same as base
    assert_eq!(result.applied_discounts.len(), 0); // No active discounts
}

#[test]
fn test_apply_achievement_bonus() {
    let env = Env::default();
    let mut fee_progression = FeeProgression::new(&env);
    let user = Address::generate(&env);

    // Apply new achievement bonus
    let achievement = Achievement {
        category: AchievementCategory::RiskManagement,
        discount_bps: 3,
        earned_at: env.ledger().timestamp(),
        expires_at: env.ledger().timestamp() + (90 * 24 * 60 * 60),
        metadata: 5,
        is_active: true,
    };

    let result = fee_progression.apply_achievement_bonus(&env, &user, achievement);
    assert!(result.is_ok());

    // Verify achievement was applied
    let status = fee_progression.get_achievement_status(&user).unwrap();
    assert!(status.achievements.iter().any(|a| a.category == AchievementCategory::RiskManagement));
}

#[test]
fn test_duplicate_achievement_prevention() {
    let env = Env::default();
    let mut fee_progression = FeeProgression::new(&env);
    let user = Address::generate(&env);

    // Apply risk management achievement twice
    let achievement = Achievement {
        category: AchievementCategory::RiskManagement,
        discount_bps: 3,
        earned_at: env.ledger().timestamp(),
        expires_at: env.ledger().timestamp() + (90 * 24 * 60 * 60),
        metadata: 5,
        is_active: true,
    };

    // First application should succeed
    let result1 = fee_progression.apply_achievement_bonus(&env, &user, achievement.clone());
    assert!(result1.is_ok());

    // Second application should fail
    let result2 = fee_progression.apply_achievement_bonus(&env, &user, achievement);
    assert!(result2.is_err());
    assert_eq!(result2.unwrap_err(), "Achievement already active");
}

#[test]
fn test_tier_progression_info() {
    let env = Env::default();
    let mut fee_progression = FeeProgression::new(&env);
    let user = Address::generate(&env);

    // Get tier progression info
    let progression = fee_progression.check_tier_progression(&env, &user);

    assert_eq!(progression.current_tier, UserTier::Novice);
    assert_eq!(progression.next_tier, UserTier::Trader);
    assert_eq!(progression.trades_to_next_tier, 10);
    assert_eq!(progression.volume_to_next_tier, 100);
    assert_eq!(progression.current_trades, 0);
    assert_eq!(progression.current_volume, 0);
    assert_eq!(progression.achievement_count, 0);
}

#[test]
fn test_update_trading_activity() {
    let env = Env::default();
    let mut fee_progression = FeeProgression::new(&env);
    let user = Address::generate(&env);

    // Update trading activity
    fee_progression.update_trading_activity(&env, &user, 5000i128, Some(3));

    // Verify the update
    let status = fee_progression.get_achievement_status(&user).unwrap();
    assert_eq!(status.volume_30_days, 5000);
    assert_eq!(status.max_loss_percentage, 3);
}

#[test]
fn test_non_stackable_achievement_combination() {
    let env = Env::default();
    let mut fee_progression = FeeProgression::new(&env);
    let user = Address::generate(&env);
    let user_tier = UserTier::Novice;

    // Create user with consistency (stackable) and community (non-stackable)
    let mut status = AchievementStatus {
        achievements: Vec::new(&env),
        current_streak: 14, // 14-day streak (2 achievements worth)
        last_trade_day: 0,
        max_loss_percentage: 0,
        leaderboard_rank: Some(50), // Community achievement
        volume_30_days: 0,
        total_discount_bps: 0,
        last_recalculation: 0,
    };

    // Add achievements manually
    let consistency_achievement = Achievement {
        category: AchievementCategory::Consistency,
        discount_bps: 4, // 2 streaks worth
        earned_at: env.ledger().timestamp(),
        expires_at: env.ledger().timestamp() + (90 * 24 * 60 * 60),
        metadata: 14,
        is_active: true,
    };

    let community_achievement = Achievement {
        category: AchievementCategory::Community,
        discount_bps: 5,
        earned_at: env.ledger().timestamp(),
        expires_at: env.ledger().timestamp() + (90 * 24 * 60 * 60),
        metadata: 50,
        is_active: true,
    };

    status.achievements.push_back(consistency_achievement);
    status.achievements.push_back(community_achievement);
    fee_progression.user_achievements.set(user.clone(), status);

    // Calculate fee - should get 4 (consistency) + 5 (community) = 9 bps
    let result = fee_progression.calculate_effective_fee(&env, &user, &user_tier);

    assert_eq!(result.base_fee_bps, 30); // Novice base fee
    assert_eq!(result.achievement_discount_bps, 9); // 4 + 5 = 9
    assert_eq!(result.effective_fee_bps, 21); // 30 - 9 = 21
    assert_eq!(result.applied_discounts.len(), 2); // Both discounts applied
}

#[test]
fn test_fee_calculation_accuracy() {
    let env = Env::default();
    let mut fee_progression = FeeProgression::new(&env);
    let user = Address::generate(&env);
    let user_tier = UserTier::Expert;

    // Set up user with specific achievements
    let mut status = AchievementStatus {
        achievements: Vec::new(&env),
        current_streak: 0,
        last_trade_day: 0,
        max_loss_percentage: 4,
        leaderboard_rank: Some(25),
        volume_30_days: 75000,
        total_discount_bps: 0,
        last_recalculation: 0,
    };

    // Add risk management and volume achievements
    let risk_achievement = Achievement {
        category: AchievementCategory::RiskManagement,
        discount_bps: 3,
        earned_at: env.ledger().timestamp(),
        expires_at: env.ledger().timestamp() + (90 * 24 * 60 * 60),
        metadata: 4,
        is_active: true,
    };

    let volume_achievement = Achievement {
        category: AchievementCategory::Volume,
        discount_bps: 4,
        earned_at: env.ledger().timestamp(),
        expires_at: env.ledger().timestamp() + (90 * 24 * 60 * 60),
        metadata: 75000,
        is_active: true,
    };

    status.achievements.push_back(risk_achievement);
    status.achievements.push_back(volume_achievement);
    fee_progression.user_achievements.set(user.clone(), status);

    // Test with specific swap amount
    let swap_amount = 10000i128; // 100.00 tokens
    let result = fee_progression.calculate_effective_fee(&env, &user, &user_tier);

    // Expected: 20 bps base - 3 bps (risk) - 4 bps (volume) = 13 bps effective
    assert_eq!(result.base_fee_bps, 20);
    assert_eq!(result.achievement_discount_bps, 7); // 3 + 4 = 7
    assert_eq!(result.effective_fee_bps, 13); // 20 - 7 = 13
    
    // Verify actual fee calculation
    let expected_fee = (swap_amount * 13) / 10000; // Should be 13 tokens
    let actual_fee = (swap_amount * result.effective_fee_bps as i128) / 10000;
    assert_eq!(actual_fee, expected_fee);
}
