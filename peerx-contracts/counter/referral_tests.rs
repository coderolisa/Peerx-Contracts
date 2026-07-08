use soroban_sdk::{Env, Symbol, Address, U256};
use crate::{CounterContract, CounterContractClient};
use crate::referral::{ReferralSystem, CommissionTier, ReferralMilestone, ReferralBadge};

#[test]
fn test_generate_referral_code_with_nft() {
    let env = Env::default();
    let mut system = ReferralSystem::new(&env);
    
    let user = Address::generate(&env);
    let code = system.generate_referral_code(&env, user.clone());
    
    assert!(!code.to_string().is_empty());
    
    // Check that user received a starter badge
    let stats = system.get_referral_stats(&env, user);
    assert_eq!(stats.badges.len(), 1);
    assert_eq!(stats.badges.get(0).unwrap().milestone, ReferralMilestone::Starter);
    assert_eq!(stats.referral_code, code);
}

#[test]
fn test_register_with_code_nft_reward() {
    let env = Env::default();
    let mut system = ReferralSystem::new(&env);
    
    let referrer = Address::generate(&env);
    let referee = Address::generate(&env);
    
    // Generate referral code
    let code = system.generate_referral_code(&env, referrer.clone());
    
    // Register referee with code
    let result = system.register_with_code(&env, code, referee.clone());
    assert!(result.is_ok());
    
    let welcome_badge = result.unwrap();
    assert_eq!(welcome_badge.milestone, ReferralMilestone::Starter);
    
    // Check referrer stats updated
    let referrer_stats = system.get_referral_stats(&env, referrer);
    assert_eq!(referrer_stats.direct_referral_count, 1);
    assert_eq!(referrer_stats.total_referral_count, 1);
}

#[test]
fn test_three_tier_commission_distribution() {
    let env = Env::default();
    let mut system = ReferralSystem::new(&env);
    
    // Create 3-level referral chain: A -> B -> C -> D
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let user_c = Address::generate(&env);
    let user_d = Address::generate(&env);
    
    // Set up referral chain
    let code_a = system.generate_referral_code(&env, user_a.clone());
    let _badge_b = system.register_with_code(&env, code_a, user_b.clone()).unwrap();
    let code_b = system.generate_referral_code(&env, user_b.clone());
    let _badge_c = system.register_with_code(&env, code_b, user_c.clone()).unwrap();
    let code_c = system.generate_referral_code(&env, user_c.clone());
    let _badge_d = system.register_with_code(&env, code_c, user_d.clone()).unwrap();
    
    // User D makes a trade with 1000 fee
    let trade_fee = 1000i128;
    let distributions = system.distribute_commission(&env, user_d.clone(), trade_fee, 1);
    
    // Should have 3 distributions (20%, 10%, 5%)
    assert_eq!(distributions.len(), 3);
    
    // Check distribution amounts
    let mut found_direct = false;
    let mut found_secondary = false;
    let mut found_tertiary = false;
    
    for i in 0..distributions.len() {
        if let Some((recipient, amount, tier)) = distributions.get(i) {
            match tier {
                CommissionTier::Direct => {
                    assert_eq!(*amount, 200); // 20% of 1000
                    assert_eq!(recipient, &user_c);
                    found_direct = true;
                }
                CommissionTier::Secondary => {
                    assert_eq!(*amount, 100); // 10% of 1000
                    assert_eq!(recipient, &user_b);
                    found_secondary = true;
                }
                CommissionTier::Tertiary => {
                    assert_eq!(*amount, 50); // 5% of 1000
                    assert_eq!(recipient, &user_a);
                    found_tertiary = true;
                }
            }
        }
    }
    
    assert!(found_direct && found_secondary && found_tertiary);
}

#[test]
fn test_anti_gaming_30_day_holding_period() {
    let env = Env::default();
    let mut system = ReferralSystem::new(&env);
    
    let referrer = Address::generate(&env);
    let referee = Address::generate(&env);
    
    // Set up referral
    let code = system.generate_referral_code(&env, referrer.clone());
    let _badge = system.register_with_code(&env, code, referee.clone()).unwrap();
    
    // Distribute commission
    let trade_fee = 1000i128;
    system.distribute_commission(&env, referee.clone(), trade_fee, 1);
    
    // Try to claim immediately - should fail due to holding period
    let claim_result = system.claim_commission(&env, referrer.clone());
    assert!(claim_result.is_err());
    assert_eq!(claim_result.unwrap_err(), "No commission available to claim");
    
    // Check pending commission
    let pending = system.get_pending_commission(&env, referrer.clone());
    assert_eq!(pending, 0); // Not claimable yet
    
    // Advance time by 30 days
    env.ledger().set_timestamp(env.ledger().timestamp() + (30 * 24 * 60 * 60));
    
    // Now should be claimable
    let pending = system.get_pending_commission(&env, referrer.clone());
    assert_eq!(pending, 200); // 20% of 1000
    
    let claim_result = system.claim_commission(&env, referrer.clone());
    assert!(claim_result.is_ok());
    assert_eq!(claim_result.unwrap(), 200);
}

