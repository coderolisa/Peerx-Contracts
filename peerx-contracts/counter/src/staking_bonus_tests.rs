/// Tests for Staking Bonus System
/// 
/// Covers all staking operations, time locks, distributions, and transparency features

#[cfg(test)]
mod staking_bonus_tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    // ────────────────────────────────────────────────────────────────────────
    // Basic Staking Tests
    // ────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_stake_30_days() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        let result = manager.stake(&env, user.clone(), 1000, 30);
        assert!(result.is_ok());

        let stake_id = result.unwrap();
        assert_eq!(stake_id, 0);

        // Verify stake was recorded
        let stakes = StakingBonusManager::get_user_stakes(&env, user.clone());
        assert_eq!(stakes.len(), 1);

        let stake = stakes.get(0).unwrap();
        assert_eq!(stake.amount, 1000);
        assert_eq!(stake.bonus_bps, 500); // 5%
        assert_eq!(stake.bonus_amount, 50); // 5% of 1000
        assert!(stake.is_active);
    }

    #[test]
    fn test_stake_60_days() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        manager.stake(&env, user.clone(), 1000, 60).unwrap();

        let stakes = StakingBonusManager::get_user_stakes(&env, user);
        let stake = stakes.get(0).unwrap();

        assert_eq!(stake.bonus_bps, 1200); // 12%
        assert_eq!(stake.bonus_amount, 120); // 12% of 1000
    }

    #[test]
    fn test_stake_90_days() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        manager.stake(&env, user.clone(), 1000, 90).unwrap();

        let stakes = StakingBonusManager::get_user_stakes(&env, user);
        let stake = stakes.get(0).unwrap();

        assert_eq!(stake.bonus_bps, 2000); // 20%
        assert_eq!(stake.bonus_amount, 200); // 20% of 1000
    }

    #[test]
    fn test_stake_365_days() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        manager.stake(&env, user.clone(), 1000, 365).unwrap();

        let stakes = StakingBonusManager::get_user_stakes(&env, user);
        let stake = stakes.get(0).unwrap();

        assert_eq!(stake.bonus_bps, 5000); // 50%
        assert_eq!(stake.bonus_amount, 500); // 50% of 1000
    }

    #[test]
    fn test_stake_invalid_amount() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        let result = manager.stake(&env, user, 0, 30);
        assert!(result.is_err());
    }

    #[test]
    fn test_stake_invalid_duration() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        let result = manager.stake(&env, user, 1000, 45); // Invalid duration
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_stakes() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        let stake_id_1 = manager.stake(&env, user.clone(), 1000, 30).unwrap();
        let stake_id_2 = manager.stake(&env, user.clone(), 2000, 60).unwrap();
        let stake_id_3 = manager.stake(&env, user.clone(), 3000, 90).unwrap();

        assert_eq!(stake_id_1, 0);
        assert_eq!(stake_id_2, 1);
        assert_eq!(stake_id_3, 2);

        let stakes = StakingBonusManager::get_user_stakes(&env, user.clone());
        assert_eq!(stakes.len(), 3);

        // Verify totals
        let total = StakingBonusManager::get_user_total_staked(&env, user);
        assert_eq!(total, 6000);
    }

    // ────────────────────────────────────────────────────────────────────────
    // Time Lock Tests
    // ────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_bonus_not_claimable_before_holding_period() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        manager.stake(&env, user.clone(), 1000, 30).unwrap();

        // Try to claim immediately
        let result = manager.claim_bonuses(&env, user.clone());
        assert!(result.is_err());

        // Verify pending is 0
        let pending = StakingBonusManager::get_user_pending_bonuses(&env, user);
        assert_eq!(pending, 0);
    }

    #[test]
    fn test_bonus_claimable_after_holding_period() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        manager.stake(&env, user.clone(), 1000, 30).unwrap();

        // Advance time by 31 days
        env.ledger()
            .set_timestamp(env.ledger().timestamp() + (31 * 24 * 60 * 60));

        // Now should be claimable
        let result = manager.claim_bonuses(&env, user.clone());
        assert!(result.is_ok());

        let claimed_amount = result.unwrap();
        assert_eq!(claimed_amount, 50); // 5% of 1000
    }

    #[test]
    fn test_stake_not_claimable_before_unlock() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        manager.stake(&env, user.clone(), 1000, 30).unwrap();

        // Advance time by 29 days (before 30-day lock)
        env.ledger()
            .set_timestamp(env.ledger().timestamp() + (29 * 24 * 60 * 60));

        // Try to claim stake
        let result = manager.claim_stake(&env, user, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_stake_claimable_after_unlock() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        manager.stake(&env, user.clone(), 1000, 30).unwrap();

        // Advance time by 31 days
        env.ledger()
            .set_timestamp(env.ledger().timestamp() + (31 * 24 * 60 * 60));

        // Now should be claimable
        let result = manager.claim_stake(&env, user.clone(), 0);
        assert!(result.is_ok());

        let principal = result.unwrap();
        assert_eq!(principal, 1000);

        // Verify stake is inactive
        let stakes = StakingBonusManager::get_user_stakes(&env, user);
        let stake = stakes.get(0).unwrap();
        assert!(!stake.is_active);
    }

    // ────────────────────────────────────────────────────────────────────────
    // Bonus Calculation Tests
    // ────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_bonus_calculations() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        // 30 days: 5%
        manager.stake(&env, user.clone(), 1000, 30).unwrap();
        let stakes = StakingBonusManager::get_user_stakes(&env, user.clone());
        assert_eq!(stakes.get(0).unwrap().bonus_amount, 50);

        // 60 days: 12%
        manager.stake(&env, user.clone(), 1000, 60).unwrap();
        let stakes = StakingBonusManager::get_user_stakes(&env, user.clone());
        assert_eq!(stakes.get(1).unwrap().bonus_amount, 120);

        // 90 days: 20%
        manager.stake(&env, user.clone(), 1000, 90).unwrap();
        let stakes = StakingBonusManager::get_user_stakes(&env, user.clone());
        assert_eq!(stakes.get(2).unwrap().bonus_amount, 200);

        // 365 days: 50%
        manager.stake(&env, user.clone(), 1000, 365).unwrap();
        let stakes = StakingBonusManager::get_user_stakes(&env, user);
        assert_eq!(stakes.get(3).unwrap().bonus_amount, 500);
    }

    #[test]
    fn test_bonus_with_large_amounts() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        manager.stake(&env, user.clone(), 1_000_000, 90).unwrap();

        let stakes = StakingBonusManager::get_user_stakes(&env, user);
        let stake = stakes.get(0).unwrap();

        // 20% of 1,000,000 = 200,000
        assert_eq!(stake.bonus_amount, 200_000);
    }

    // ────────────────────────────────────────────────────────────────────────
    // Early Unstaking Tests (Penalty)
    // ────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_early_unstaking_penalty() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        manager.stake(&env, user.clone(), 1000, 90).unwrap();

        // Unstake immediately (before lock expires)
        let result = manager.unstake_early(&env, user.clone(), 0);
        assert!(result.is_ok());

        let (principal_returned, penalty) = result.unwrap();

        // 10% penalty
        assert_eq!(penalty, 100);
        assert_eq!(principal_returned, 900);

        // Verify stake is inactive
        let stakes = StakingBonusManager::get_user_stakes(&env, user);
        let stake = stakes.get(0).unwrap();
        assert!(!stake.is_active);
    }

    #[test]
    fn test_cannot_unstake_already_claimed() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        manager.stake(&env, user.clone(), 1000, 30).unwrap();

        // Advance time to make unlock possible
        env.ledger()
            .set_timestamp(env.ledger().timestamp() + (31 * 24 * 60 * 60));

        // Claim stake normally
        manager.claim_stake(&env, user.clone(), 0).unwrap();

        // Try to unstake early should fail
        let result = manager.unstake_early(&env, user, 0);
        assert!(result.is_err());
    }

    // ────────────────────────────────────────────────────────────────────────
    // Transparency and Query Tests
    // ────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_query_user_totals() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        manager.stake(&env, user.clone(), 1000, 30).unwrap();
        manager.stake(&env, user.clone(), 2000, 60).unwrap();
        manager.stake(&env, user.clone(), 3000, 90).unwrap();

        let total_staked = StakingBonusManager::get_user_total_staked(&env, user.clone());
        assert_eq!(total_staked, 6000);

        let earned_bonuses = StakingBonusManager::get_user_earned_bonuses(&env, user);
        assert_eq!(earned_bonuses, 50 + 120 + 200); // 370
    }

    #[test]
    fn test_query_pending_bonuses() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        manager.stake(&env, user.clone(), 1000, 30).unwrap();
        manager.stake(&env, user.clone(), 2000, 60).unwrap();

        // Pending before holding period
        let pending = StakingBonusManager::get_user_pending_bonuses(&env, user.clone());
        assert_eq!(pending, 0);

        // Advance time by 31 days
        env.ledger()
            .set_timestamp(env.ledger().timestamp() + (31 * 24 * 60 * 60));

        // Now both should be pending
        let pending = StakingBonusManager::get_user_pending_bonuses(&env, user);
        assert_eq!(pending, 50 + 120); // 170
    }

    #[test]
    fn test_query_stake_details() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        manager.stake(&env, user.clone(), 1000, 90).unwrap();

        let result = StakingBonusManager::get_stake_details(&env, user.clone(), 0);
        assert!(result.is_ok());

        let stake = result.unwrap();
        assert_eq!(stake.amount, 1000);
        assert_eq!(stake.bonus_bps, 2000);
        assert_eq!(stake.bonus_amount, 200);

        // Invalid stake ID
        let result = StakingBonusManager::get_stake_details(&env, user, 999);
        assert!(result.is_err());
    }

    #[test]
    fn test_query_statistics() {
        let env = Env::default();
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let manager = StakingBonusManager::new();

        manager.stake(&env, user1, 1000, 30).unwrap();
        manager.stake(&env, user2, 2000, 60).unwrap();

        let (total_staked, _, _) = StakingBonusManager::get_statistics(&env);
        assert_eq!(total_staked, 3000);
    }

    // ────────────────────────────────────────────────────────────────────────
    // Distribution Tests
    // ────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_distribution_requires_waiting_period() {
        let env = Env::default();
        let manager = StakingBonusManager::new();

        let result = manager.execute_distribution(&env);
        assert!(result.is_ok()); // First distribution always succeeds

        // Try immediately again
        let result = manager.execute_distribution(&env);
        assert!(result.is_err()); // Should fail - waiting period not elapsed
    }

    #[test]
    fn test_distribution_history_tracking() {
        let env = Env::default();
        let manager = StakingBonusManager::new();

        manager.execute_distribution(&env).unwrap();

        let history = StakingBonusManager::get_distribution_history(&env);
        assert_eq!(history.len(), 1);

        let record = history.get(0).unwrap();
        assert!(record.distributed_at > 0);
    }

    // ────────────────────────────────────────────────────────────────────────
    // Integration Tests
    // ────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_full_staking_lifecycle() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        // 1. User stakes
        manager.stake(&env, user.clone(), 1000, 90).unwrap();
        assert_eq!(
            StakingBonusManager::get_user_total_staked(&env, user.clone()),
            1000
        );

        // 2. Advance time to make bonus claimable (31 days)
        env.ledger()
            .set_timestamp(env.ledger().timestamp() + (31 * 24 * 60 * 60));

        // 3. Claim bonus
        let bonus = manager.claim_bonuses(&env, user.clone()).unwrap();
        assert_eq!(bonus, 200); // 20% of 1000

        let claimed = StakingBonusManager::get_user_claimed_bonuses(&env, user.clone());
        assert_eq!(claimed, 200);

        // 4. Verify stake is still locked beyond 31 days
        let result = manager.claim_stake(&env, user.clone(), 0);
        assert!(result.is_err()); // Not unlocked yet (30 days < 90 days)

        // 5. Advance to near complete lock period
        env.ledger()
            .set_timestamp(env.ledger().timestamp() + (60 * 24 * 60 * 60));

        // 6. Now claim stake principal
        let principal = manager.claim_stake(&env, user.clone(), 0).unwrap();
        assert_eq!(principal, 1000);

        // 7. Verify stake is inactive
        let stakes = StakingBonusManager::get_user_stakes(&env, user);
        assert!(!stakes.get(0).unwrap().is_active);
    }

    #[test]
    fn test_claimed_bonus_not_reusable() {
        let env = Env::default();
        let user = Address::generate(&env);
        let manager = StakingBonusManager::new();

        manager.stake(&env, user.clone(), 1000, 30).unwrap();

        // Advance time by 31 days
        env.ledger()
            .set_timestamp(env.ledger().timestamp() + (31 * 24 * 60 * 60));

        // Claim bonus
        manager.claim_bonuses(&env, user.clone()).unwrap();

        // Try to claim again
        let result = manager.claim_bonuses(&env, user);
        assert!(result.is_err()); // No more claimable bonuses
    }
}