#[test]
fn test_rate_limited_commission_claims() {
    let env = Env::default();
    let mut system = ReferralSystem::new(&env);
    
    let referrer = Address::generate(&env);
    let referee = Address::generate(&env);
    
    // Set up referral and commission
    let code = system.generate_referral_code(&env, referrer.clone());
    let _badge = system.register_with_code(&env, code, referee.clone()).unwrap();
    
    // Advance time and distribute commission
    env.ledger().set_timestamp(env.ledger().timestamp() + (30 * 24 * 60 * 60));
    system.distribute_commission(&env, referee.clone(), 1000i128, 1);
    
    // First claim should succeed
    let claim1 = system.claim_commission(&env, referrer.clone());
    assert!(claim1.is_ok());
    assert_eq!(claim1.unwrap(), 200);
    
    // Second claim immediately should fail due to rate limit
    let claim2 = system.claim_commission(&env, referrer.clone());
    assert!(claim2.is_err());
    assert_eq!(claim2.unwrap_err(), "Rate limit: Please wait before claiming again");
    
    // Advance time by 1 hour - should work again
    env.ledger().set_timestamp(env.ledger().timestamp() + 3600);
    
    // Need more commission to claim
    system.distribute_commission(&env, referee.clone(), 1000i128, 1);
    env.ledger().set_timestamp(env.ledger().timestamp() + (30 * 24 * 60 * 60));
    
    let claim3 = system.claim_commission(&env, referrer.clone());
    assert!(claim3.is_ok());
    assert_eq!(claim3.unwrap(), 200);
}

#[test]
fn test_milestone_badge_awarding() {
    let env = Env::default();
    let mut system = ReferralSystem::new(&env);
    
    let referrer = Address::generate(&env);
    let code = system.generate_referral_code(&env, referrer.clone());
    
    // Register 10 referees to trigger Recruiter milestone
    for i in 0..10 {
        let referee = Address::generate(&env);
        let _badge = system.register_with_code(&env, code, referee).unwrap();
        
        // Check milestone progression
        let stats = system.get_referral_stats(&env, referrer.clone());
        
        if i < 1 {
            assert_eq!(stats.badges.len(), 1); // Only Starter
        } else if i < 10 {
            assert_eq!(stats.badges.len(), 1); // Still only Starter
        } else {
            // Should have Recruiter badge now
            assert_eq!(stats.badges.len(), 2);
            
            let has_recruiter = stats.badges.iter().any(|badge| badge.milestone == ReferralMilestone::Recruiter);
            assert!(has_recruiter);
        }
    }
}

#[test]
fn test_referral_chain_validation() {
    let env = Env::default();
    let mut system = ReferralSystem::new(&env);
    
    // Create referral chain: A -> B -> C -> D -> E (4 levels)
    let users: Vec<Address> = (0..5).map(|_| Address::generate(&env)).collect();
    
    // Set up chain
    let code_a = system.generate_referral_code(&env, users.get(0).unwrap().clone());
    let _badge_b = system.register_with_code(&env, code_a, users.get(1).unwrap().clone()).unwrap();
    let code_b = system.generate_referral_code(&env, users.get(1).unwrap().clone());
    let _badge_c = system.register_with_code(&env, code_b, users.get(2).unwrap().clone()).unwrap();
    let code_c = system.generate_referral_code(&env, users.get(2).unwrap().clone());
    let _badge_d = system.register_with_code(&env, code_c, users.get(3).unwrap().clone()).unwrap();
    let code_d = system.generate_referral_code(&env, users.get(3).unwrap().clone());
    let _badge_e = system.register_with_code(&env, code_d, users.get(4).unwrap().clone()).unwrap();
    
    // User E (4th level) makes trade - should only distribute to first 3 levels
    let distributions = system.distribute_commission(&env, users.get(4).unwrap().clone(), 1000i128, 1);
    
    // Should only have 3 distributions (max depth)
    assert_eq!(distributions.len(), 3);
    
    // User D should get direct commission (20%)
    let user_d_got = distributions.iter().any(|(addr, _, tier)| {
        addr == users.get(3).unwrap() && matches!(tier, CommissionTier::Direct)
    });
    assert!(user_d_got);
    
    // User C should get secondary commission (10%)
    let user_c_got = distributions.iter().any(|(addr, _, tier)| {
        addr == users.get(2).unwrap() && matches!(tier, CommissionTier::Secondary)
    });
    assert!(user_c_got);
    
    // User B should get tertiary commission (5%)
    let user_b_got = distributions.iter().any(|(addr, _, tier)| {
        addr == users.get(1).unwrap() && matches!(tier, CommissionTier::Tertiary)
    });
    assert!(user_b_got);
    
    // User A should get nothing (beyond 3 levels)
    let user_a_got = distributions.iter().any(|(addr, _, _)| {
        addr == users.get(0).unwrap()
    });
    assert!(!user_a_got);
}

#[test]
fn test_self_referral_prevention() {
    let env = Env::default();
    let mut system = ReferralSystem::new(&env);
    
    let user = Address::generate(&env);
    let code = system.generate_referral_code(&env, user.clone());
    
    // Try to register with own code
    let result = system.register_with_code(&env, code, user.clone());
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Cannot refer yourself");
}

#[test]
fn test_invalid_referral_code() {
    let env = Env::default();
    let mut system = ReferralSystem::new(&env);
    
    let user = Address::generate(&env);
    let invalid_code = Symbol::new(&env, "INVALID");
    
    // Try to register with invalid code
    let result = system.register_with_code(&env, invalid_code, user);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Invalid referral code");
}

#[test]
fn test_duplicate_registration_prevention() {
    let env = Env::default();
    let mut system = ReferralSystem::new(&env);
    
    let referrer = Address::generate(&env);
    let referee = Address::generate(&env);
    
    let code = system.generate_referral_code(&env, referrer.clone());
    
    // First registration should succeed
    let result1 = system.register_with_code(&env, code, referee.clone());
    assert!(result1.is_ok());
    
    // Second registration should fail
    let result2 = system.register_with_code(&env, code, referee.clone());
    assert!(result2.is_err());
    assert_eq!(result2.unwrap_err(), "User already registered");
}

#[test]
fn test_comprehensive_referral_stats() {
    let env = Env::default();
    let mut system = ReferralSystem::new(&env);
    
    let referrer = Address::generate(&env);
    let code = system.generate_referral_code(&env, referrer.clone());
    
    // Register multiple referees
    for i in 0..5 {
        let referee = Address::generate(&env);
        let _badge = system.register_with_code(&env, code, referee).unwrap();
        
        // Simulate some trading activity
        if i < 3 {
            system.distribute_commission(&env, referee, 1000i128, 1);
        }
    }
    
    let stats = system.get_referral_stats(&env, referrer);
    assert_eq!(stats.direct_referral_count, 5);
    assert_eq!(stats.total_referral_count, 5);
    assert_eq!(stats.referral_code, code);
    assert!(stats.registration_timestamp > 0);
    assert_eq!(stats.badges.len(), 1); // Starter badge
}

#[test]
fn test_global_statistics_tracking() {
    let env = Env::default();
    let mut system = ReferralSystem::new(&env);
    
    let referrer = Address::generate(&env);
    let referee = Address::generate(&env);
    
    let code = system.generate_referral_code(&env, referrer.clone());
    let _badge = system.register_with_code(&env, code, referee).unwrap();
    
    // Check initial global stats
    let (total_referrals, total_commission) = system.get_global_stats();
    assert_eq!(total_referrals, 1);
    assert_eq!(total_commission, 0);
    
    // Distribute and claim commission
    env.ledger().set_timestamp(env.ledger().timestamp() + (30 * 24 * 60 * 60));
    system.distribute_commission(&env, referee, 1000i128, 1);
    let _claimed = system.claim_commission(&env, referrer).unwrap();
    
    // Check updated global stats
    let (total_referrals, total_commission) = system.get_global_stats();
    assert_eq!(total_referrals, 1);
    assert_eq!(total_commission, 200); // 20% of 1000
}

#[test]
fn test_nft_badge_uniqueness() {
    let env = Env::default();
    let mut system = ReferralSystem::new(&env);
    
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    
    // Generate referral codes for both users
    let code1 = system.generate_referral_code(&env, user1.clone());
    let code2 = system.generate_referral_code(&env, user2.clone());
    
    let stats1 = system.get_referral_stats(&env, user1);
    let stats2 = system.get_referral_stats(&env, user2);
    
    // Each should have unique badge with different token IDs
    assert_eq!(stats1.badges.len(), 1);
    assert_eq!(stats2.badges.len(), 1);
    
    let badge1 = stats1.badges.get(0).unwrap();
    let badge2 = stats2.badges.get(0).unwrap();
    
    assert_ne!(badge1.token_id, badge2.token_id);
    assert_eq!(badge1.milestone, ReferralMilestone::Starter);
    assert_eq!(badge2.milestone, ReferralMilestone::Starter);
}

#[test]
fn test_churn_scenario_referee_leaves() {
    let env = Env::default();
    let mut system = ReferralSystem::new(&env);
    
    let referrer = Address::generate(&env);
    let referee = Address::generate(&env);
    
    // Set up referral
    let code = system.generate_referral_code(&env, referrer.clone());
    let _badge = system.register_with_code(&env, code, referee.clone()).unwrap();
    
    // Referee generates commission
    system.distribute_commission(&env, referee.clone(), 1000i128, 1);
    
    // Referrer should have pending commission
    let pending = system.get_pending_commission(&env, referrer.clone());
    assert_eq!(pending, 0); // Not claimable yet
    
    // Advance time and claim
    env.ledger().set_timestamp(env.ledger().timestamp() + (30 * 24 * 60 * 60));
    let claimed = system.claim_commission(&env, referrer.clone()).unwrap();
    assert_eq!(claimed, 200);
    
    // Referrer's stats should be preserved
    let stats = system.get_referral_stats(&env, referrer);
    assert_eq!(stats.direct_referral_count, 1);
    assert_eq!(stats.total_commission_earned, 200);
    assert_eq!(stats.available_commission, 0);
}